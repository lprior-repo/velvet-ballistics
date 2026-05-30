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
fn deliver_sink_cleans_temp_file_when_final_path_appears_before_persist() -> Result<(), String> {
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
