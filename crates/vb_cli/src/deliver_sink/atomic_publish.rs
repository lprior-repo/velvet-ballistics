use std::ffi::{OsStr, OsString};
use std::fs::{File, canonicalize};
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, CWD, Mode, OFlags, fstat, fsync, linkat, openat, statat, unlinkat};
use serde_json::Value;

use super::deliver_error::TempStageCreation;
use super::deliver_error::{
    DeliverSinkError, MAX_PATH_BYTES, MAX_TEMP_STAGE_ATTEMPTS, MINIMUM_STAGE_BASE_NAME, MODE,
    PublishedPathIdentity, to_io_error, to_rustix_io_error,
};
use super::deliver_target::{DeliverFileTarget, DeliverTarget};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Write a single JSON line to the deliver target.
///
/// For `Stdout` this is a simple write. For `NewFile` this stages a JSON line
/// into a temporary file, atomically links it as the final file, syncs the
/// directory, and cleans up the staging file — all while detecting races on
/// the final path.
pub(crate) fn write_json_line(
    target: &DeliverTarget,
    value: &Value,
) -> Result<(), DeliverSinkError> {
    match target {
        DeliverTarget::Stdout => write_json_line_to_writer(io::stdout().lock(), value),
        DeliverTarget::NewFile(target) => write_json_line_to_new_file(target, value),
    }
}

// ---------------------------------------------------------------------------
// File publish lifecycle
// ---------------------------------------------------------------------------

fn write_json_line_to_new_file(
    target: &DeliverFileTarget,
    value: &Value,
) -> Result<(), DeliverSinkError> {
    ensure_target_parent_current(target)?;
    let (temp_name, temp_file) = create_temp_stage_file(target)?;
    let published_identity = write_json_line_to_temp_file(target, &temp_name, temp_file, value)?;
    maybe_change_parent_path_before_link(target)?;
    persist_temp_file(target, &temp_name, published_identity)
}

fn write_json_line_to_temp_file(
    target: &DeliverFileTarget,
    temp_name: &OsStr,
    mut file: File,
    value: &Value,
) -> Result<PublishedPathIdentity, DeliverSinkError> {
    let write_result = write_json_line_to_writer(&mut file, value)
        .and_then(|()| file.sync_all().map_err(to_io_error));
    match write_result {
        Ok(()) => super::deliver_error::published_path_identity_for_file(&file),
        Err(write_error) => {
            let cleanup_failed =
                cleanup_unpublished_temp_file(&target.parent_dir, temp_name, write_error).is_err();
            if cleanup_failed {
                return Err(DeliverSinkError::PublishStateUnknown);
            }
            Err(DeliverSinkError::PublishStateUnknown)
        }
    }
}

fn persist_temp_file(
    target: &DeliverFileTarget,
    temp_name: &OsStr,
    published_identity: PublishedPathIdentity,
) -> Result<(), DeliverSinkError> {
    if let Err(error) = ensure_target_parent_current(target) {
        return cleanup_unpublished_temp_file(&target.parent_dir, temp_name, error);
    }

    match linkat(
        &target.parent_dir,
        temp_name,
        &target.parent_dir,
        &target.file_name,
        AtFlags::empty(),
    ) {
        Ok(()) => persist_linked_temp_file(target, temp_name, published_identity),
        Err(error) => {
            let delivery_error = if error == rustix::io::Errno::EXIST {
                DeliverSinkError::ExistingFile
            } else {
                to_rustix_io_error(error)
            };
            cleanup_unpublished_temp_file(&target.parent_dir, temp_name, delivery_error)
        }
    }
}

fn persist_linked_temp_file(
    target: &DeliverFileTarget,
    temp_name: &OsStr,
    published_identity: PublishedPathIdentity,
) -> Result<(), DeliverSinkError> {
    match sync_parent_directory(&target.parent_dir) {
        Ok(()) => {
            maybe_change_parent_path_after_link_sync(target)?;
            if let Err(error) = ensure_target_parent_current(target) {
                return rollback_linked_publish(target, temp_name, error);
            }
            cleanup_published_temp_file(target, temp_name, published_identity)
        }
        Err(sync_error) => rollback_linked_publish(target, temp_name, sync_error),
    }
}

#[cfg(test)]
fn maybe_change_parent_path_after_final_sync(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    let parent = target.delivery_parent()?;
    crate::deliver_sink::deliver_test_support::test_support::maybe_change_parent_path_after_final_sync(parent)
        .map_err(|error| DeliverSinkError::Io(error.kind()))?;
    Ok(())
}

#[cfg(not(test))]
fn maybe_change_parent_path_after_final_sync(
    _target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    Ok(())
}

#[cfg(test)]
fn maybe_change_final_path_after_final_sync(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    crate::deliver_sink::deliver_test_support::test_support::maybe_change_final_path_after_final_sync(target.delivery_path())
        .map_err(|error| DeliverSinkError::Io(error.kind()))?;
    Ok(())
}

// Only the final-path hook gets an `instrumented-cli` arm: `deliver_sink_integration.rs` drives the real binary with `VB_DELIVER_SINK_TEST_POST_COMMIT_FINAL_ACTION` to cover the post-publish `confirm_published_final_path` path end-to-end.
#[cfg(all(not(test), feature = "instrumented-cli"))]
fn maybe_change_final_path_after_final_sync(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    crate::deliver_sink::deliver_debug_test_support::maybe_change_final_path_after_final_sync(
        target.delivery_path(),
    )
}

#[cfg(all(not(test), not(feature = "instrumented-cli")))]
fn maybe_change_final_path_after_final_sync(
    _target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    Ok(())
}

fn confirm_published_final_path(
    target: &DeliverFileTarget,
    published_identity: PublishedPathIdentity,
) -> Result<(), DeliverSinkError> {
    if ensure_target_parent_current(target).is_err() {
        return Err(DeliverSinkError::PublishStateUnknown);
    }

    let final_stat = match statat(
        &target.parent_dir,
        &target.file_name,
        AtFlags::SYMLINK_NOFOLLOW,
    ) {
        Ok(stat) => stat,
        Err(_) => return Err(DeliverSinkError::PublishStateUnknown),
    };

    if final_stat.st_dev == published_identity.device
        && final_stat.st_ino == published_identity.inode
    {
        Ok(())
    } else {
        Err(DeliverSinkError::PublishStateUnknown)
    }
}

fn cleanup_published_temp_file(
    target: &DeliverFileTarget,
    temp_name: &OsStr,
    published_identity: PublishedPathIdentity,
) -> Result<(), DeliverSinkError> {
    if !cleanup_link_path(&target.parent_dir, temp_name) {
        return Err(DeliverSinkError::PublishStateUnknown);
    }

    if sync_parent_directory(&target.parent_dir).is_err() {
        return Err(DeliverSinkError::PublishStateUnknown);
    }

    maybe_change_parent_path_after_final_sync(target)?;
    maybe_change_final_path_after_final_sync(target)?;

    confirm_published_final_path(target, published_identity)
}

fn rollback_linked_publish(
    target: &DeliverFileTarget,
    temp_name: &OsStr,
    publish_error: DeliverSinkError,
) -> Result<(), DeliverSinkError> {
    let final_cleared = cleanup_link_path(&target.parent_dir, &target.file_name);
    let temp_cleared = cleanup_link_path(&target.parent_dir, temp_name);
    let rollback_changed = final_cleared || temp_cleared;
    let rollback_durable = if rollback_changed {
        sync_parent_directory(&target.parent_dir).is_ok()
    } else {
        false
    };
    if final_cleared && temp_cleared && rollback_durable {
        Err(publish_error)
    } else {
        Err(DeliverSinkError::PublishStateUnknown)
    }
}

fn cleanup_unpublished_temp_file(
    parent_dir: &OwnedFd,
    temp_name: &OsStr,
    delivery_error: DeliverSinkError,
) -> Result<(), DeliverSinkError> {
    // Best-effort cleanup; the original semantic error (e.g. `ExistingFile`
    // for the linkat-EXIST branch) is what the caller actually needs to
    // surface, so we always return it regardless of whether the unlinkat
    // succeeded, failed, or was forced to fail by the test/debug hooks.
    let _ = cleanup_link_path(parent_dir, temp_name);
    Err(delivery_error)
}

fn cleanup_link_path(parent_dir: &OwnedFd, path: &OsStr) -> bool {
    #[cfg(test)]
    if crate::deliver_sink::deliver_test_support::test_support::should_fail_cleanup(path) {
        return false;
    }

    #[cfg(all(not(test), feature = "instrumented-cli"))]
    if crate::deliver_sink::deliver_debug_test_support::should_fail_cleanup(path) {
        return false;
    }

    match unlinkat(parent_dir, path, AtFlags::empty()) {
        Ok(()) => true,
        Err(error) => {
            error == rustix::io::Errno::NOENT
                || super::deliver_error::path_is_absent(parent_dir, path)
        }
    }
}

fn ensure_target_parent_current(target: &DeliverFileTarget) -> Result<(), DeliverSinkError> {
    ensure_parent_matches_path(&target.parent_dir, target.delivery_parent()?)
}

fn sync_parent_directory(parent_dir: &OwnedFd) -> Result<(), DeliverSinkError> {
    #[cfg(test)]
    if let Some(result) =
        crate::deliver_sink::deliver_test_support::test_support::next_sync_result()
    {
        return result;
    }

    #[cfg(all(not(test), feature = "instrumented-cli"))]
    if let Some(result) = crate::deliver_sink::deliver_debug_test_support::next_sync_result() {
        return result;
    }

    fsync(parent_dir).map_err(to_rustix_io_error)
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

// ---------------------------------------------------------------------------
// Temporary staging helpers
// ---------------------------------------------------------------------------

fn create_temp_stage_file(
    target: &DeliverFileTarget,
) -> Result<(OsString, File), DeliverSinkError> {
    let Some(parent) = target.delivery_path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };

    let base_names: [OsString; 4] = [
        preferred_temp_name(&target.file_name),
        hashed_temp_name(target.delivery_path()),
        OsString::from(".tmp"),
        OsString::from(MINIMUM_STAGE_BASE_NAME),
    ];

    try_create_with_stage_base_names(&target.parent_dir, parent, &base_names)
}

fn try_create_with_stage_base_names(
    parent_dir: &OwnedFd,
    parent: &Path,
    base_names: &[OsString],
) -> Result<(OsString, File), DeliverSinkError> {
    let mut exhausted_candidates = false;
    for base_name in base_names {
        match create_temp_stage_file_from_base_name(parent_dir, parent, base_name)? {
            TempStageCreation::Created(stage_file) => return Ok(stage_file),
            TempStageCreation::Exhausted => exhausted_candidates = true,
            TempStageCreation::NameTooLong {
                had_occupied_candidate,
            } => exhausted_candidates |= had_occupied_candidate,
        }
    }

    if exhausted_candidates {
        Err(DeliverSinkError::StagingUnavailable)
    } else {
        Err(DeliverSinkError::OverlongPath)
    }
}

pub(super) fn preferred_temp_name(file_name: &OsStr) -> OsString {
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp");
    temp_name
}

pub(super) fn hashed_temp_name(path: &Path) -> OsString {
    use std::fmt::Write as _;
    let digest = blake3::hash(path.as_os_str().as_encoded_bytes());
    let mut temp_name = String::with_capacity(".vb".len().saturating_add(16));
    temp_name.push_str(".vb");
    for byte in &digest.as_bytes()[..8] {
        let _ = write!(&mut temp_name, "{byte:02x}").ok();
    }
    OsString::from(temp_name)
}

fn create_temp_stage_file_from_base_name(
    parent_dir: &OwnedFd,
    parent: &Path,
    base_name: &OsStr,
) -> Result<TempStageCreation, DeliverSinkError> {
    let mut had_occupied_candidate = false;

    for attempt in 0..MAX_TEMP_STAGE_ATTEMPTS {
        let candidate_name = temp_stage_name(base_name, attempt);
        if super::deliver_target::path_with_name_len(parent, &candidate_name)? > MAX_PATH_BYTES {
            return Ok(TempStageCreation::NameTooLong {
                had_occupied_candidate,
            });
        }

        match create_new_file_at(parent_dir, &candidate_name) {
            Ok(file) => return Ok(TempStageCreation::Created((candidate_name, file))),
            Err(DeliverSinkError::ExistingFile) => had_occupied_candidate = true,
            Err(DeliverSinkError::OverlongPath) => {
                return Ok(TempStageCreation::NameTooLong {
                    had_occupied_candidate,
                });
            }
            Err(error) => return Err(error),
        }
    }

    Ok(TempStageCreation::Exhausted)
}

pub(super) fn temp_stage_name(base_name: &OsStr, attempt: usize) -> OsString {
    if attempt == 0 {
        return base_name.to_os_string();
    }

    let mut candidate_name = base_name.to_os_string();
    candidate_name.push(".");
    candidate_name.push(attempt.to_string());
    candidate_name
}

fn create_new_file_at(parent_dir: &OwnedFd, path: &OsStr) -> Result<File, DeliverSinkError> {
    openat(
        parent_dir,
        path,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
        MODE,
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

// ---------------------------------------------------------------------------
// Pre-link parent-change hooks (test / instrumented)
// ---------------------------------------------------------------------------

#[cfg(test)]
fn maybe_change_parent_path_before_link(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    let parent = target.delivery_parent()?;
    crate::deliver_sink::deliver_test_support::test_support::maybe_change_parent_path_before_link(
        parent,
    )
    .map_err(|error| DeliverSinkError::Io(error.kind()))?;
    Ok(())
}

#[cfg(not(test))]
fn maybe_change_parent_path_before_link(
    _target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    Ok(())
}

#[cfg(test)]
fn maybe_change_parent_path_after_link_sync(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    let parent = target.delivery_parent()?;
    crate::deliver_sink::deliver_test_support::test_support::maybe_change_parent_path_after_link_sync(parent)
        .map_err(|error| DeliverSinkError::Io(error.kind()))?;
    Ok(())
}

#[cfg(not(test))]
fn maybe_change_parent_path_after_link_sync(
    _target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON writer (shared)
// ---------------------------------------------------------------------------

pub(super) fn write_json_line_to_writer<W: Write>(
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
