use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{File, OpenOptions, canonicalize, hard_link, remove_file};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

const STDOUT_TARGET: &str = "stdout";
const FILE_SCHEME: &str = "file";
const WEBHOOK_SCHEME: &str = "webhook";
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeliverTarget {
    Stdout,
    NewFile(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliverSinkError {
    MissingScheme,
    MissingFilePath,
    UnknownScheme,
    UnsupportedWebhook,
    RelativePath,
    MissingParent,
    Directory,
    BlockedPath,
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
            Self::Directory => f.write_str("deliver file target is a directory"),
            Self::BlockedPath => f.write_str("deliver file path uses a blocked system root"),
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
        DeliverTarget::NewFile(path) => write_json_line_to_new_file(path, value),
    }
}

fn parse_file_target(value: &str) -> Result<DeliverTarget, DeliverSinkError> {
    if value.is_empty() {
        return Err(DeliverSinkError::MissingFilePath);
    }
    let requested_path = PathBuf::from(value);
    validate_requested_file_path(&requested_path)?;
    let normalized_path = normalize_absolute_path(&requested_path)?;
    let delivery_path = resolve_new_file_path(&normalized_path)?;
    Ok(DeliverTarget::NewFile(delivery_path))
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

fn normalize_absolute_path(path: &Path) -> Result<PathBuf, DeliverSinkError> {
    if !path.is_absolute() {
        return Err(DeliverSinkError::RelativePath);
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }
    Ok(normalized)
}

fn resolve_new_file_path(path: &Path) -> Result<PathBuf, DeliverSinkError> {
    validate_requested_file_path(path)?;
    if is_blocked_root(path) {
        return Err(DeliverSinkError::BlockedPath);
    }
    if path.is_dir() {
        return Err(DeliverSinkError::Directory);
    }

    let Some(parent) = path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };
    if !parent.is_dir() {
        return Err(DeliverSinkError::MissingParent);
    }
    let resolved_parent = canonicalize(parent).map_err(to_io_error)?;
    if is_blocked_root(&resolved_parent) {
        return Err(DeliverSinkError::BlockedPath);
    }

    let Some(file_name) = path.file_name() else {
        return Err(DeliverSinkError::MissingFilePath);
    };
    let resolved_path = resolved_parent.join(file_name);
    if resolved_path.as_os_str().as_encoded_bytes().len() > MAX_PATH_BYTES {
        return Err(DeliverSinkError::OverlongPath);
    }
    if resolved_path.is_dir() {
        return Err(DeliverSinkError::Directory);
    }
    if resolved_path.try_exists().map_err(to_io_error)? {
        return Err(DeliverSinkError::ExistingFile);
    }

    Ok(resolved_path)
}

fn is_blocked_root(path: &Path) -> bool {
    path.starts_with("/dev") || path.starts_with("/proc") || path.starts_with("/sys")
}

fn write_json_line_to_new_file(path: &Path, value: &Value) -> Result<(), DeliverSinkError> {
    let delivery_path = resolve_new_file_path(path)?;
    let temp_path = temporary_path(&delivery_path)?;
    write_json_line_to_temp_file(&temp_path, value)?;
    persist_temp_file(&temp_path, &delivery_path)
}

fn write_json_line_to_temp_file(path: &Path, value: &Value) -> Result<(), DeliverSinkError> {
    let mut file = create_new_file(path)?;
    let write_result = write_json_line_to_writer(&mut file, value)
        .and_then(|()| file.sync_all().map_err(to_io_error));
    match write_result {
        Ok(()) => Ok(()),
        Err(write_error) => match remove_file(path).map_err(to_io_error) {
            Ok(()) => Err(write_error),
            Err(cleanup_error) => Err(cleanup_error),
        },
    }
}

fn persist_temp_file(temp_path: &Path, path: &Path) -> Result<(), DeliverSinkError> {
    match hard_link(temp_path, path) {
        Ok(()) => {
            cleanup_temp_file_best_effort(temp_path);
            Ok(())
        }
        Err(error) => {
            let delivery_error = match error.kind() {
                io::ErrorKind::AlreadyExists => DeliverSinkError::ExistingFile,
                kind => DeliverSinkError::Io(kind),
            };
            match remove_file(temp_path).map_err(to_io_error) {
                Ok(()) => Err(delivery_error),
                Err(cleanup_error) => Err(cleanup_error),
            }
        }
    }
}

fn cleanup_temp_file_best_effort(path: &Path) {
    match remove_file(path) {
        Ok(()) | Err(_) => {}
    }
}

fn temporary_path(path: &Path) -> Result<PathBuf, DeliverSinkError> {
    let Some(parent) = path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };
    let Some(file_name) = path.file_name() else {
        return Err(DeliverSinkError::MissingFilePath);
    };
    let preferred = preferred_temp_name(file_name);
    if path_with_name_len(parent, &preferred)? <= MAX_PATH_BYTES {
        return Ok(parent.join(preferred));
    }

    let hashed = hashed_temp_name(path);
    if path_with_name_len(parent, &hashed)? <= MAX_PATH_BYTES {
        return Ok(parent.join(hashed));
    }

    let fallback = OsString::from(".tmp");
    if path_with_name_len(parent, &fallback)? <= MAX_PATH_BYTES {
        return Ok(parent.join(fallback));
    }

    let minimal = OsString::from(".t");
    if path_with_name_len(parent, &minimal)? <= MAX_PATH_BYTES {
        return Ok(parent.join(minimal));
    }

    Err(DeliverSinkError::OverlongPath)
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

fn create_new_file(path: &Path) -> Result<File, DeliverSinkError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => DeliverSinkError::ExistingFile,
            kind => DeliverSinkError::Io(kind),
        })
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

#[cfg(test)]
mod tests {
    use super::{DeliverTarget, parse_deliver_target};

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
            DeliverTarget::NewFile(path) => {
                let expected = real_parent.join("agent-context.jsonl");
                if path == expected {
                    Ok(())
                } else {
                    Err(format!(
                        "expected resolved delivery path {}, got {}",
                        expected.display(),
                        path.display()
                    ))
                }
            }
            DeliverTarget::Stdout => Err(String::from("expected file delivery target")),
        }
    }

    #[cfg(unix)]
    fn path_text(path: &std::path::Path) -> Result<String, String> {
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| String::from("test path must be UTF-8"))
    }
}
