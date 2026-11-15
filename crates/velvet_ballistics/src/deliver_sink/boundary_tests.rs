#![cfg(test)]

use std::fs;
use std::path::Path;

use serde_json::json;

use super::{
    DeliverSinkError, DeliverTarget, parse_deliver_target, persist_temp_file, temporary_path,
    validate_new_file_path, write_json_line,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn directory() -> TestResult<tempfile::TempDir> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deliver-sink-boundary-tests");
    fs::create_dir_all(&root)?;
    Ok(tempfile::Builder::new()
        .prefix("vb-deliver-")
        .tempdir_in(root)?)
}

fn target(path: &Path) -> TestResult<DeliverTarget> {
    Ok(
        parse_deliver_target(&format!("file:{}", path.to_str().ok_or("UTF-8 path")?))
            .map_err(|error| error.to_string())?,
    )
}

fn assert_blocked_without_writing(path: &Path) -> TestResult {
    assert!(!path.try_exists()?);
    assert_eq!(
        parse_deliver_target(&format!("file:{}", path.to_str().ok_or("UTF-8 path")?)),
        Err(DeliverSinkError::BlockedPath)
    );
    assert_eq!(
        validate_new_file_path(path),
        Err(DeliverSinkError::BlockedPath)
    );
    assert!(!path.try_exists()?);
    Ok(())
}

#[test]
fn deliver_boundary_dot_parent_aliases_are_blocked_parse_only() -> TestResult {
    let dir = directory()?;
    let unique = dir
        .path()
        .file_name()
        .ok_or("fixture name")?
        .to_string_lossy();
    for root in ["dev/shm", "proc", "sys"] {
        let path = format!("/home/lewis/../../{root}/{unique}-never-created.jsonl");
        assert_blocked_without_writing(Path::new(&path))?;
    }
    Ok(())
}

#[test]
fn deliver_boundary_symlink_parent_aliases_are_blocked_parse_only() -> TestResult {
    let dir = directory()?;
    for (index, root) in ["/dev/shm", "/proc", "/sys"].iter().enumerate() {
        let alias = dir.path().join(format!("alias-{index}"));
        std::os::unix::fs::symlink(root, &alias)?;
        let unique = dir.path().file_name().ok_or("fixture name")?;
        assert_blocked_without_writing(&alias.join(unique))?;
    }
    Ok(())
}

fn publish_name(name: &str) -> TestResult {
    let dir = directory()?;
    let path = dir.path().join(name);
    assert_eq!(target(&path)?, DeliverTarget::NewFile(path.clone()));
    let value = json!({"delivery": [1, true, "exact"]});
    assert_eq!(write_json_line(&target(&path)?, &value), Ok(()));
    assert_eq!(fs::read(&path)?, b"{\"delivery\":[1,true,\"exact\"]}\n");
    let stage = temporary_path(&path).map_err(|error| error.to_string())?;
    assert!(stage.file_name().ok_or("stage")?.as_encoded_bytes().len() <= 255);
    assert!(!stage.try_exists()?);
    assert_eq!(fs::read_dir(dir.path())?.count(), 1);
    assert_eq!(
        write_json_line(&DeliverTarget::NewFile(path.clone()), &value),
        Err(DeliverSinkError::ExistingFile)
    );
    assert_eq!(fs::read(&path)?, b"{\"delivery\":[1,true,\"exact\"]}\n");
    Ok(())
}

#[test]
fn deliver_boundary_250_byte_name_keeps_common_stage_spelling() -> TestResult {
    let dir = directory()?;
    let name = "a".repeat(250);
    let path = dir.path().join(&name);
    assert_eq!(
        temporary_path(&path).map_err(|error| error.to_string())?,
        dir.path().join(format!(".{name}.tmp"))
    );
    publish_name(&name)
}

#[test]
fn deliver_boundary_251_byte_name_publishes_exact_line() -> TestResult {
    publish_name(&"b".repeat(251))
}

#[test]
fn deliver_boundary_255_byte_name_publishes_exact_line() -> TestResult {
    publish_name(&"c".repeat(255))
}

#[test]
fn deliver_boundary_unicode_255_bytes_publishes_exact_line() -> TestResult {
    let name = format!("{}x", "é".repeat(127));
    assert_eq!(name.len(), 255);
    publish_name(&name)
}

#[test]
fn deliver_boundary_long_names_preserve_foreign_stage_ownership() -> TestResult {
    let dir = directory()?;
    let path = dir.path().join("d".repeat(255));
    let stage = temporary_path(&path).map_err(|error| error.to_string())?;
    fs::write(&stage, b"foreign stage")?;
    assert_eq!(
        write_json_line(&target(&path)?, &json!(1)),
        Err(DeliverSinkError::ExistingFile)
    );
    assert!(!path.try_exists()?);
    assert_eq!(fs::read(&stage)?, b"foreign stage");
    assert_eq!(fs::read_dir(dir.path())?.count(), 1);
    Ok(())
}

#[test]
fn deliver_boundary_long_owned_stage_is_cleaned_on_publication_conflict() -> TestResult {
    let dir = directory()?;
    let path = dir.path().join("e".repeat(255));
    let stage = temporary_path(&path).map_err(|error| error.to_string())?;
    fs::write(&stage, b"owned stage")?;
    fs::write(&path, b"existing target")?;
    assert_eq!(
        persist_temp_file(&stage, &path),
        Err(DeliverSinkError::ExistingFile)
    );
    assert!(!stage.try_exists()?);
    assert_eq!(fs::read(&path)?, b"existing target");
    Ok(())
}

#[test]
fn deliver_boundary_allowed_parent_alias_keeps_input_path_and_writes() -> TestResult {
    let dir = directory()?;
    let actual = dir.path().join("actual");
    fs::create_dir(&actual)?;
    let alias = dir.path().join("alias");
    std::os::unix::fs::symlink(&actual, &alias)?;
    let path = alias.join("out.jsonl");
    assert_eq!(target(&path)?, DeliverTarget::NewFile(path.clone()));
    assert_eq!(write_json_line(&target(&path)?, &json!("valid")), Ok(()));
    assert_eq!(fs::read(actual.join("out.jsonl"))?, b"\"valid\"\n");
    assert!(!actual.join(".out.jsonl.tmp").try_exists()?);
    Ok(())
}

#[test]
fn deliver_boundary_distinct_long_targets_keep_independent_stages_in_one_parent() -> TestResult {
    let dir = directory()?;
    let first = dir.path().join("first".repeat(51));
    let second = dir.path().join("second".repeat(42));
    assert_eq!(
        first
            .file_name()
            .ok_or("first name")?
            .as_encoded_bytes()
            .len(),
        255
    );
    assert_eq!(
        second
            .file_name()
            .ok_or("second name")?
            .as_encoded_bytes()
            .len(),
        252
    );
    let first_stage = temporary_path(&first).map_err(|error| error.to_string())?;
    let second_stage = temporary_path(&second).map_err(|error| error.to_string())?;
    assert_ne!(first_stage, second_stage);
    assert_eq!(
        temporary_path(&first).map_err(|error| error.to_string())?,
        first_stage
    );
    assert_eq!(
        temporary_path(&second).map_err(|error| error.to_string())?,
        second_stage
    );
    fs::write(&first_stage, b"foreign first stage")?;
    assert_eq!(
        write_json_line(&target(&second)?, &json!({"target": "second"})),
        Ok(())
    );
    assert_eq!(fs::read(&second)?, b"{\"target\":\"second\"}\n");
    assert!(!second_stage.try_exists()?);
    assert_eq!(fs::read(&first_stage)?, b"foreign first stage");
    assert_eq!(
        write_json_line(&target(&first)?, &json!({"target": "first"})),
        Err(DeliverSinkError::ExistingFile)
    );
    assert!(!first.try_exists()?);
    assert_eq!(fs::read(&first_stage)?, b"foreign first stage");
    assert_eq!(fs::read(&second)?, b"{\"target\":\"second\"}\n");
    assert_eq!(fs::read_dir(dir.path())?.count(), 2);
    Ok(())
}
