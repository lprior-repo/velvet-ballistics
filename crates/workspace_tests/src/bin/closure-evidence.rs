//! vb-jpq7 closure evidence checker.
//! Validates that every closed child bead has verifiable evidence of passing or
//! a documented followup, then outputs a machine-readable result.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "../closure_evidence/mod.rs"]
mod closure_evidence;

use closure_evidence::cli;
use closure_evidence::{parse_issues, parse_manifest, run_self_test, validate_closed_children};

fn main() {
    match cli::parse() {
        Ok(args) => run_args(&args),
        Err(msg) => exit_error(2, msg),
    }
}

fn run_args(args: &cli::CliArgs) {
    if args.self_test {
        run_self_test_cmd();
        return;
    }

    let root = match env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            exit_error(2, format!("cannot get cwd: {}", e));
            return;
        }
    };

    let manifest_path = resolve_path(&root, &args.manifest);

    let issues = match load_issues(&args.parent, &args.bd_workdir) {
        Ok(i) => i,
        Err(e) => {
            exit_error(2, e);
            return;
        }
    };

    if issues.is_empty() {
        exit_error(2, "no issues found for parent".to_string());
        return;
    }

    let manifest_text = match fs::read_to_string(&manifest_path) {
        Ok(t) => t,
        Err(e) => {
            let closed_count = issues.iter().filter(|i| i.status == "closed").count();
            write_validation_failure(
                closed_count,
                &format!("cannot read manifest {}: {}", manifest_path.display(), e),
            );
            return;
        }
    };

    let (rows, row_errors) = parse_manifest(&manifest_text);
    let outcome = validate_closed_children(&root, &issues, &rows);

    let all_failures: Vec<String> = outcome.failures.into_iter().chain(row_errors).collect();

    if all_failures.is_empty() {
        write_pass_marker(outcome.closed_count);
    } else {
        write_failures(outcome.closed_count, &all_failures);
    }
}

fn run_self_test_cmd() {
    match run_self_test() {
        Ok(code) => exit_with_message(
            code,
            if code == 0 {
                "SELF_TEST_PASS"
            } else {
                "SELF_TEST_FAIL"
            },
        ),
        Err(e) => exit_error(2, format!("SELF_TEST_ERROR: {}", e)),
    }
}

fn load_issues(parent: &str, workdir: &str) -> Result<Vec<closure_evidence::Issue>, String> {
    let output = Command::new("bd")
        .args([
            "list", "--parent", parent, "--status", "all", "--json", "-n", "0",
        ])
        .current_dir(workdir)
        .output()
        .map_err(|e| format!("failed to spawn bd: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "bd list failed exit={} stderr={}",
            output.status.code().unwrap_or(-1),
            compact(&stderr)
        ));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("bd output not UTF-8: {}", e))?;

    parse_issues(&stdout).map_err(|e| format!("failed to parse children JSON: {}", e))
}

fn resolve_path(root: &Path, raw_path: &str) -> PathBuf {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn compact(text: &str) -> String {
    text.lines().next().unwrap_or("").to_string()
}

fn write_pass_marker(closed_count: usize) {
    let msg = format!(
        "VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children={}\n",
        closed_count
    );
    match write_stdout(&msg) {
        Ok(()) => exit_with_code(0),
        Err(e) => exit_error(1, format!("failed to write output: {}", e)),
    }
}

fn write_failures(closed_count: usize, failures: &[String]) {
    let mut msg = format!(
        "VB_JPQ7_CLOSURE_EVIDENCE_FAIL closed_children={}\n",
        closed_count
    );
    for f in failures {
        msg.push_str(&format!("- {}\n", f));
    }
    match write_stdout(&msg) {
        Ok(()) => exit_with_code(1),
        Err(e) => exit_error(1, format!("failed to write output: {}", e)),
    }
}

fn write_validation_failure(closed_count: usize, reason: &str) {
    write_failures(closed_count, &[reason.to_string()]);
}

fn write_stdout(msg: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(msg.as_bytes())?;
    Ok(())
}

fn write_stderr(msg: &str) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_all(msg.as_bytes())?;
    Ok(())
}

fn exit_error(code: i32, msg: String) {
    let line = format!("CHECK FAILED: {}\n", msg);
    match write_stderr(&line) {
        Ok(()) => exit_with_code(code),
        Err(_) => exit_with_code(code),
    }
}

fn exit_with_code(code: i32) {
    std::process::exit(code);
}

fn exit_with_message(code: i32, msg: &str) {
    let line = format!("{}\n", msg);
    match write_stdout(&line) {
        Ok(()) => exit_with_code(code),
        Err(e) => exit_error(code, format!("failed to write output: {}", e)),
    }
}
