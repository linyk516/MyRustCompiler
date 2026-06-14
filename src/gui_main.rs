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
#[cfg(debug_assertions)]
use std::{
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
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
    let _frontend_dev_server = ensure_frontend_dev_server();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![compile_source, build_and_run])
        .run(tauri::generate_context!())
        .expect("failed to run MyRustCompiler GUI");
}

#[cfg(not(debug_assertions))]
fn ensure_frontend_dev_server() {}

#[cfg(debug_assertions)]
fn ensure_frontend_dev_server() -> Option<FrontendDevServer> {
    if frontend_is_ready() {
        return None;
    }

    let gui_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("gui");
    let mut child = Command::new("npm")
        .arg("run")
        .arg("dev")
        .current_dir(gui_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to start GUI frontend dev server with `npm run dev`");

    for _ in 0..120 {
        if frontend_is_ready() {
            return Some(FrontendDevServer { child: Some(child) });
        }

        if let Ok(Some(status)) = child.try_wait() {
            panic!("GUI frontend dev server exited before becoming ready: {status}");
        }

        thread::sleep(Duration::from_millis(250));
    }

    let _ = child.kill();
    let _ = child.wait();
    panic!("timed out waiting for GUI frontend dev server at http://127.0.0.1:1420");
}

#[cfg(debug_assertions)]
fn frontend_is_ready() -> bool {
    let addr: SocketAddr = "127.0.0.1:1420"
        .parse()
        .expect("valid GUI frontend dev server address");

    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

#[cfg(debug_assertions)]
struct FrontendDevServer {
    child: Option<Child>,
}

#[cfg(debug_assertions)]
impl Drop for FrontendDevServer {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
