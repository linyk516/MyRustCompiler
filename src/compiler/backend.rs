use crate::{compiler::output::CompileOutcome, ir::pretty::IrDump};
use std::{
    ffi::OsString,
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub enum LlEmitError {
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

#[derive(Debug)]
pub enum TargetEmitError {
    Ll(LlEmitError),
    BackendNotFound,
    BackendFailed {
        program: PathBuf,
        status: String,
        stderr: String,
    },
    BackendLaunchFailed {
        program: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for TargetEmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetEmitError::Ll(error) => write!(f, "{error}"),
            TargetEmitError::BackendNotFound => {
                f.write_str("could not find clang; install LLVM/clang or put clang on PATH")
            }
            TargetEmitError::BackendFailed {
                program,
                status,
                stderr,
            } => {
                if stderr.is_empty() {
                    write!(f, "{} failed with status {status}", program.display())
                } else {
                    write!(
                        f,
                        "{} failed with status {status}: {stderr}",
                        program.display()
                    )
                }
            }
            TargetEmitError::BackendLaunchFailed { program, source } => {
                write!(f, "failed to run {}: {source}", program.display())
            }
        }
    }
}

impl From<LlEmitError> for TargetEmitError {
    fn from(error: LlEmitError) -> Self {
        TargetEmitError::Ll(error)
    }
}

pub fn llvm_output_path(source_path: &Path) -> PathBuf {
    source_path.with_extension("ll")
}

pub fn target_temp_llvm_path(output_path: &Path) -> PathBuf {
    let output_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "my_rust_compiler_{output_name}_{}_{}.ll",
        std::process::id(),
        unique
    ))
}

pub fn emit_llvm_ir_file(
    outcome: &CompileOutcome,
    source_path: &Path,
) -> Result<PathBuf, LlEmitError> {
    let path = llvm_output_path(source_path);
    emit_llvm_ir_to_path(outcome, &path)?;
    Ok(path)
}

pub fn emit_llvm_ir_to_path(outcome: &CompileOutcome, path: &Path) -> Result<(), LlEmitError> {
    let ir = outcome
        .output
        .as_ref()
        .and_then(|output| output.ir())
        .ok_or(LlEmitError::IrUnavailable)?;
    let dump = IrDump::new(&ir.program).dump();
    fs::write(path, dump).map_err(|source| LlEmitError::WriteFailed {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

pub fn emit_target_file(
    outcome: &CompileOutcome,
    output_path: &Path,
) -> Result<PathBuf, TargetEmitError> {
    let ll_path = target_temp_llvm_path(output_path);
    emit_llvm_ir_to_path(outcome, &ll_path)?;

    let result = link_llvm_ir_with_clang(&ll_path, output_path);
    let _ = fs::remove_file(&ll_path);
    result?;

    Ok(output_path.to_path_buf())
}

pub fn clang_command_args(ll_path: &Path, output_path: &Path) -> Vec<OsString> {
    vec![
        ll_path.as_os_str().to_os_string(),
        OsString::from("-o"),
        output_path.as_os_str().to_os_string(),
    ]
}

pub fn clang_candidates() -> [PathBuf; 3] {
    [
        PathBuf::from("clang"),
        PathBuf::from("/opt/homebrew/bin/clang"),
        PathBuf::from("/opt/homebrew/opt/llvm/bin/clang"),
    ]
}

pub fn link_llvm_ir_with_clang(ll_path: &Path, output_path: &Path) -> Result<(), TargetEmitError> {
    let args = clang_command_args(ll_path, output_path);

    for program in clang_candidates() {
        match Command::new(&program).args(&args).output() {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                return Err(TargetEmitError::BackendFailed {
                    program,
                    status: output.status.to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(TargetEmitError::BackendLaunchFailed { program, source });
            }
        }
    }

    Err(TargetEmitError::BackendNotFound)
}
