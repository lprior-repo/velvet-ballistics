use super::check_test_integrity_commands::{
    git_allow_fail_bytes, git_output, git_output_bytes, jj_allow_fail_bytes, jj_output,
    jj_output_bytes,
};
use super::{ChangedEntry, RenamePair, Vcs};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Changed-file enumeration
// ---------------------------------------------------------------------------

pub(crate) fn changed_files(
    root: &Path,
    base: &str,
    vcs: Vcs,
) -> Result<Vec<ChangedEntry>, String> {
    match vcs {
        Vcs::Git => {
            let bytes = git_output_bytes(
                &["diff", "--name-status", "-z", "--no-renames", base, "--"],
                root,
            )?;
            let mut entries = Vec::new();
            let mut segments = bytes.split(|&byte| byte == 0);
            while let Some(status_seg) = segments.next() {
                if status_seg.is_empty() {
                    continue;
                }
                let status = match status_seg.first() {
                    Some(&byte) => byte,
                    None => continue,
                };
                let path_seg = segments.next().ok_or_else(|| {
                    "malformed git name-status output: dangling status byte".to_owned()
                })?;
                if path_seg.is_empty() {
                    continue;
                }
                let path = String::from_utf8(path_seg.to_vec())
                    .map_err(|_| "changed path is not UTF-8".to_owned())?;
                entries.push(ChangedEntry {
                    status,
                    old_path: path.clone(),
                    new_path: path,
                });
            }
            Ok(entries)
        }
        Vcs::Jj => {
            let text = jj_output(&["diff", "--types", "--from", base, "--to", "@"], root)?;
            let mut entries = Vec::new();
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let type_code = trimmed.get(..2).ok_or_else(|| {
                    format!("malformed jj --types line (shorter than 2 bytes): {line:?}")
                })?;
                let path = trimmed
                    .get(2..)
                    .map(str::trim_start)
                    .ok_or_else(|| format!("malformed jj --types line (path offset): {line:?}"))?;
                if path.is_empty() {
                    return Err(format!("malformed jj --types line (empty path): {line:?}"));
                }
                let status = match type_code {
                    "-F" => b'A',
                    "F-" => b'D',
                    "FF" => b'M',
                    other => {
                        return Err(format!(
                            "unsupported jj --types type code {other:?} for path {path:?} (fail-closed)"
                        ));
                    }
                };
                entries.push(ChangedEntry {
                    status,
                    old_path: path.to_owned(),
                    new_path: path.to_owned(),
                });
            }
            Ok(entries)
        }
    }
}

// ---------------------------------------------------------------------------
// Rename-pair enumeration (sound old/new mapping via --find-renames)
// ---------------------------------------------------------------------------

fn git_rename_pairs(root: &Path, base: &str) -> Result<Vec<RenamePair>, String> {
    let bytes = git_output_bytes(
        &["diff", "--name-status", "-z", "--find-renames", base, "--"],
        root,
    )?;
    let mut segments = bytes.split(|&byte| byte == 0);
    let mut pairs = Vec::new();
    while let Some(status_seg) = segments.next() {
        if status_seg.is_empty() {
            continue;
        }
        let status = match status_seg.first() {
            Some(&byte) => byte,
            None => continue,
        };
        match status {
            b'R' => {
                let old_seg = segments.next().ok_or_else(|| {
                    "malformed git rename output: missing old path".to_owned()
                })?;
                let new_seg = segments.next().ok_or_else(|| {
                    "malformed git rename output: missing new path".to_owned()
                })?;
                if old_seg.is_empty() || new_seg.is_empty() {
                    return Err("malformed git rename output: empty path".to_owned());
                }
                let old_path = String::from_utf8(old_seg.to_vec())
                    .map_err(|_| "rename old path is not UTF-8".to_owned())?;
                let new_path = String::from_utf8(new_seg.to_vec())
                    .map_err(|_| "rename new path is not UTF-8".to_owned())?;
                pairs.push(RenamePair {
                    old_path,
                    new_path,
                });
            }
            b'A' | b'M' | b'D' | b'T' | b'U' | b'X' | b'B' => {
                let _ = segments.next().ok_or_else(|| {
                    "malformed git name-status output: missing path".to_owned()
                })?;
            }
            other => {
                return Err(format!(
                    "unsupported git name-status status byte 0x{other:02x} (fail-closed)"
                ));
            }
        }
    }
    // Ambiguity guard: a rename source/destination must be unique.
    let mut seen_old = BTreeSet::new();
    let mut seen_new = BTreeSet::new();
    for pair in &pairs {
        if !seen_old.insert(pair.old_path.clone()) {
            return Err(format!(
                "ambiguous rename: old path {} appears in multiple rename pairs",
                pair.old_path
            ));
        }
        if !seen_new.insert(pair.new_path.clone()) {
            return Err(format!(
                "ambiguous rename: new path {} appears in multiple rename pairs",
                pair.new_path
            ));
        }
    }
    Ok(pairs)
}

fn jj_rename_pairs(root: &Path, base: &str) -> Result<Vec<RenamePair>, String> {
    let text = jj_output(&["diff", "--types", "--from", base, "--to", "@"], root)?;
    let mut pairs = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains('{') || trimmed.contains('}') || trimmed.contains("=>") {
            // jj renames surface as `{old => new}` (optionally after a type code).
            let Some((old, new)) = trimmed.rsplit_once("=>") else {
                return Err(format!(
                    "ambiguous jj --types rename syntax: {trimmed:?} (fail-closed)"
                ));
            };
            let old = old.trim();
            let new = new.trim();
            let old_path = old
                .strip_prefix('{')
                .map(str::trim)
                .or_else(|| old.split_whitespace().last())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    format!("malformed jj --types rename old path: {trimmed:?}")
                })?;
            let new_path = new
                .strip_suffix('}')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    format!("malformed jj --types rename new path: {trimmed:?}")
                })?;
            pairs.push(RenamePair {
                old_path: old_path.to_owned(),
                new_path: new_path.to_owned(),
            });
        }
    }
    Ok(pairs)
}

pub(crate) fn rename_pairs(root: &Path, base: &str, vcs: Vcs) -> Result<Vec<RenamePair>, String> {
    match vcs {
        Vcs::Git => git_rename_pairs(root, base),
        Vcs::Jj => jj_rename_pairs(root, base),
    }
}

// ---------------------------------------------------------------------------
// File content access (raw bytes) and blob resolution
// ---------------------------------------------------------------------------

pub(crate) fn base_file_bytes(root: &Path, base: &str, path: &str, vcs: Vcs) -> Option<Vec<u8>> {
    match vcs {
        Vcs::Git => git_allow_fail_bytes(&["show", &format!("{base}:{path}")], root),
        Vcs::Jj => jj_allow_fail_bytes(&["file", "show", "-r", base, path], root),
    }
}

pub(crate) fn current_file_bytes(root: &Path, path: &str) -> Option<Vec<u8>> {
    fs::read(root.join(path)).ok()
}

pub(crate) fn resolve_base_blob(
    root: &Path,
    base: &str,
    path: &str,
) -> Result<Option<String>, String> {
    let spec = format!("{base}:{path}");
    match git_output(&["rev-parse", "--verify", &spec], root) {
        Ok(text) => {
            let hash = text.trim().to_owned();
            if hash.is_empty() {
                Ok(None)
            } else {
                Ok(Some(hash))
            }
        }
        Err(_) => Ok(None),
    }
}

pub(crate) fn resolve_current_blob(root: &Path, path: &str) -> Result<Option<String>, String> {
    let full = root.join(path);
    if !full.is_file() {
        return Ok(None);
    }
    let full_str = full
        .to_str()
        .ok_or_else(|| format!("non-UTF8 path {}", full.display()))?;
    let text = git_output(&["hash-object", full_str], root)?;
    let hash = text.trim().to_owned();
    if hash.is_empty() {
        return Err("git hash-object returned empty output".to_owned());
    }
    Ok(Some(hash))
}

pub(crate) fn is_hex40(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// ---------------------------------------------------------------------------
// Diff scanning (per-path, strict UTF-8)
// ---------------------------------------------------------------------------

pub(crate) fn path_diff_bytes(
    root: &Path,
    base: &str,
    path: &str,
    vcs: Vcs,
) -> Result<Vec<u8>, String> {
    match vcs {
        Vcs::Git => git_output_bytes(
            &[
                "diff",
                "--no-renames",
                "--no-textconv",
                "--no-ext-diff",
                "--text",
                "--unified=0",
                base,
                "--",
                path,
            ],
            root,
        ),
        Vcs::Jj => jj_output_bytes(
            &["diff", "--git", "--from", base, "--to", "@", "--", path],
            root,
        ),
    }
}
