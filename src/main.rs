use crate::compiler::Compiler;
use crate::compiler::output::CompileOutcome;
use crate::compiler::render::{CliRenderer, RenderConfig};
use crate::compiler::source::SourceFile;
use crate::ir::pretty::IrDump;
use clap::Parser;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

pub mod ast;
pub mod compiler;
pub mod hir;
pub mod ir;
pub mod lexer;
mod my_grammar;
pub mod parser;
pub mod thir;
pub mod typecheck;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    rebuild: bool,
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    #[arg(long, default_value_t = false)]
    show_tokens: bool,
    #[arg(long, default_value_t = false)]
    show_ast: bool,
    #[arg(long, default_value_t = false)]
    show_hir: bool,
    #[arg(long, visible_alias = "show-typeck", default_value_t = false)]
    show_typecheck: bool,
    #[arg(long, default_value_t = false)]
    show_thir: bool,
    #[arg(long, default_value_t = false)]
    show_ir: bool,
    #[arg(long = "ll", default_value_t = false)]
    emit_ll: bool,
    #[arg(long, default_value_t = false)]
    color: bool,
    #[arg(long, default_value_t = false)]
    no_color: bool,
    #[arg(num_args = 1..)]
    file_paths: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse_from(normalize_ll_alias(std::env::args_os()));
    let file_paths = &args.file_paths;
    if file_paths.len() == 0 {
        // 冗余检查
        println!("No file paths provided.");
        return;
    }
    if args.verbose {
        println!("Verbose mode enabled.");
    }
    let rebuild = args.rebuild;
    let verbose = args.verbose;

    // 当前默认只处理第一个文件
    let compiler = Compiler::build(rebuild).expect("failed to build compiler");

    let file = SourceFile::from_path(&file_paths[0]).expect("failed to read file");

    let outcome = compiler.compile(file.clone());

    let use_color = !args.no_color && (args.color || std::io::stderr().is_terminal());
    let renderer = CliRenderer::new(
        RenderConfig::new(verbose)
            .with_show_tokens(args.show_tokens)
            .with_show_ast(args.show_ast)
            .with_show_hir(args.show_hir)
            .with_show_typecheck(args.show_typecheck)
            .with_show_thir(args.show_thir)
            .with_show_ir(args.show_ir)
            .with_color(use_color),
    );
    let rendered = renderer.render_outcome(&compiler, &outcome);
    print!("{}", rendered.stdout);
    eprint!("{}", rendered.stderr);

    if args.emit_ll {
        match emit_llvm_ir_file(&outcome, &file_paths[0]) {
            Ok(path) => println!("LLVM IR written to {}", path.display()),
            Err(error) => eprintln!("failed to emit LLVM IR: {error}"),
        }
    }
}

fn normalize_ll_alias<I, T>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    args.into_iter()
        .map(|arg| {
            let arg = arg.into();
            if arg == OsString::from("-ll") {
                OsString::from("--ll")
            } else {
                arg
            }
        })
        .collect()
}

fn llvm_output_path(source_path: &Path) -> PathBuf {
    source_path.with_extension("ll")
}

#[derive(Debug)]
enum LlEmitError {
    IrUnavailable,
    WriteFailed {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for LlEmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlEmitError::IrUnavailable => {
                f.write_str("LLVM IR is not available because IR generation did not finish")
            }
            LlEmitError::WriteFailed { path, source } => {
                write!(f, "failed to write {}: {source}", path.display())
            }
        }
    }
}

fn emit_llvm_ir_file(outcome: &CompileOutcome, source_path: &Path) -> Result<PathBuf, LlEmitError> {
    let ir = outcome
        .output
        .as_ref()
        .and_then(|output| output.ir())
        .ok_or(LlEmitError::IrUnavailable)?;
    let path = llvm_output_path(source_path);
    let dump = IrDump::new(&ir.program).dump();
    fs::write(&path, dump).map_err(|source| LlEmitError::WriteFailed {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normalize_ll_alias_accepts_dash_ll() {
        let args = normalize_ll_alias([
            OsString::from("compiler"),
            OsString::from("-ll"),
            OsString::from("input.txt"),
        ]);

        assert_eq!(args[1], OsString::from("--ll"));
    }

    #[test]
    fn llvm_output_path_replaces_source_extension_with_ll() {
        let path = llvm_output_path(&PathBuf::from("example_source/source1.txt"));

        assert_eq!(path, PathBuf::from("example_source/source1.ll"));
    }

    #[test]
    fn emit_llvm_ir_file_writes_dump_when_ir_is_available() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let source_path =
            std::env::temp_dir().join(format!("my_rust_compiler_ll_emit_{unique}.txt"));
        let ll_path = llvm_output_path(&source_path);

        fs::write(&source_path, "fn main() {}").expect("should create temporary source");
        let compiler = Compiler::build(false).expect("compiler should build");
        let source = SourceFile::from_path(&source_path).expect("source should load");
        let outcome = compiler.compile(source);

        let written = emit_llvm_ir_file(&outcome, &source_path).expect("should emit llvm ir");
        let text = fs::read_to_string(&written).expect("should read emitted llvm ir");

        assert_eq!(written, ll_path);
        assert!(text.contains("; LLVM-like IR"));
        assert!(text.contains("define void @main"));

        let _ = fs::remove_file(source_path);
        let _ = fs::remove_file(written);
    }
}
