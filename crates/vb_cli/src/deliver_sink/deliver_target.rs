use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, FileType, Mode, OFlags, openat, statat, CWD};
use std::os::fd::OwnedFd;

use super::deliver_error::{DeliverSinkError, MAX_PATH_BYTES};
use super::deliver_error::to_rustix_io_error;

const STDOUT_TARGET: &str = "stdout";
const FILE_SCHEME: &str = "file";
const WEBHOOK_SCHEME: &str = "webhook";

pub(crate) struct DeliverFileTarget {
    pub(crate) parent_dir: OwnedFd,
    pub(crate) file_name: OsString,
    pub(crate) delivery_path: PathBuf,
}

impl DeliverFileTarget {
    pub(crate) fn delivery_path(&self) -> &Path {
        &self.delivery_path
    }

    pub(crate) fn delivery_parent(&self) -> Result<&Path, DeliverSinkError> {
        self.delivery_path
            .parent()
            .ok_or(DeliverSinkError::MissingParent)
    }
}

/// A deliver target that can write JSON lines either to stdout or to a new file.
///
/// `Stdout` writes a single JSON line to standard output. `NewFile` resolves
/// the requested path, opens the parent directory, and is ready to atomically
/// stage and publish a JSON-line file.
pub(crate) enum DeliverTarget {
    Stdout,
    NewFile(DeliverFileTarget),
}

/// Parse a raw deliver-target string into a typed `DeliverTarget`.
///
/// Supported formats:
/// - `"stdout"` → `DeliverTarget::Stdout`
/// - `"file:<path>"` → `DeliverTarget::NewFile` (after validation & resolution)
/// - `"webhook:<url>"` → unsupported
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
    let (parent_dir, resolved_parent) = open_and_resolve_parent(parent)?;
    let Some(file_name) = path.file_name() else {
        return Err(DeliverSinkError::MissingFilePath);
    };
    let resolved_path = resolved_parent.join(file_name);
    validate_resolved_target(&parent_dir, &resolved_parent, file_name, &resolved_path)?;

    Ok(DeliverFileTarget {
        parent_dir,
        file_name: file_name.to_os_string(),
        delivery_path: resolved_path,
    })
}

fn open_and_resolve_parent(parent: &Path) -> Result<(OwnedFd, PathBuf), DeliverSinkError> {
    let parent_dir = open_parent_directory(parent)?;
    #[cfg(test)]
    #[allow(clippy::let_underscore_must_use)]
    let _ = crate::deliver_sink::deliver_test_support::test_support::maybe_change_parent_path(parent);
    let resolved_parent = canonicalize_parent_path(parent)?;
    if is_blocked_root(&resolved_parent) {
        return Err(DeliverSinkError::BlockedPath);
    }
    ensure_parent_matches_path(&parent_dir, &resolved_parent)?;
    Ok((parent_dir, resolved_parent))
}

fn validate_resolved_target(
    parent_dir: &OwnedFd,
    resolved_parent: &Path,
    file_name: &OsStr,
    resolved_path: &Path,
) -> Result<(), DeliverSinkError> {
    if resolved_path.as_os_str().as_encoded_bytes().len() > MAX_PATH_BYTES {
        return Err(DeliverSinkError::OverlongPath);
    }

    if path_with_name_len(resolved_parent, OsStr::new(super::deliver_error::MINIMUM_STAGE_BASE_NAME))? > MAX_PATH_BYTES {
        return Err(DeliverSinkError::OverlongPath);
    }

    validate_target_absent(parent_dir, file_name)?;

    Ok(())
}

fn is_blocked_root(path: &Path) -> bool {
    path.starts_with("/dev") || path.starts_with("/proc") || path.starts_with("/sys")
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
    std::fs::canonicalize(parent).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => DeliverSinkError::ParentChanged,
        kind => DeliverSinkError::Io(kind),
    })
}

fn ensure_parent_matches_path(
    parent_dir: &OwnedFd,
    resolved_parent: &Path,
) -> Result<(), DeliverSinkError> {
    let parent_stat = rustix::fs::fstat(parent_dir).map_err(to_rustix_io_error)?;
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
    rustix::fs::unlinkat(parent_dir, path, AtFlags::empty()).map_err(to_rustix_io_error)
}

pub(super) fn path_with_name_len(parent: &Path, file_name: &OsStr) -> Result<usize, DeliverSinkError> {
    let separator = if parent == Path::new("/") { 0 } else { 1 };
    parent
        .as_os_str()
        .as_encoded_bytes()
        .len()
        .checked_add(separator)
        .and_then(|value| value.checked_add(file_name.as_encoded_bytes().len()))
        .ok_or(DeliverSinkError::OverlongPath)
}
