//! Doc taint consistency checker CLI.
//!
//! Pure calculation is in the doc_taint_consistency module; this is the shell.

use std::io::{self, Write};
use std::path::Path;

#[path = "../doc_taint_consistency/mod.rs"]
mod doc_taint_consistency;

enum DocError {
    WrongFile { path: String },
    NotFile { path: String },
    CannotRead { path: String, detail: String },
    InvalidUtf8 { path: String, detail: String },
}

fn main() {
    let mut args = std::env::args_os().skip(1);

    match (args.next(), args.next().is_some()) {
        (Some(path_arg), false) => {
            let path = Path::new(&path_arg);
            let result = read_doc(path);
            match result {
                Ok(text) => run_scan(&text),
                Err(doc_err) => exit_error(doc_err),
            }
        }
        _ => {
            write_usage();
            std::process::exit(2);
        }
    }
}

fn write_usage() {
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();
    let result = writeln!(
        stderr_lock,
        "usage: check-doc-taint-consistency velvet-ballistics-MASTER.md"
    );
    if result.is_err() {
        std::process::exit(2);
    }
}

fn exit_error(doc_err: DocError) {
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();
    let msg = error_to_string(&doc_err);
    let result = writeln!(stderr_lock, "{}", msg);
    if result.is_err() {
        std::process::exit(2);
    }
    std::process::exit(2);
}

fn error_to_string(err: &DocError) -> String {
    match err {
        DocError::WrongFile { path } => {
            format!("error: expected velvet-ballistics-MASTER.md, got {}", path)
        }
        DocError::NotFile { path } => {
            format!("error: document not found: {}", path)
        }
        DocError::CannotRead { path, detail } => {
            format!("error: cannot read {}: {}", path, detail)
        }
        DocError::InvalidUtf8 { path, detail } => {
            format!("error: invalid utf-8 in {}: {}", path, detail)
        }
    }
}

fn read_doc(path: &Path) -> Result<String, DocError> {
    let path_str = path.to_string_lossy().into_owned();

    if path
        .file_name()
        .is_none_or(|n| n != "velvet-ballistics-MASTER.md")
    {
        return Err(DocError::WrongFile { path: path_str });
    }

    if !path.is_file() {
        return Err(DocError::NotFile { path: path_str });
    }

    let bytes = std::fs::read(path).map_err(|e| DocError::CannotRead {
        path: path_str.clone(),
        detail: e.to_string(),
    })?;

    String::from_utf8(bytes).map_err(|e| DocError::InvalidUtf8 {
        path: path_str,
        detail: e.to_string(),
    })
}

fn run_scan(text: &str) {
    let findings = doc_taint_consistency::scan::scan(text);

    if findings.is_empty() {
        exit_pass();
    } else {
        exit_fail(&findings);
    }
}

fn exit_pass() {
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    let result = writeln!(stdout_lock, "doc taint consistency: PASS");
    if result.is_err() {
        std::process::exit(2);
    }
    std::process::exit(0);
}

fn exit_fail(findings: &[String]) {
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();
    let result = writeln!(stderr_lock, "doc taint consistency: FAIL");
    if result.is_err() {
        std::process::exit(2);
    }
    let write_result = findings
        .iter()
        .try_for_each(|finding| writeln!(stderr_lock, "- {}", finding));
    if write_result.is_err() {
        std::process::exit(2);
    }
    std::process::exit(1);
}
