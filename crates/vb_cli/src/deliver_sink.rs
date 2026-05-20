use std::ffi::OsString;
use std::fmt;
use std::fs::{File, OpenOptions, hard_link, remove_file};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
            Self::MissingScheme => f.write_str("deliver target must be stdout or file:<path>"),
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
    let path = PathBuf::from(value);
    validate_new_file_path(&path)?;
    Ok(DeliverTarget::NewFile(path))
}

fn validate_new_file_path(path: &Path) -> Result<(), DeliverSinkError> {
    if path.as_os_str().as_encoded_bytes().len() > MAX_PATH_BYTES {
        return Err(DeliverSinkError::OverlongPath);
    }
    if !path.is_absolute() {
        return Err(DeliverSinkError::RelativePath);
    }
    if is_blocked_root(path) {
        return Err(DeliverSinkError::BlockedPath);
    }
    if path.is_dir() {
        return Err(DeliverSinkError::Directory);
    }
    if path.try_exists().map_err(to_io_error)? {
        return Err(DeliverSinkError::ExistingFile);
    }

    let Some(parent) = path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };
    if !parent.is_dir() {
        return Err(DeliverSinkError::MissingParent);
    }

    Ok(())
}

fn is_blocked_root(path: &Path) -> bool {
    path.starts_with("/dev") || path.starts_with("/proc") || path.starts_with("/sys")
}

fn write_json_line_to_new_file(path: &Path, value: &Value) -> Result<(), DeliverSinkError> {
    validate_new_file_path(path)?;
    let temp_path = temporary_path(path)?;
    write_json_line_to_temp_file(&temp_path, value)?;
    persist_temp_file(&temp_path, path)
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
        Ok(()) => remove_file(temp_path).map_err(to_io_error),
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

fn temporary_path(path: &Path) -> Result<PathBuf, DeliverSinkError> {
    let Some(parent) = path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };
    let Some(file_name) = path.file_name() else {
        return Err(DeliverSinkError::MissingFilePath);
    };
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp");
    Ok(parent.join(temp_name))
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
    use std::fs;
    use std::path::Path;

    use serde_json::json;

    use super::{
        DeliverSinkError, DeliverTarget, parse_deliver_target, persist_temp_file, temporary_path,
        write_json_line, write_json_line_to_writer,
    };

    #[test]
    fn deliver_sink_parses_stdout_target() {
        assert_eq!(parse_deliver_target("stdout"), Ok(DeliverTarget::Stdout));
    }

    #[test]
    fn deliver_sink_requires_file_scheme_for_paths() {
        assert_eq!(
            parse_deliver_target("/tmp/out.jsonl"),
            Err(DeliverSinkError::MissingScheme)
        );
    }

    #[test]
    fn deliver_sink_rejects_unknown_and_webhook_schemes() {
        assert_eq!(
            parse_deliver_target("s3:/tmp/out"),
            Err(DeliverSinkError::UnknownScheme)
        );
        assert_eq!(
            parse_deliver_target("webhook:https://example.invalid/hook"),
            Err(DeliverSinkError::UnsupportedWebhook)
        );
    }

    #[test]
    fn deliver_sink_rejects_relative_file_path() {
        assert_eq!(
            parse_deliver_target("file:out.jsonl"),
            Err(DeliverSinkError::RelativePath)
        );
    }

    #[test]
    fn deliver_sink_rejects_missing_parent() -> Result<(), String> {
        let dir = deliver_test_tempdir()?;
        let path = dir.path().join("missing-parent").join("out.jsonl");

        assert_eq!(
            parse_deliver_target(format!("file:{}", path_text(&path)?).as_str()),
            Err(DeliverSinkError::MissingParent)
        );
        Ok(())
    }

    #[test]
    fn deliver_sink_rejects_directory_target() -> Result<(), String> {
        let dir = deliver_test_tempdir()?;

        assert_eq!(
            parse_deliver_target(format!("file:{}", path_text(dir.path())?).as_str()),
            Err(DeliverSinkError::Directory)
        );
        Ok(())
    }

    #[test]
    fn deliver_sink_rejects_blocked_roots() {
        assert_eq!(
            parse_deliver_target("file:/dev/vb-deliver.jsonl"),
            Err(DeliverSinkError::BlockedPath)
        );
        assert_eq!(
            parse_deliver_target("file:/proc/vb-deliver.jsonl"),
            Err(DeliverSinkError::BlockedPath)
        );
        assert_eq!(
            parse_deliver_target("file:/sys/vb-deliver.jsonl"),
            Err(DeliverSinkError::BlockedPath)
        );
    }

    #[test]
    fn deliver_sink_rejects_existing_file() -> Result<(), String> {
        let dir = deliver_test_tempdir()?;
        let path = dir.path().join("out.jsonl");
        fs::write(&path, b"already here").map_err(|error| error.to_string())?;

        assert_eq!(
            parse_deliver_target(format!("file:{}", path_text(&path)?).as_str()),
            Err(DeliverSinkError::ExistingFile)
        );
        Ok(())
    }

    #[test]
    fn deliver_sink_rejects_overlong_path() {
        let mut path = String::from("file:/tmp/");
        path.extend(std::iter::repeat_n('a', 4092));

        assert_eq!(
            parse_deliver_target(&path),
            Err(DeliverSinkError::OverlongPath)
        );
    }

    #[test]
    fn deliver_sink_parses_absolute_new_file_path() -> Result<(), String> {
        let dir = deliver_test_tempdir()?;
        let path = dir.path().join("out.jsonl");

        assert_eq!(
            parse_deliver_target(format!("file:{}", path_text(&path)?).as_str()),
            Ok(DeliverTarget::NewFile(path))
        );
        Ok(())
    }

    #[test]
    fn deliver_sink_writes_value_unchanged_as_one_json_line_to_writer() {
        let value = json!({"b":[true,null,3],"a":"text"});
        let mut out = Vec::new();

        assert_eq!(write_json_line_to_writer(&mut out, &value), Ok(()));
        assert_eq!(
            out,
            br#"{"a":"text","b":[true,null,3]}"#
                .iter()
                .copied()
                .chain([b'\n'])
                .collect::<Vec<u8>>()
        );
    }

    #[test]
    fn deliver_sink_writes_value_unchanged_as_one_json_line_to_new_file() -> Result<(), String> {
        let dir = deliver_test_tempdir()?;
        let path = dir.path().join("out.jsonl");
        let value = json!([{"z":1},"raw"]);
        let target = DeliverTarget::NewFile(path.clone());

        let write_result = write_json_line(&target, &value);
        let bytes = fs::read(&path).map_err(|error| error.to_string())?;

        assert_eq!(write_result, Ok(()));
        assert_eq!(bytes, b"[{\"z\":1},\"raw\"]\n".to_vec());
        assert!(
            !temporary_path(&path)
                .map_err(|error| error.to_string())?
                .exists()
        );
        Ok(())
    }

    #[test]
    fn deliver_sink_cleans_temp_file_when_final_path_appears_before_persist() -> Result<(), String>
    {
        let dir = deliver_test_tempdir()?;
        let path = dir.path().join("out.jsonl");
        let temp_path = temporary_path(&path).map_err(|error| error.to_string())?;
        fs::write(&temp_path, b"payload").map_err(|error| error.to_string())?;
        fs::write(&path, b"existing").map_err(|error| error.to_string())?;

        let result = persist_temp_file(&temp_path, &path);

        assert_eq!(result, Err(DeliverSinkError::ExistingFile));
        assert!(!temp_path.exists());
        assert_eq!(
            fs::read(&path).map_err(|error| error.to_string())?,
            b"existing".to_vec()
        );
        Ok(())
    }

    fn path_text(path: &Path) -> Result<String, String> {
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| String::from("test path must be UTF-8"))
    }

    fn deliver_test_tempdir() -> Result<tempfile::TempDir, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deliver-sink-unit-tmp");
        std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        tempfile::Builder::new()
            .prefix("vb-deliver-unit-")
            .tempdir_in(root)
            .map_err(|error| error.to_string())
    }
}
