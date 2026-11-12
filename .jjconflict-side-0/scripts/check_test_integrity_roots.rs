use super::check_test_integrity_commands::{git_allow_fail, git_output, jj_output};
use super::{RootInfo, Vcs};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Root / base resolution
// ---------------------------------------------------------------------------

pub(crate) fn root_dir() -> Result<RootInfo, String> {
    if let Ok(text) = git_output(&["rev-parse", "--show-toplevel"], Path::new(".")) {
        return Ok(RootInfo {
            path: PathBuf::from(text.trim()),
            vcs: Vcs::Git,
        });
    }
    jj_output(&["workspace", "root"], Path::new(".")).map(|text| RootInfo {
        path: PathBuf::from(text.trim()),
        vcs: Vcs::Jj,
    })
}

pub(crate) fn default_base(root: &Path, vcs: Vcs) -> String {
    if let Ok(value) = std::env::var("TEST_INTEGRITY_BASE") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if vcs == Vcs::Jj {
        return "@-".to_owned();
    }
    let dirty = git_output(&["status", "--porcelain"], root)
        .map(|text| !text.trim().is_empty())
        .unwrap_or(true);
    if dirty {
        return "HEAD".to_owned();
    }
    git_allow_fail(&["merge-base", "origin/main", "HEAD"], root)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "HEAD".to_owned())
}

pub(crate) fn validate_base_revision(root: &Path, base: &str, vcs: Vcs) -> Result<(), String> {
    if base.trim().is_empty() {
        return Err("empty base revision".to_owned());
    }
    match vcs {
        Vcs::Git => {
            let commit = format!("{base}^{{commit}}");
            git_output(&["rev-parse", "--verify", &commit], root).map(|_| ())
        }
        Vcs::Jj => {
            jj_output(&["log", "--no-graph", "-r", base, "-T", "commit_id"], root).map(|_| ())
        }
    }
    .map_err(|error| format!("invalid base revision {base:?}: {error}"))
}

pub(crate) fn resolve_base_hash(root: &Path, base: &str, vcs: Vcs) -> Result<String, String> {
    validate_base_revision(root, base, vcs)?;
    match vcs {
        Vcs::Git => {
            let commit = format!("{base}^{{commit}}");
            let hash = git_output(&["rev-parse", "--verify", &commit], root)?
                .trim()
                .to_owned();
            if hash.is_empty() {
                return Err(format!("could not resolve base {base:?} to a commit hash"));
            }
            Ok(hash)
        }
        Vcs::Jj => {
            let hash = jj_output(&["log", "--no-graph", "-r", base, "-T", "commit_id"], root)?
                .trim()
                .to_owned();
            if hash.is_empty() {
                return Err(format!("could not resolve base {base:?} to a commit hash"));
            }
            Ok(hash)
        }
    }
}
