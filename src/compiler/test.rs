use super::source::SourceFile;
use crate::compiler::Compiler;
use crate::compiler::diagnostic::DiagnosticDetails;
use crate::compiler::render::{CliRenderer, RenderConfig};
use crate::lexer::token::Span;
use crate::parser::CstSpanDisplayMode;
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

    assert_eq!(
        source.path(),
        Some(PathBuf::from("/tmp/sample.my").as_path())
    );
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
    let path = std::env::temp_dir().join(format!(
        "source_file_test_{}_{}.my",
        std::process::id(),
        unique
    ));

    fs::write(&path, "line1\nline2").expect("should create temp file");
    let source = SourceFile::from_path(&path).expect("should load file");

    assert_eq!(source.path(), Some(path.as_path()));
    assert_eq!(source.text(), "line1\nline2");
    assert_eq!(source.line_col(6), Some((1, 0)));

    fs::remove_file(path).expect("should clean temp file");
}

#[test]
fn compiler_compile_returns_output_with_tokens_and_cst() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");

    let outcome = compiler.compile(source);
    assert!(!outcome.has_errors());
    let output = outcome.output.as_ref().expect("source should parse");

    assert!(!output.tokens().is_empty());
    assert!(!output.cst().nodes.is_empty());

    let cst_text = format!("{}", compiler.display_cst(output, &outcome.source));
    assert!(!cst_text.is_empty());
}

#[test]
fn compiler_compile_returns_error_for_unknown_char_in_function_body() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {@}");

    let outcome = compiler.compile(source);

    assert!(matches!(
        outcome
            .diagnostics
            .first()
            .map(|diagnostic| &diagnostic.details),
        Some(DiagnosticDetails::LexUnknownCharacter { ch: '@' })
    ));
    assert!(outcome.output.is_some());
    assert_eq!(outcome.diagnostics[0].labels[0].span.start, 11);
    assert_eq!(outcome.diagnostics[0].labels[0].span.end, 12);
}

#[test]
fn compiler_compile_continues_to_parse_after_lex_error() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() { @ let x:i32 = ; }");

    let outcome = compiler.compile(source);

    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.details,
            DiagnosticDetails::LexUnknownCharacter { ch: '@' }
        )
    }));
    assert!(outcome.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.details,
            DiagnosticDetails::ParseUnexpectedToken { .. }
        )
    }));
}

#[test]
fn compiler_compile_returns_error_for_unterminated_block_comment() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() { /* comment");

    let outcome = compiler.compile(source);

    assert!(matches!(
        outcome
            .diagnostics
            .first()
            .map(|diagnostic| &diagnostic.details),
        Some(DiagnosticDetails::LexUnterminatedBlockComment)
    ));
    assert_eq!(outcome.diagnostics[0].labels[0].span.start, 12);
    assert_eq!(
        outcome.diagnostics[0].labels[0].span.end,
        "fn main() { /* comment".len()
    );
}

#[test]
fn compiler_cst_display_can_switch_to_range_mode() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");

    let outcome = compiler.compile(source);
    assert!(!outcome.has_errors());
    let output = outcome.output.as_ref().expect("source should parse");

    let cst_text = format!(
        "{}",
        compiler.display_cst_with_mode(output, &outcome.source, CstSpanDisplayMode::Text)
    );
    let cst_range = format!(
        "{}",
        compiler.display_cst_with_mode(output, &outcome.source, CstSpanDisplayMode::Range)
    );

    assert!(cst_text.contains("\"fn main() {}\""));
    assert!(cst_range.contains("[0.."));
    assert!(!cst_range.contains("\"fn main() {}\""));
}

#[test]
fn cli_renderer_prints_source_snippet_for_diagnostic() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {@}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stderr.contains("error[E0001]"));
    assert!(rendered.stderr.contains("fn main() {@}"));
    assert!(rendered.stderr.contains("^ unknown character `@`"));
    assert!(
        rendered
            .stdout
            .contains("Compile finished with diagnostics")
    );
    assert!(!rendered.stderr.contains("Compile failed."));
}

#[test]
fn cli_renderer_prints_token_table_when_enabled() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_tokens(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("Tokens"));
    assert!(rendered.stdout.contains("Kind"));
    assert!(rendered.stdout.contains("Text"));
    assert!(rendered.stdout.contains("Line:Col"));
    assert!(rendered.stdout.contains("Keyword(Fn)"));
    assert!(rendered.stdout.contains("Ident"));
    assert!(rendered.stdout.contains("<eof>"));
    assert!(rendered.stdout.contains("1:1"));
}

#[test]
fn cli_renderer_prints_ast_when_enabled() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_ast(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("AST"));
    assert!(rendered.stdout.contains("Program"));
    assert!(rendered.stdout.contains("Fn main"));
}

#[test]
fn cli_renderer_prints_hir_when_enabled() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_hir(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("HIR"));
    assert!(rendered.stdout.contains("HIR Program"));
    assert!(rendered.stdout.contains("DefTable"));
    assert!(rendered.stdout.contains("LocalTable"));
}

#[test]
fn cli_renderer_prints_typecheck_when_enabled() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_typecheck(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("Typecheck"));
    assert!(rendered.stdout.contains("TypeckResults"));
    assert!(rendered.stdout.contains("DefTys"));
    assert!(rendered.stdout.contains("LocalTys"));
    assert!(rendered.stdout.contains("ExprTys"));
}

#[test]
fn compiler_compile_returns_thir_for_type_correct_program() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");

    let outcome = compiler.compile(source);
    assert!(!outcome.has_errors());
    let output = outcome.output.as_ref().expect("source should parse");

    assert!(output.thir().is_some());
}

#[test]
fn compiler_compile_skips_thir_when_typecheck_fails() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() -> i32 {}");

    let outcome = compiler.compile(source);
    let output = outcome.output.as_ref().expect("source should parse");

    assert!(outcome.has_errors());
    assert!(output.typeck().is_some());
    assert!(output.thir().is_none());
}

#[test]
fn compiler_compile_skips_thir_when_frontend_has_errors() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() { @ }");

    let outcome = compiler.compile(source);
    let output = outcome
        .output
        .as_ref()
        .expect("recovering parse should keep output");

    assert!(outcome.has_errors());
    assert!(output.thir().is_none());
}

#[test]
fn cli_renderer_prints_thir_when_enabled() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_thir(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("THIR"));
    assert!(rendered.stdout.contains("THIR Program"));
    assert!(rendered.stdout.contains("Bodies"));
}

#[test]
fn compiler_compile_returns_ir_for_type_correct_program() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {}");

    let outcome = compiler.compile(source);
    assert!(!outcome.has_errors());
    let output = outcome.output.as_ref().expect("source should parse");

    assert!(output.ir().is_some());
}

#[test]
fn compiler_compile_skips_ir_when_typecheck_fails() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() -> i32 {}");

    let outcome = compiler.compile(source);
    let output = outcome.output.as_ref().expect("source should parse");

    assert!(outcome.has_errors());
    assert!(output.typeck().is_some());
    assert!(output.thir().is_none());
    assert!(output.ir().is_none());
}

#[test]
fn compiler_compile_skips_ir_when_frontend_has_errors() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() { @ }");

    let outcome = compiler.compile(source);
    let output = outcome
        .output
        .as_ref()
        .expect("recovering parse should keep output");

    assert!(outcome.has_errors());
    assert!(output.ir().is_none());
}

#[test]
fn cli_renderer_prints_ir_when_enabled() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() { let mut x:i32 = 1; x = x + 1; }");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_ir(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("IR"));
    assert!(rendered.stdout.contains("; LLVM-like IR"));
    assert!(rendered.stdout.contains("define void @main"));
    assert!(rendered.stdout.contains("entry:"));
    assert!(rendered.stdout.contains("alloca i32"));
    assert!(rendered.stdout.contains("store i32"));
    assert!(rendered.stdout.contains("add i32"));
    assert!(rendered.stdout.contains("ret void"));
}

#[test]
fn cli_renderer_prints_ir_unavailable_when_typecheck_failed() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() -> i32 {}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_ir(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("IR"));
    assert!(rendered.stdout.contains("<not available>"));
}

#[test]
fn cli_renderer_prints_thir_unavailable_when_typecheck_failed() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() -> i32 {}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_show_thir(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stdout.contains("THIR"));
    assert!(rendered.stdout.contains("<not available>"));
}

#[test]
fn cli_renderer_can_color_diagnostics_when_enabled() {
    let compiler = Compiler::build(false).expect("compiler should build");
    let source = SourceFile::new("fn main() {@}");
    let outcome = compiler.compile(source);
    let renderer = CliRenderer::new(RenderConfig::new(false).with_color(true));

    let rendered = renderer.render_outcome(&compiler, &outcome);

    assert!(rendered.stderr.contains("\x1b[31;1merror\x1b[0m"));
    assert!(rendered.stderr.contains("\x1b[1m[E0001]\x1b[0m"));
    assert!(rendered.stderr.contains("\x1b[34;1m-->\x1b[0m"));
    assert!(rendered.stderr.contains("\x1b[31m"));
}
