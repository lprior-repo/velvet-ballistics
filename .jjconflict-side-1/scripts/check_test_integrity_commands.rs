use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Command helpers
// ---------------------------------------------------------------------------

fn tool_output_bytes(tool: &str, args: &[&str], cwd: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("{tool} {} failed to start: {error}", args.join(" ")))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{tool} {} failed: {stderr}", args.join(" ")))
    }
}

pub(crate) fn tool_output(tool: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let bytes = tool_output_bytes(tool, args, cwd)?;
    String::from_utf8(bytes).map_err(|error| {
        format!(
            "{tool} {} returned non-UTF8 stdout: {error}",
            args.join(" ")
        )
    })
}

fn tool_output_allow_fail_bytes(tool: &str, args: &[&str], cwd: &Path) -> Option<Vec<u8>> {
    Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| output.stdout)
}

fn tool_output_allow_fail(tool: &str, args: &[&str], cwd: &Path) -> Option<String> {
    tool_output_allow_fail_bytes(tool, args, cwd).and_then(|bytes| String::from_utf8(bytes).ok())
}

pub(crate) fn git_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    tool_output("git", args, cwd)
}

pub(crate) fn git_output_bytes(args: &[&str], cwd: &Path) -> Result<Vec<u8>, String> {
    tool_output_bytes("git", args, cwd)
}

pub(crate) fn git_allow_fail(args: &[&str], cwd: &Path) -> Option<String> {
    tool_output_allow_fail("git", args, cwd)
}

pub(crate) fn git_allow_fail_bytes(args: &[&str], cwd: &Path) -> Option<Vec<u8>> {
    tool_output_allow_fail_bytes("git", args, cwd)
}

pub(crate) fn jj_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    tool_output("jj", args, cwd)
}

pub(crate) fn jj_output_bytes(args: &[&str], cwd: &Path) -> Result<Vec<u8>, String> {
    tool_output_bytes("jj", args, cwd)
}

pub(crate) fn jj_allow_fail_bytes(args: &[&str], cwd: &Path) -> Option<Vec<u8>> {
    tool_output_allow_fail_bytes("jj", args, cwd)
}
