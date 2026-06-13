use crate::compiler::Compiler;
use crate::compiler::backend::emit_target_file;
use crate::compiler::output::CompileOutcome;
use crate::compiler::render::{CliRenderer, RenderConfig};
use crate::compiler::source::SourceFile;
use crate::hir::pretty::HirDump;
use crate::ir::pretty::IrDump;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::CstSpanDisplayMode;
use crate::thir::pretty::ThirDump;
use crate::typecheck::pretty::TypeckDump;
use serde::{Deserialize, Serialize};
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const NOT_AVAILABLE: &str = "<not available>";

/// GUI 编译请求。
///
/// GUI 使用内存中的源码进行编译，`file_name` 只用于诊断显示和临时文件命名，
/// 不要求真实存在于磁盘。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileRequest {
    pub source: String,
    pub file_name: Option<String>,
    pub verbose: bool,
}

/// GUI 编译响应。
///
/// 所有阶段输出都以文本形式暴露，避免前端绑定 Rust 内部节点结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResponse {
    pub success: bool,
    pub summary: String,
    pub diagnostics: String,
    pub tokens: String,
    pub cst: String,
    pub ast: String,
    pub hir: String,
    pub typecheck: String,
    pub thir: String,
    pub ir: String,
}

/// GUI 构建并运行请求。
///
/// `stdin` 会被写入生成的可执行文件，便于演示 `scanf` 一类 C 接口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRunRequest {
    pub source: String,
    pub file_name: Option<String>,
    pub stdin: String,
}

/// GUI 构建并运行响应。
///
/// 编译失败时 `compile` 中仍保留前端各阶段输出，运行相关字段保持空值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRunResponse {
    pub compile: CompileResponse,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub backend_error: Option<String>,
}

/// 编译内存源码并返回 GUI 可直接展示的阶段文本。
pub fn compile_source(request: CompileRequest) -> CompileResponse {
    match compile_for_gui(&request) {
        Ok((_compiler, _outcome, response)) => response,
        Err(message) => init_error_response(message),
    }
}

/// 编译内存源码，调用 LLVM/clang 后端生成可执行文件，并运行该文件。
pub fn build_and_run(request: BuildRunRequest) -> BuildRunResponse {
    let compile_request = CompileRequest {
        source: request.source,
        file_name: request.file_name,
        verbose: false,
    };

    let (_compiler, outcome, compile) = match compile_for_gui(&compile_request) {
        Ok(result) => result,
        Err(message) => {
            return BuildRunResponse {
                compile: init_error_response(message),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                backend_error: None,
            };
        }
    };

    if !compile.success {
        return BuildRunResponse {
            compile,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            backend_error: None,
        };
    }

    let output_path = temp_executable_path(compile_request.file_name.as_deref());
    if let Err(error) = emit_target_file(&outcome, &output_path) {
        return BuildRunResponse {
            compile,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            backend_error: Some(error.to_string()),
        };
    }

    let run_result = run_executable(&output_path, &request.stdin);
    let _ = std::fs::remove_file(&output_path);

    match run_result {
        Ok(output) => BuildRunResponse {
            compile,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
            backend_error: None,
        },
        Err(error) => BuildRunResponse {
            compile,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            backend_error: Some(error.to_string()),
        },
    }
}

fn compile_for_gui(
    request: &CompileRequest,
) -> Result<(Compiler, CompileOutcome, CompileResponse), String> {
    let compiler = Compiler::build(false).map_err(|error| format!("{error:?}"))?;
    let source =
        SourceFile::with_path(gui_file_name(request.file_name.as_deref()), &request.source);
    let outcome = compiler.compile(source);
    let response = response_from_outcome(&compiler, &outcome, request.verbose);
    Ok((compiler, outcome, response))
}

fn response_from_outcome(
    compiler: &Compiler,
    outcome: &CompileOutcome,
    verbose: bool,
) -> CompileResponse {
    let rendered = CliRenderer::new(RenderConfig::new(verbose)).render_outcome(compiler, outcome);

    let mut response = CompileResponse {
        success: !outcome.has_errors(),
        summary: rendered.stdout,
        diagnostics: rendered.stderr,
        tokens: NOT_AVAILABLE.to_string(),
        cst: NOT_AVAILABLE.to_string(),
        ast: NOT_AVAILABLE.to_string(),
        hir: NOT_AVAILABLE.to_string(),
        typecheck: NOT_AVAILABLE.to_string(),
        thir: NOT_AVAILABLE.to_string(),
        ir: NOT_AVAILABLE.to_string(),
    };

    let Some(output) = &outcome.output else {
        return response;
    };

    response.tokens = dump_tokens(output.tokens(), &outcome.source);
    response.cst = format!(
        "{}",
        compiler.display_cst_with_mode(output, &outcome.source, CstSpanDisplayMode::Range)
    );

    if let Some(ast) = output.ast() {
        response.ast = ast.to_string();
    }

    if let Some(hir) = output.hir() {
        let dump = HirDump::new(&hir.hir, &hir.defs, &hir.locals);
        response.hir = format!("{}{}{}", dump, dump.dum_def(), dump.dum_local());
    }

    if let Some(typeck) = output.typeck() {
        response.typecheck = TypeckDump::new(&typeck.results, &typeck.tys).dump();
    }

    if let (Some(thir), Some(typeck)) = (output.thir(), output.typeck()) {
        response.thir = ThirDump::new(&thir.program, &typeck.tys).dump();
    }

    if let Some(ir) = output.ir() {
        response.ir = IrDump::new(&ir.program).dump();
    }

    response
}

fn dump_tokens(tokens: &[Token], source: &SourceFile) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<6}{:<24}{:<24}{:<14}{}",
        "#", "Kind", "Text", "Span", "Line:Col"
    );

    for (index, token) in tokens.iter().enumerate() {
        let _ = writeln!(
            out,
            "{:<6}{:<24}{:<24}{:<14}{}",
            index,
            truncate_text(&format!("{:?}", token.kind), 23),
            truncate_text(&token_text(token, source), 23),
            format!("{}..{}", token.span.start, token.span.end),
            token_position(token, source)
        );
    }

    out
}

fn token_text(token: &Token, source: &SourceFile) -> String {
    if matches!(&token.kind, TokenKind::Eof) {
        return "<eof>".to_string();
    }

    match token.span.text(source.text()) {
        Some(text) => text
            .chars()
            .flat_map(|ch| ch.escape_default())
            .collect::<String>(),
        None => "<invalid span>".to_string(),
    }
}

fn token_position(token: &Token, source: &SourceFile) -> String {
    source
        .line_utf8_col(token.span.start)
        .map(|(line, column)| format!("{}:{}", line + 1, column + 1))
        .unwrap_or_else(|| "-".to_string())
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }

    let keep_chars = max_chars.saturating_sub(3);
    let mut truncated = text.chars().take(keep_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn init_error_response(message: String) -> CompileResponse {
    CompileResponse {
        success: false,
        summary: String::new(),
        diagnostics: format!("failed to initialize compiler: {message}"),
        tokens: NOT_AVAILABLE.to_string(),
        cst: NOT_AVAILABLE.to_string(),
        ast: NOT_AVAILABLE.to_string(),
        hir: NOT_AVAILABLE.to_string(),
        typecheck: NOT_AVAILABLE.to_string(),
        thir: NOT_AVAILABLE.to_string(),
        ir: NOT_AVAILABLE.to_string(),
    }
}

fn run_executable(path: &PathBuf, stdin_text: &str) -> std::io::Result<std::process::Output> {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_text.as_bytes())?;
    }

    child.wait_with_output()
}

fn temp_executable_path(file_name: Option<&str>) -> PathBuf {
    let stem = file_name
        .and_then(|name| {
            std::path::Path::new(name)
                .file_stem()
                .and_then(|stem| stem.to_str())
        })
        .unwrap_or("gui_program");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "my_rust_compiler_gui_{stem}_{}_{}",
        std::process::id(),
        unique
    ))
}

fn gui_file_name(file_name: Option<&str>) -> PathBuf {
    PathBuf::from(file_name.unwrap_or("gui_input.txt"))
}
