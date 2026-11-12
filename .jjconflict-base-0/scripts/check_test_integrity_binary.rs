use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Real binary resolution (self-test temp commits bypass the guard wrapper)
// ---------------------------------------------------------------------------

fn is_elf(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return false;
    }
    magic == [0x7f, b'E', b'L', b'F']
}

pub(crate) fn resolve_real_binary(name: &str) -> Option<PathBuf> {
    let mut fallback: Option<PathBuf> = None;
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if !candidate.is_file() {
                continue;
            }
            let canonical = fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
            if fallback.is_none() {
                fallback = Some(canonical.clone());
            }
            if is_elf(&canonical) {
                return Some(canonical);
            }
        }
    }
    for dir in ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"] {
        let candidate = Path::new(dir).join(name);
        if is_elf(&candidate) {
            return Some(candidate);
        }
    }
    fallback
}

pub(crate) fn git_cmd(git: &Path, cwd: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(git)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("git {} failed to start: {error}", args.join(" ")))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git {} failed: {stderr}", args.join(" ")))
    }
}
