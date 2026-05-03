//! External tool integration: rustfmt and rustc for generated source validation.

use crate::{CodegenError, CodegenResult};
use std::process::Command;

/// Run rustfmt on generated source and return the formatted output.
pub fn format_generated_rust(source: &str) -> CodegenResult<String> {
    let mut child = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CodegenError::RustfmtFailed {
            detail: e.to_string(),
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| CodegenError::RustfmtFailed {
                detail: e.to_string(),
            })?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| CodegenError::RustfmtFailed {
            detail: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(CodegenError::RustfmtFailed {
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    String::from_utf8(output.stdout).map_err(|e| CodegenError::RustfmtFailed {
        detail: e.to_string(),
    })
}

/// Verify that generated Rust source compiles under the pinned nightly toolchain.
pub fn compile_check_generated_rust(
    source: &str,
    temp_dir: &std::path::Path,
) -> CodegenResult<()> {
    let file_path = temp_dir.join("generated_workflow.rs");
    std::fs::write(&file_path, source)?;

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("-o")
        .arg(temp_dir.join("generated_workflow.rlib"))
        .arg(&file_path)
        .output()
        .map_err(|e| CodegenError::CompileCheckFailed {
            detail: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(CodegenError::CompileCheckFailed {
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(())
}
