use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{File, canonicalize};
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, CWD, FileType, Mode, OFlags, fstat, linkat, openat, statat, unlinkat};
use serde_json::Value;

const STDOUT_TARGET: &str = "stdout";
const FILE_SCHEME: &str = "file";
const WEBHOOK_SCHEME: &str = "webhook";
// Linux path strings are effectively capped at 4095 bytes because the final
// NUL terminator consumes the last PATH_MAX byte.
const MAX_PATH_BYTES: usize = 4095;
const MAX_TEMP_STAGE_ATTEMPTS: usize = 8;
const MINIMUM_STAGE_BASE_NAME: &str = ".t";

enum TempStageCreation {
    Created((OsString, File)),
    Exhausted,
    NameTooLong,
}

pub(crate) struct DeliverFileTarget {
    parent_dir: OwnedFd,
    file_name: OsString,
    delivery_path: PathBuf,
}

impl DeliverFileTarget {
    fn delivery_path(&self) -> &Path {
        &self.delivery_path
    }
}

pub(crate) enum DeliverTarget {
    Stdout,
    NewFile(DeliverFileTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliverSinkError {
    MissingScheme,
    MissingFilePath,
    UnknownScheme,
    UnsupportedWebhook,
    RelativePath,
    MissingParent,
    ParentChanged,
    Directory,
    BlockedPath,
    StagingUnavailable,
    ExistingFile,
    OverlongPath,
    Io(io::ErrorKind),
}

impl fmt::Display for DeliverSinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingScheme => {
                f.write_str("deliver target must be stdout, file:<path>, or webhook:<url>")
            }
            Self::MissingFilePath => f.write_str("deliver file target is missing a path"),
            Self::UnknownScheme => f.write_str("deliver target scheme is unknown"),
            Self::UnsupportedWebhook => f.write_str("deliver webhook target is not supported yet"),
            Self::RelativePath => f.write_str("deliver file path must be absolute"),
            Self::MissingParent => f.write_str("deliver file parent directory is missing"),
            Self::ParentChanged => f.write_str("deliver file parent changed during validation"),
            Self::Directory => f.write_str("deliver file target is a directory"),
            Self::BlockedPath => f.write_str("deliver file path uses a blocked system root"),
            Self::StagingUnavailable => {
                f.write_str("deliver temporary staging path is unavailable")
            }
            Self::ExistingFile => f.write_str("deliver file target already exists"),
            Self::OverlongPath => f.write_str("deliver file path is too long"),
            Self::Io(kind) => write!(f, "deliver I/O failed: {kind:?}"),
        }
    }
}

pub(crate) fn parse_deliver_target(raw: &str) -> Result<DeliverTarget, DeliverSinkError> {
    if raw == STDOUT_TARGET {
        return Ok(DeliverTarget::Stdout);
    }

    let Some((scheme, value)) = raw.split_once(':') else {
        return Err(DeliverSinkError::MissingScheme);
    };
    match scheme {
        FILE_SCHEME => parse_file_target(value),
        WEBHOOK_SCHEME => Err(DeliverSinkError::UnsupportedWebhook),
        _ => Err(DeliverSinkError::UnknownScheme),
    }
}

pub(crate) fn write_json_line(
    target: &DeliverTarget,
    value: &Value,
) -> Result<(), DeliverSinkError> {
    match target {
        DeliverTarget::Stdout => write_json_line_to_writer(io::stdout().lock(), value),
        DeliverTarget::NewFile(target) => write_json_line_to_new_file(target, value),
    }
}

fn parse_file_target(value: &str) -> Result<DeliverTarget, DeliverSinkError> {
    if value.is_empty() {
        return Err(DeliverSinkError::MissingFilePath);
    }
    let requested_path = PathBuf::from(value);
    let delivery_target = resolve_new_file_target(&requested_path)?;
    Ok(DeliverTarget::NewFile(delivery_target))
}

fn validate_requested_file_path(path: &Path) -> Result<(), DeliverSinkError> {
    if path.as_os_str().as_encoded_bytes().len() > MAX_PATH_BYTES {
        return Err(DeliverSinkError::OverlongPath);
    }
    if !path.is_absolute() {
        return Err(DeliverSinkError::RelativePath);
    }
    Ok(())
}

fn resolve_new_file_target(path: &Path) -> Result<DeliverFileTarget, DeliverSinkError> {
    validate_requested_file_path(path)?;
    if path.is_dir() {
        return Err(DeliverSinkError::Directory);
    }

    let Some(parent) = path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };
    let parent_dir = open_parent_directory(parent)?;
    let resolved_parent = canonicalize_parent_path(parent)?;
    if is_blocked_root(&resolved_parent) {
        return Err(DeliverSinkError::BlockedPath);
    }
    ensure_parent_matches_path(&parent_dir, &resolved_parent)?;

    let Some(file_name) = path.file_name() else {
        return Err(DeliverSinkError::MissingFilePath);
    };
    let resolved_path = resolved_parent.join(file_name);
    validate_resolved_file_path(&parent_dir, &resolved_parent, file_name, &resolved_path)?;

    Ok(DeliverFileTarget {
        parent_dir,
        file_name: file_name.to_os_string(),
        delivery_path: resolved_path,
    })
}

fn validate_resolved_file_path(
    parent_dir: &OwnedFd,
    resolved_parent: &Path,
    file_name: &OsStr,
    resolved_path: &Path,
) -> Result<(), DeliverSinkError> {
    if resolved_path.as_os_str().as_encoded_bytes().len() > MAX_PATH_BYTES {
        return Err(DeliverSinkError::OverlongPath);
    }

    let minimum_stage_retry_name = temp_stage_name(
        OsStr::new(MINIMUM_STAGE_BASE_NAME),
        MAX_TEMP_STAGE_ATTEMPTS - 1,
    );
    if path_with_name_len(resolved_parent, &minimum_stage_retry_name)? > MAX_PATH_BYTES {
        return Err(DeliverSinkError::OverlongPath);
    }

    validate_target_absent(parent_dir, file_name)?;

    Ok(())
}

fn is_blocked_root(path: &Path) -> bool {
    path.starts_with("/dev") || path.starts_with("/proc") || path.starts_with("/sys")
}

fn write_json_line_to_new_file(
    target: &DeliverFileTarget,
    value: &Value,
) -> Result<(), DeliverSinkError> {
    let (temp_name, temp_file) = create_temp_stage_file(target)?;
    write_json_line_to_temp_file(target, &temp_name, temp_file, value)?;
    persist_temp_file(target, &temp_name)
}

fn write_json_line_to_temp_file(
    target: &DeliverFileTarget,
    temp_name: &OsStr,
    mut file: File,
    value: &Value,
) -> Result<(), DeliverSinkError> {
    let write_result = write_json_line_to_writer(&mut file, value)
        .and_then(|()| file.sync_all().map_err(to_io_error));
    match write_result {
        Ok(()) => Ok(()),
        Err(write_error) => match unlink_at(&target.parent_dir, temp_name) {
            Ok(()) => Err(write_error),
            Err(cleanup_error) => Err(cleanup_error),
        },
    }
}

fn persist_temp_file(
    target: &DeliverFileTarget,
    temp_name: &OsStr,
) -> Result<(), DeliverSinkError> {
    match linkat(
        &target.parent_dir,
        temp_name,
        &target.parent_dir,
        &target.file_name,
        AtFlags::empty(),
    ) {
        Ok(()) => {
            cleanup_temp_file_best_effort(&target.parent_dir, temp_name);
            Ok(())
        }
        Err(error) => {
            let delivery_error = if error == rustix::io::Errno::EXIST {
                DeliverSinkError::ExistingFile
            } else {
                to_rustix_io_error(error)
            };
            match unlink_at(&target.parent_dir, temp_name) {
                Ok(()) => Err(delivery_error),
                Err(cleanup_error) => Err(cleanup_error),
            }
        }
    }
}

fn cleanup_temp_file_best_effort(parent_dir: &OwnedFd, path: &OsStr) {
    match unlinkat(parent_dir, path, AtFlags::empty()) {
        Ok(()) | Err(_) => {}
    }
}

fn create_temp_stage_file(
    target: &DeliverFileTarget,
) -> Result<(OsString, File), DeliverSinkError> {
    let Some(parent) = target.delivery_path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };

    let mut exhausted_candidates = false;

    let preferred = preferred_temp_name(&target.file_name);
    match create_temp_stage_file_from_base_name(&target.parent_dir, parent, &preferred)? {
        TempStageCreation::Created(stage_file) => return Ok(stage_file),
        TempStageCreation::Exhausted => exhausted_candidates = true,
        TempStageCreation::NameTooLong => {}
    }

    let hashed = hashed_temp_name(target.delivery_path());
    match create_temp_stage_file_from_base_name(&target.parent_dir, parent, &hashed)? {
        TempStageCreation::Created(stage_file) => return Ok(stage_file),
        TempStageCreation::Exhausted => exhausted_candidates = true,
        TempStageCreation::NameTooLong => {}
    }

    let fallback = OsString::from(".tmp");
    match create_temp_stage_file_from_base_name(&target.parent_dir, parent, &fallback)? {
        TempStageCreation::Created(stage_file) => return Ok(stage_file),
        TempStageCreation::Exhausted => exhausted_candidates = true,
        TempStageCreation::NameTooLong => {}
    }

    let minimal = OsString::from(MINIMUM_STAGE_BASE_NAME);
    match create_temp_stage_file_from_base_name(&target.parent_dir, parent, &minimal)? {
        TempStageCreation::Created(stage_file) => return Ok(stage_file),
        TempStageCreation::Exhausted => exhausted_candidates = true,
        TempStageCreation::NameTooLong => {}
    }

    if exhausted_candidates {
        Err(DeliverSinkError::StagingUnavailable)
    } else {
        Err(DeliverSinkError::OverlongPath)
    }
}

fn preferred_temp_name(file_name: &OsStr) -> OsString {
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp");
    temp_name
}

fn hashed_temp_name(path: &Path) -> OsString {
    let digest = blake3::hash(path.as_os_str().as_encoded_bytes());
    let mut temp_name = String::from(".vb");
    for byte in &digest.as_bytes()[..8] {
        temp_name.push_str(&format!("{byte:02x}"));
    }
    OsString::from(temp_name)
}

fn create_temp_stage_file_from_base_name(
    parent_dir: &OwnedFd,
    parent: &Path,
    base_name: &OsStr,
) -> Result<TempStageCreation, DeliverSinkError> {
    for attempt in 0..MAX_TEMP_STAGE_ATTEMPTS {
        let candidate_name = temp_stage_name(base_name, attempt);
        if path_with_name_len(parent, &candidate_name)? > MAX_PATH_BYTES {
            return Ok(TempStageCreation::NameTooLong);
        }

        match create_new_file_at(parent_dir, &candidate_name) {
            Ok(file) => return Ok(TempStageCreation::Created((candidate_name, file))),
            Err(DeliverSinkError::ExistingFile) => {}
            Err(error) => return Err(error),
        }
    }

    Ok(TempStageCreation::Exhausted)
}

fn temp_stage_name(base_name: &OsStr, attempt: usize) -> OsString {
    if attempt == 0 {
        return base_name.to_os_string();
    }

    let mut candidate_name = base_name.to_os_string();
    candidate_name.push(".");
    candidate_name.push(attempt.to_string());
    candidate_name
}

fn path_with_name_len(parent: &Path, file_name: &OsStr) -> Result<usize, DeliverSinkError> {
    let separator = if parent == Path::new("/") {
        0_usize
    } else {
        1_usize
    };
    parent
        .as_os_str()
        .as_encoded_bytes()
        .len()
        .checked_add(separator)
        .and_then(|value| value.checked_add(file_name.as_encoded_bytes().len()))
        .ok_or(DeliverSinkError::OverlongPath)
}

fn create_new_file_at(parent_dir: &OwnedFd, path: &OsStr) -> Result<File, DeliverSinkError> {
    openat(
        parent_dir,
        path,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
        created_file_mode(),
    )
    .map(File::from)
    .map_err(|error| {
        if error == rustix::io::Errno::EXIST {
            DeliverSinkError::ExistingFile
        } else {
            to_rustix_io_error(error)
        }
    })
}

fn created_file_mode() -> Mode {
    Mode::RUSR | Mode::WUSR | Mode::RGRP | Mode::WGRP | Mode::ROTH | Mode::WOTH
}

fn open_parent_directory(parent: &Path) -> Result<OwnedFd, DeliverSinkError> {
    openat(
        CWD,
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| match error {
        rustix::io::Errno::NOENT | rustix::io::Errno::NOTDIR => DeliverSinkError::MissingParent,
        other => to_rustix_io_error(other),
    })
}

fn canonicalize_parent_path(parent: &Path) -> Result<PathBuf, DeliverSinkError> {
    canonicalize(parent).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => DeliverSinkError::ParentChanged,
        kind => DeliverSinkError::Io(kind),
    })
}

fn ensure_parent_matches_path(
    parent_dir: &OwnedFd,
    resolved_parent: &Path,
) -> Result<(), DeliverSinkError> {
    let parent_stat = fstat(parent_dir).map_err(to_rustix_io_error)?;
    let resolved_stat = statat(CWD, resolved_parent, AtFlags::empty()).map_err(|error| {
        if error == rustix::io::Errno::NOENT {
            DeliverSinkError::ParentChanged
        } else {
            to_rustix_io_error(error)
        }
    })?;
    if parent_stat.st_dev == resolved_stat.st_dev && parent_stat.st_ino == resolved_stat.st_ino {
        Ok(())
    } else {
        Err(DeliverSinkError::ParentChanged)
    }
}

fn validate_target_absent(parent_dir: &OwnedFd, file_name: &OsStr) -> Result<(), DeliverSinkError> {
    match statat(parent_dir, file_name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => {
            let file_type = FileType::from_raw_mode(stat.st_mode);
            if file_type.is_dir() {
                Err(DeliverSinkError::Directory)
            } else {
                Err(DeliverSinkError::ExistingFile)
            }
        }
        Err(error) => {
            if error == rustix::io::Errno::NOENT {
                Ok(())
            } else {
                Err(to_rustix_io_error(error))
            }
        }
    }
}

fn unlink_at(parent_dir: &OwnedFd, path: &OsStr) -> Result<(), DeliverSinkError> {
    unlinkat(parent_dir, path, AtFlags::empty()).map_err(to_rustix_io_error)
}

fn write_json_line_to_writer<W: Write>(
    mut writer: W,
    value: &Value,
) -> Result<(), DeliverSinkError> {
    serde_json::to_writer(&mut writer, value).map_err(|error| {
        error.io_error_kind().map_or(
            DeliverSinkError::Io(io::ErrorKind::InvalidData),
            DeliverSinkError::Io,
        )
    })?;
    writer.write_all(b"\n").map_err(to_io_error)?;
    writer.flush().map_err(to_io_error)
}

fn to_io_error(error: io::Error) -> DeliverSinkError {
    DeliverSinkError::Io(error.kind())
}

fn to_rustix_io_error(error: rustix::io::Errno) -> DeliverSinkError {
    DeliverSinkError::Io(io::Error::from(error).kind())
}

#[cfg(test)]
mod tests {
    use super::{DeliverTarget, parse_deliver_target, write_json_line};

    #[cfg(unix)]
    #[test]
    fn parse_deliver_target_resolves_parent_symlink_before_storing_new_file_path()
    -> Result<(), String> {
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let real_parent = temp_dir.path().join("real-parent");
        std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
        let alias_parent = temp_dir.path().join("alias-parent");
        std::os::unix::fs::symlink(&real_parent, &alias_parent)
            .map_err(|error| error.to_string())?;

        let requested_path = alias_parent.join("agent-context.jsonl");
        let target = format!("file:{}", path_text(&requested_path)?);

        match parse_deliver_target(&target).map_err(|error| error.to_string())? {
            DeliverTarget::NewFile(target) => {
                let expected = real_parent.join("agent-context.jsonl");
                if target.delivery_path() == expected {
                    Ok(())
                } else {
                    Err(format!(
                        "expected resolved delivery path {}, got {}",
                        expected.display(),
                        target.delivery_path().display()
                    ))
                }
            }
            DeliverTarget::Stdout => Err(String::from("expected file delivery target")),
        }
    }

    #[cfg(unix)]
    #[test]
    fn write_json_line_keeps_writes_on_validated_parent_inode_after_parent_path_swap()
    -> Result<(), String> {
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let real_parent = temp_dir.path().join("real-parent");
        std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
        let alias_parent = temp_dir.path().join("alias-parent");
        std::os::unix::fs::symlink(&real_parent, &alias_parent)
            .map_err(|error| error.to_string())?;

        let requested_path = alias_parent.join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&requested_path)?))
            .map_err(|error| error.to_string())?;

        let moved_parent = temp_dir.path().join("moved-parent");
        std::fs::rename(&real_parent, &moved_parent).map_err(|error| error.to_string())?;
        std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;

        write_json_line(&target, &serde_json::json!({"kind": "AgentContext"}))
            .map_err(|error| error.to_string())?;

        let moved_file = moved_parent.join("agent-context.jsonl");
        let replacement_file = real_parent.join("agent-context.jsonl");
        if !moved_file.exists() {
            return Err(format!(
                "expected pinned delivery at {}, but file was missing",
                moved_file.display()
            ));
        }
        if replacement_file.exists() {
            return Err(format!(
                "replacement parent path unexpectedly received delivery at {}",
                replacement_file.display()
            ));
        }

        let written = std::fs::read_to_string(&moved_file).map_err(|error| error.to_string())?;
        if written.contains("\"kind\":\"AgentContext\"") {
            Ok(())
        } else {
            Err(format!(
                "expected AgentContext payload in pinned file, got {written}"
            ))
        }
    }

    #[cfg(unix)]
    fn path_text(path: &std::path::Path) -> Result<String, String> {
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| String::from("test path must be UTF-8"))
    }
}
