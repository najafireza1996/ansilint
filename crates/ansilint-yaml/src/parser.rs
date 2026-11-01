//! Event-driven construction of the span-carrying AST.

use ansilint_core::Span;
use saphyr_parser::{Event, Parser, ScalarStyle as YamlStyle, Span as YamlSpan, StrInput};

use crate::ast::{
    Alias, Document, Mapping, MappingEntry, Node, Scalar, ScalarStyle, Sequence, SourceFile,
};
use crate::comments::scan_comments;
use crate::error::YamlError;

/// Maximum nesting of sequences and mappings. Deeper input is rejected with a
/// spanned [`YamlError`] (reported as `parser-error`) instead of recursing
/// further, so pathological files can never overflow the stack (H-B3). This
/// is a safety limit; `max_block_depth` is the separate lint setting.
pub const MAX_NESTING_DEPTH: usize = 128;

/// Parses every YAML document in `source`.
///
/// # Errors
///
/// Returns [`YamlError`] with the scanner's location when the YAML is invalid.
pub fn parse(source: &SourceFile) -> Result<Vec<Document>, YamlError> {
    let text = source.as_str();
    let comments = scan_comments(text, &source.line_index);
    let mut parser = Parser::new_from_str(text);
    let mut documents = Vec::new();
    let mut starts = Vec::new();
    while let Some(next) = parser.next_event() {
        let (event, span) = next.map_err(|error| YamlError::from_scan(&error))?;
        match event {
            Event::StreamStart => {}
            Event::StreamEnd => break,
            Event::DocumentStart(explicit) => {
                starts.push(to_span(span).start);
                documents.push(parse_document(text, &mut parser, explicit, span)?);
            }
            other => {
                return Err(YamlError::unexpected(to_span(span), &format!("{other:?}")));
            }
        }
    }
    assign_comments(&mut documents, &starts, comments);
    Ok(documents)
}

/// Moves each comment into the document whose start precedes it; comments
/// before the first document (a header) belong to the first one. A `#` that
/// starts inside a quoted or block scalar is content, not a comment (H-D1).
fn assign_comments(documents: &mut [Document], starts: &[u32], comments: Vec<crate::Comment>) {
    let content = content_spans(documents);
    for comment in comments {
        let position = comment.span.start;
        let inside = content
            .partition_point(|span| span.start <= position)
            .checked_sub(1)
            .and_then(|index| content.get(index))
            .is_some_and(|span| position < span.end);
        if inside {
            continue;
        }
        let owner = starts
            .partition_point(|start| *start <= comment.span.start)
            .saturating_sub(1);
        if let Some(document) = documents.get_mut(owner) {
            document.comments.push(comment);
        }
    }
}

/// Spans of quoted and block scalars, sorted. Plain scalars cannot contain
/// ` #`, so only these can hide a `#` that the line scanner mistakes for a
/// comment. Scalars never nest, so the spans do not overlap.
fn content_spans(documents: &[Document]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut stack: Vec<&Node> = documents.iter().map(|document| &document.root).collect();
    while let Some(node) = stack.pop() {
        match node {
            Node::Scalar(scalar) if scalar.style != ScalarStyle::Plain => spans.push(scalar.span),
            Node::Sequence(sequence) => stack.extend(&sequence.items),
            Node::Mapping(mapping) => stack.extend(
                mapping
                    .entries
                    .iter()
                    .flat_map(|entry| [&entry.key, &entry.value]),
            ),
            _ => {}
        }
    }
    spans.sort_unstable();
    spans
}

fn parse_document<'input>(
    text: &'input str,
    parser: &mut Parser<'input, StrInput<'input>>,
    explicit_start: bool,
    start_span: YamlSpan,
) -> Result<Document, YamlError> {
    let (event, span) = next_event(parser)?;
    if matches!(event, Event::DocumentEnd) {
        return Ok(Document {
            root: Node::Null {
                span: to_span(span),
            },
            comments: Vec::new(),
            doc_start: explicit_start.then(|| to_span(start_span)),
            doc_end: explicit_end(text, span),
            explicit_start,
        });
    }
    let root = parse_node(text, event, span, parser, 0)?;
    let (event, end_span) = next_event(parser)?;
    if !matches!(event, Event::DocumentEnd) {
        return Err(YamlError::unexpected(
            to_span(end_span),
            &format!("{event:?}"),
        ));
    }
    Ok(Document {
        root,
        comments: Vec::new(),
        doc_start: explicit_start.then(|| to_span(start_span)),
        doc_end: explicit_end(text, end_span),
        explicit_start,
    })
}

fn next_event<'input>(
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(Event<'input>, YamlSpan), YamlError> {
    let Some(next) = parser.next_event() else {
        return Err(YamlError::eof());
    };
    next.map_err(|error| YamlError::from_scan(&error))
}

fn parse_node<'input>(
    text: &'input str,
    event: Event<'input>,
    span: YamlSpan,
    parser: &mut Parser<'input, StrInput<'input>>,
    depth: usize,
) -> Result<Node, YamlError> {
    if depth >= MAX_NESTING_DEPTH {
        return Err(YamlError {
            span: to_span(span),
            message: format!("nesting depth exceeds the supported limit ({MAX_NESTING_DEPTH})"),
        });
    }
    match event {
        Event::Scalar(value, style, anchor, _tag) => {
            Ok(scalar_node(text, value.as_ref(), style, anchor, span))
        }
        Event::Alias(anchor) => Ok(Node::Alias(Alias {
            anchor,
            span: to_span(span),
        })),
        Event::SequenceStart(anchor, _tag) => parse_sequence(text, span, anchor, parser, depth),
        Event::MappingStart(anchor, _tag) => parse_mapping(text, span, anchor, parser, depth),
        other => Err(YamlError::unexpected(to_span(span), &format!("{other:?}"))),
    }
}

fn parse_sequence<'input>(
    text: &'input str,
    start_span: YamlSpan,
    anchor: usize,
    parser: &mut Parser<'input, StrInput<'input>>,
    depth: usize,
) -> Result<Node, YamlError> {
    let start = to_span(start_span).start;
    let mut items = Vec::new();
    loop {
        let (event, span) = next_event(parser)?;
        if matches!(event, Event::SequenceEnd) {
            let end = to_span(span).end;
            return Ok(Node::Sequence(Sequence {
                items,
                span: Span::new(start, end),
                anchor: anchor_id(anchor),
            }));
        }
        items.push(parse_node(
            text,
            event,
            span,
            parser,
            depth.saturating_add(1),
        )?);
    }
}

fn parse_mapping<'input>(
    text: &'input str,
    start_span: YamlSpan,
    anchor: usize,
    parser: &mut Parser<'input, StrInput<'input>>,
    depth: usize,
) -> Result<Node, YamlError> {
    let start = to_span(start_span).start;
    let mut entries = Vec::new();
    loop {
        let (event, key_span) = next_event(parser)?;
        if matches!(event, Event::MappingEnd) {
            let end = to_span(key_span).end;
            return Ok(Node::Mapping(Mapping {
                entries,
                span: Span::new(start, end),
                anchor: anchor_id(anchor),
            }));
        }
        let key = parse_node(text, event, key_span, parser, depth.saturating_add(1))?;
        let (event, value_span) = next_event(parser)?;
        let value = parse_node(text, event, value_span, parser, depth.saturating_add(1))?;
        entries.push(MappingEntry {
            key_span: key.span(),
            value_span: value.span(),
            key,
            value,
        });
    }
}

fn scalar_node(text: &str, value: &str, style: YamlStyle, anchor: usize, span: YamlSpan) -> Node {
    let span = scalar_span(text, to_span(span), style);
    let style = map_style(style);
    if style == ScalarStyle::Plain && is_null(value) {
        return Node::Null { span };
    }
    Node::Scalar(Scalar {
        value: value.to_string(),
        style,
        span,
        anchor: anchor_id(anchor),
    })
}

fn scalar_span(text: &str, span: Span, style: YamlStyle) -> Span {
    match style {
        YamlStyle::SingleQuoted | YamlStyle::DoubleQuoted => {
            quoted_span(text, span).unwrap_or_else(|| trim_span(text, span))
        }
        YamlStyle::Plain | YamlStyle::Literal | YamlStyle::Folded => trim_span(text, span),
    }
}

fn trim_span(text: &str, span: Span) -> Span {
    let Some(raw) = span.text(text) else {
        return span;
    };
    let leading = raw.len().saturating_sub(raw.trim_start().len());
    let trailing = raw.len().saturating_sub(raw.trim_end().len());
    let start = span.start.saturating_add(as_u32(leading));
    let end = span.end.saturating_sub(as_u32(trailing));
    Span::new(start, end.max(start))
}

fn quoted_span(text: &str, span: Span) -> Option<Span> {
    let raw = span.text(text)?;
    let bytes = raw.as_bytes();
    let quote = *bytes.first()?;
    let close = match quote {
        b'"' => double_quote_end(bytes)?,
        b'\'' => single_quote_end(bytes)?,
        _ => return None,
    };
    let length = u32::try_from(close.saturating_add(1)).ok()?;
    Some(Span::new(span.start, span.start.saturating_add(length)))
}

fn double_quote_end(bytes: &[u8]) -> Option<usize> {
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate().skip(1) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            return Some(index);
        }
    }
    None
}

fn single_quote_end(bytes: &[u8]) -> Option<usize> {
    let mut index = 1;
    while index < bytes.len() {
        if bytes.get(index) != Some(&b'\'') {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'\'') {
            index += 2;
            continue;
        }
        return Some(index);
    }
    None
}

fn explicit_end(text: &str, span: YamlSpan) -> Option<Span> {
    let span = to_span(span);
    let raw = span.text(text)?;
    raw.starts_with("...").then_some(span)
}

const fn anchor_id(anchor: usize) -> Option<usize> {
    if anchor == 0 {
        None
    } else {
        Some(anchor)
    }
}

fn is_null(value: &str) -> bool {
    matches!(value, "" | "~" | "null" | "Null" | "NULL")
}

const fn map_style(style: YamlStyle) -> ScalarStyle {
    match style {
        YamlStyle::Plain => ScalarStyle::Plain,
        YamlStyle::SingleQuoted => ScalarStyle::SingleQuoted,
        YamlStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
        YamlStyle::Literal => ScalarStyle::Literal,
        YamlStyle::Folded => ScalarStyle::Folded,
    }
}

fn to_span(span: YamlSpan) -> Span {
    Span::new(as_u32(span.start.index()), as_u32(span.end.index()))
}

fn as_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
