use super::source::SourceFile;
use crate::compiler::Compiler;
use crate::lexer::token::Span;
use std::path::PathBuf;

#[test]
fn source_file_new_populates_basic_fields() {
    let source = SourceFile::new("let x = 1;");

    assert!(source.path().is_none());
    assert_eq!(source.text(), "let x = 1;");
    assert_eq!(source.len_bytes(), 10);
    assert!(!source.is_empty());
}

#[test]
fn source_file_new_handles_empty_text() {
    let source = SourceFile::new("");

    assert!(source.is_empty());
    assert_eq!(source.len_bytes(), 0);
    assert_eq!(source.line_col(0), Some((0, 0)));
    assert_eq!(source.line_text(0), Some(""));
    assert_eq!(source.line_text(1), None);
}

#[test]
fn source_file_with_path_keeps_path_and_text() {
    let source = SourceFile::with_path("/tmp/sample.my", "abc");

    assert_eq!(source.path(), Some(PathBuf::from("/tmp/sample.my").as_path()));
    assert_eq!(source.text(), "abc");
}

#[test]
fn source_file_slice_returns_substring_for_valid_span() {
    let source = SourceFile::new("hello world");

    let span = Span { start: 6, end: 11 };
    assert_eq!(source.slice(span), Some("world"));
}

#[test]
fn source_file_slice_returns_none_for_out_of_bounds_span() {
    let source = SourceFile::new("hello");

    let span = Span { start: 0, end: 6 };
    assert_eq!(source.slice(span), None);
}

#[test]
fn source_file_line_col_maps_positions_across_lines() {
    let source = SourceFile::new("ab\n中d\n");

    assert_eq!(source.line_col(0), Some((0, 0)));
    assert_eq!(source.line_col(2), Some((0, 2)));
    assert_eq!(source.line_col(3), Some((1, 0)));
    assert_eq!(source.line_col(6), Some((1, 3)));
    assert_eq!(source.line_col(8), Some((2, 0)));
    assert_eq!(source.line_col(9), None);
}

#[test]
fn source_file_line_text_returns_each_line_including_newline() {
    let source = SourceFile::new("ab\n中d\n");

    assert_eq!(source.line_text(0), Some("ab\n"));
    assert_eq!(source.line_text(1), Some("中d\n"));
    assert_eq!(source.line_text(2), Some(""));
    assert_eq!(source.line_text(3), None);
}

#[test]
fn source_file_from_path_reads_file_content() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("source_file_test_{}_{}.my", std::process::id(), unique));

    fs::write(&path, "line1\nline2").expect("should create temp file");
    let source = SourceFile::from_path(&path).expect("should load file");

    assert_eq!(source.path(), Some(path.as_path()));
    assert_eq!(source.text(), "line1\nline2");
    assert_eq!(source.line_col(6), Some((1, 0)));

    fs::remove_file(path).expect("should clean temp file");
}

#[test]
fn compiler_compile_returns_output_with_tokens_and_cst() {
    let compiler = Compiler::build().expect("compiler should build");
    let source = SourceFile::new("fn main() {}");

    let output = compiler.compile(source).expect("source should parse");

    assert!(!output.tokens().is_empty());
    assert!(!output.cst().nodes.is_empty());

    let cst_text = format!("{}", compiler.display_cst(&output));
    assert!(!cst_text.is_empty());
}
