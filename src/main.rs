use crate::compiler::Compiler;
use crate::compiler::render::{CliRenderer, RenderConfig};
use crate::compiler::source::SourceFile;
use clap::Parser;
use std::io::IsTerminal;
use std::path::PathBuf;

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
    #[arg(long, default_value_t = false)]
    color: bool,
    #[arg(long, default_value_t = false)]
    no_color: bool,
    #[arg(num_args = 1..)]
    file_paths: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
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
}
