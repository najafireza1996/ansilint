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
/// before the first document (a header) belong to the first one.
fn assign_comments(documents: &mut [Document], starts: &[u32], comments: Vec<crate::Comment>) {
    for comment in comments {
        let owner = starts
            .partition_point(|start| *start <= comment.span.start)
            .saturating_sub(1);
        if let Some(document) = documents.get_mut(owner) {
            document.comments.push(comment);
        }
    }
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
mod tests {
    #![allow(
        clippy::assert_is_empty,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "tests use fixed fixtures and panic on broken assertions"
    )]

    use super::*;
    use crate::SourceFile;

    fn documents(text: &str) -> Vec<Document> {
        let source = SourceFile::new("test.yml", text);
        parse(&source).expect("valid YAML")
    }

    #[test]
    fn maps_keys_and_values_to_exact_spans() {
        let docs = documents("a: b # c\n");
        let mapping = docs[0].root.as_mapping().expect("mapping");
        let entry = &mapping.entries[0];
        assert_eq!(entry.key_span, Span::new(0, 1));
        assert_eq!(entry.value_span, Span::new(3, 4));
        assert_eq!(entry.value.as_str(), Some("b"));
    }

    #[test]
    fn quoted_scalar_spans_exclude_trailing_comments() {
        let docs = documents("a: \"b # c\" # d\n");
        let value = &docs[0].root.as_mapping().expect("mapping").entries[0].value;
        assert_eq!(value.as_str(), Some("b # c"));
        assert_eq!(value.span(), Span::new(3, 10));
    }

    #[test]
    fn duplicate_keys_are_preserved() {
        let docs = documents("a: 1\na: 2\n");
        let mapping = docs[0].root.as_mapping().expect("mapping");
        assert_eq!(mapping.entries.len(), 2);
        assert_eq!(mapping.get("a").and_then(Node::as_str), Some("1"));
        assert_eq!(mapping.get_all("a").count(), 2);
    }

    #[test]
    fn parses_multiple_documents() {
        let docs = documents("---\na: 1\n---\nb: 2\n");
        assert_eq!(docs.len(), 2);
        assert!(docs[0].explicit_start);
        assert!(docs[1].explicit_start);
        assert!(docs[0].doc_start.is_some());
        assert!(docs[1].doc_start.is_some());
    }

    fn nesting_error(text: String) -> YamlError {
        parse(&SourceFile::new("deep.yml", text)).expect_err("deep nesting is rejected")
    }

    #[test]
    fn accepts_nesting_up_to_the_documented_limit() {
        let depth = crate::MAX_NESTING_DEPTH - 1;
        assert!(parse(&SourceFile::new(
            "ok.yml",
            format!("{}x\n", "- ".repeat(depth))
        ))
        .is_ok());
        let error = nesting_error(format!("{}x\n", "- ".repeat(depth + 2)));
        assert!(error.message.contains("nesting depth"));
    }

    #[test]
    fn rejects_hundred_thousand_deep_input_with_a_spanned_error() {
        for text in [
            format!("{}x\n", "- ".repeat(100_000)),
            format!("{}{}\n", "[".repeat(100_000), "]".repeat(100_000)),
            format!("{}1{}\n", "{a: ".repeat(100_000), "}".repeat(100_000)),
        ] {
            // Flow collections hit saphyr's own guard first; both are errors.
            let error = nesting_error(text);
            assert!(
                error.message.contains("nesting depth")
                    || error.message.contains("recursion limit"),
                "{}",
                error.message
            );
            assert!(error.span.start > 0);
        }
    }

    #[test]
    fn rejects_excessive_nesting_without_overflowing() {
        let text = format!("{}x\n", "- ".repeat(300));
        let source = SourceFile::new("deep.yml", text);
        let error = parse(&source).expect_err("deep nesting is rejected");
        assert!(error.message.contains("nesting depth"));
    }

    #[test]
    fn records_comments_with_lines() {
        let docs = documents("a: 1 # note\n# full\nb: \"x # y\"\n");
        let comments = &docs[0].comments;
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].line, 0);
        assert_eq!(comments[0].content(), "note");
        assert_eq!(comments[1].line, 1);
        assert_eq!(comments[1].content(), "full");
    }

    fn contents(document: &Document) -> Vec<&str> {
        document
            .comments
            .iter()
            .map(crate::Comment::content)
            .collect()
    }

    #[test]
    fn each_document_owns_only_its_own_comments() {
        let docs = documents(
            "# header\n---\na: 1 # one\n...\n--- # start two\nb: 2\n# trailing\n---\n- 3 # three\n",
        );
        assert_eq!(docs.len(), 3);
        assert_eq!(contents(&docs[0]), ["header", "one"]);
        assert_eq!(contents(&docs[1]), ["start two", "trailing"]);
        assert_eq!(contents(&docs[2]), ["three"]);
    }

    #[test]
    fn preserves_anchor_ids_and_aliases() {
        let docs = documents("a: &x 1\nb: *x\n");
        let mapping = docs[0].root.as_mapping().expect("mapping");
        let anchored = &mapping.entries[0].value;
        let Node::Scalar(scalar) = anchored else {
            panic!("expected a scalar");
        };
        assert!(scalar.anchor.is_some());
        let alias = mapping.entries[1].value.as_alias().expect("alias");
        assert_eq!(Some(alias.anchor), scalar.anchor);
    }

    #[test]
    fn empty_input_has_no_documents() {
        assert!(documents("").is_empty());
        assert!(documents("# only a comment\n").is_empty());
    }

    #[test]
    fn explicit_empty_document_is_null() {
        let docs = documents("---\n");
        assert_eq!(docs.len(), 1);
        assert!(docs[0].root.is_null());
        assert!(docs[0].explicit_start);
    }

    #[test]
    fn reports_the_error_location() {
        let source = SourceFile::new("bad.yml", "a: [1,\n");
        let error = parse(&source).expect_err("invalid YAML");
        assert!(error.span.start > 0);
        assert!(!error.message.is_empty());
    }
}
