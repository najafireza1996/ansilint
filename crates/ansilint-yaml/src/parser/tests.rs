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
            error.message.contains("nesting depth") || error.message.contains("recursion limit"),
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

#[test]
fn hashes_inside_block_and_multiline_quoted_scalars_are_not_comments() {
    let docs = documents(
        "a: |\n  # not a comment\n  text\nb: >\n  # folded, not a comment\nc: \"x\n  # inside quotes\"\nd: 1 # real\n",
    );
    assert_eq!(contents(&docs[0]), ["real"]);
}

#[test]
fn comments_in_a_stream_without_documents_are_kept() {
    // A comment-only file has no YAML documents; its comments (e.g. `# noqa`)
    // live on an implicit null document instead of being dropped.
    let docs = documents("# noqa: warning\n# second\n");
    assert_eq!(docs.len(), 1);
    assert!(docs[0].root.is_null());
    assert_eq!(contents(&docs[0]), ["noqa: warning", "second"]);
    assert!(documents("").is_empty());
}
