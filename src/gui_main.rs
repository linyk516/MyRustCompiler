#![cfg(feature = "gui")]

pub mod ast;
pub mod borrowck;
pub mod compiler;
pub mod hir;
pub mod ir;
pub mod lexer;
mod my_grammar;
pub mod parser;
pub mod thir;
pub mod typecheck;

use crate::compiler::gui_api::{
    BuildRunRequest, BuildRunResponse, CompileRequest, CompileResponse,
};

/// Tauri command：编译内存源码并返回各阶段文本。
#[tauri::command]
fn compile_source(request: CompileRequest) -> CompileResponse {
    crate::compiler::gui_api::compile_source(request)
}

/// Tauri command：编译、链接、运行内存源码，并把 stdin 传给生成的程序。
#[tauri::command]
fn build_and_run(request: BuildRunRequest) -> BuildRunResponse {
    crate::compiler::gui_api::build_and_run(request)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![compile_source, build_and_run])
        .run(tauri::generate_context!())
        .expect("failed to run MyRustCompiler GUI");
}
