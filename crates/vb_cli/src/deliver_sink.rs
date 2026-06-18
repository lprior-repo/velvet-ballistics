use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{File, canonicalize};
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};

use rustix::fs::{
    AtFlags, CWD, FileType, Mode, OFlags, fstat, fsync, linkat, openat, statat, unlinkat,
};
use serde_json::Value;

const STDOUT_TARGET: &str = "stdout";
const FILE_SCHEME: &str = "file";
const WEBHOOK_SCHEME: &str = "webhook";
// Linux path strings are effectively capped at 4095 bytes because the final
// NUL terminator consumes the last PATH_MAX byte.
const MAX_PATH_BYTES: usize = 4095;
const MAX_TEMP_STAGE_ATTEMPTS: usize = 8;
const MINIMUM_STAGE_BASE_NAME: &str = ".t";
// Bare (no envelope) message returned by `Display` for `PublishStateUnknown`;
// the binary prefixes `deliver failed: ` before writing it to stderr. Exposed
// as `pub` so the integration test can assert against the single source of
// truth instead of duplicating the literal.
pub const PUBLISH_STATE_UNKNOWN_MESSAGE: &str =
    "deliver publish outcome is unknown after post-commit state could not be confirmed";
// Owner-only read+write mode for newly created delivery files. Computed via
// `Mode::RUSR.union(Mode::WUSR)` at compile time so the value avoids a runtime
// call and the per-bit assembly stays in the constant evaluator.
const MODE: Mode = Mode::RUSR.union(Mode::WUSR);

enum TempStageCreation {
    Created((OsString, File)),
    Exhausted,
    NameTooLong { had_occupied_candidate: bool },
}

#[derive(Clone, Copy)]
struct PublishedPathIdentity {
    device: u64,
    inode: u64,
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

    fn delivery_parent(&self) -> Result<&Path, DeliverSinkError> {
        self.delivery_path
            .parent()
            .ok_or(DeliverSinkError::MissingParent)
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
    PublishStateUnknown,
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
            Self::ParentChanged => f.write_str("deliver file parent changed before publish"),
            Self::Directory => f.write_str("deliver file target is a directory"),
            Self::BlockedPath => f.write_str("deliver file path uses a blocked system root"),
            Self::StagingUnavailable => {
                f.write_str("deliver temporary staging path is unavailable")
            }
            Self::ExistingFile => f.write_str("deliver file target already exists"),
            Self::OverlongPath => f.write_str("deliver file path is too long"),
            Self::PublishStateUnknown => f.write_str(PUBLISH_STATE_UNKNOWN_MESSAGE),
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
    let _ = test_support::maybe_change_parent_path(parent);
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

    if path_with_name_len(resolved_parent, OsStr::new(MINIMUM_STAGE_BASE_NAME))? > MAX_PATH_BYTES {
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
        Ok(()) => published_path_identity_for_file(&file),
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

#[cfg(test)]
fn maybe_change_parent_path_after_final_sync(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    let parent = target.delivery_parent()?;
    test_support::maybe_change_parent_path_after_final_sync(parent)
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
    test_support::maybe_change_final_path_after_final_sync(target.delivery_path())
        .map_err(|error| DeliverSinkError::Io(error.kind()))?;
    Ok(())
}

// Only the final-path hook gets an `instrumented-cli` arm: `deliver_sink_integration.rs` drives the real binary with `VB_DELIVER_SINK_TEST_POST_COMMIT_FINAL_ACTION` to cover the post-publish `confirm_published_final_path` path end-to-end.
#[cfg(all(not(test), feature = "instrumented-cli"))]
fn maybe_change_final_path_after_final_sync(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    debug_test_support::maybe_change_final_path_after_final_sync(target.delivery_path())
}

#[cfg(all(not(test), not(feature = "instrumented-cli")))]
fn maybe_change_final_path_after_final_sync(
    _target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    Ok(())
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
    if test_support::should_fail_cleanup(path) {
        return false;
    }

    #[cfg(all(not(test), feature = "instrumented-cli"))]
    if debug_test_support::should_fail_cleanup(path) {
        return false;
    }

    match unlinkat(parent_dir, path, AtFlags::empty()) {
        Ok(()) => true,
        Err(error) => error == rustix::io::Errno::NOENT || path_is_absent(parent_dir, path),
    }
}

fn ensure_target_parent_current(target: &DeliverFileTarget) -> Result<(), DeliverSinkError> {
    ensure_parent_matches_path(&target.parent_dir, target.delivery_parent()?)
}

fn path_is_absent(parent_dir: &OwnedFd, path: &OsStr) -> bool {
    match statat(parent_dir, path, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(_) => false,
        Err(error) => error == rustix::io::Errno::NOENT,
    }
}

fn published_path_identity_for_file(
    file: &File,
) -> Result<PublishedPathIdentity, DeliverSinkError> {
    let stat = fstat(file).map_err(to_rustix_io_error)?;
    Ok(PublishedPathIdentity {
        device: stat.st_dev,
        inode: stat.st_ino,
    })
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

fn sync_parent_directory(parent_dir: &OwnedFd) -> Result<(), DeliverSinkError> {
    #[cfg(test)]
    if let Some(result) = test_support::next_sync_result() {
        return result;
    }

    #[cfg(all(not(test), feature = "instrumented-cli"))]
    if let Some(result) = debug_test_support::next_sync_result() {
        return result;
    }

    fsync(parent_dir).map_err(to_rustix_io_error)
}

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

fn preferred_temp_name(file_name: &OsStr) -> OsString {
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp");
    temp_name
}

fn hashed_temp_name(path: &Path) -> OsString {
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
        if path_with_name_len(parent, &candidate_name)? > MAX_PATH_BYTES {
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

#[cfg(test)]
fn maybe_change_parent_path_before_link(
    target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    let parent = target.delivery_parent()?;
    test_support::maybe_change_parent_path_before_link(parent)
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
    test_support::maybe_change_parent_path_after_link_sync(parent)
        .map_err(|error| DeliverSinkError::Io(error.kind()))?;
    Ok(())
}

#[cfg(not(test))]
fn maybe_change_parent_path_after_link_sync(
    _target: &DeliverFileTarget,
) -> Result<(), DeliverSinkError> {
    Ok(())
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
    let separator = if parent == Path::new("/") { 0 } else { 1 };
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
    if error == rustix::io::Errno::NAMETOOLONG {
        DeliverSinkError::OverlongPath
    } else {
        DeliverSinkError::Io(io::Error::from(error).kind())
    }
}

#[cfg(all(not(test), feature = "instrumented-cli"))]
mod debug_test_support {
    #![allow(
        clippy::absurd_extreme_comparisons,
        clippy::approx_constant,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::bool_comparison,
        clippy::borrow_deref_ref,
        clippy::cast_abs_to_unsigned,
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::clone_on_copy,
        clippy::cloned_ref_to_slice_refs,
        clippy::cmp_owned,
        clippy::collapsible_if,
        clippy::collapsible_match,
        clippy::const_is_empty,
        clippy::derivable_impls,
        clippy::duplicated_attributes,
        clippy::enum_variant_names,
        clippy::err_expect,
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::explicit_counter_loop,
        clippy::field_reassign_with_default,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::get_first,
        clippy::identity_op,
        clippy::if_let_mutex,
        clippy::if_not_else,
        clippy::if_same_then_else,
        clippy::implicit_clone,
        clippy::implicit_saturating_sub,
        clippy::inconsistent_struct_constructor,
        clippy::indexing_slicing,
        clippy::inefficient_to_string,
        clippy::io_other_error,
        clippy::items_after_test_module,
        clippy::iter_count,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_futures,
        clippy::large_stack_arrays,
        clippy::large_types_passed_by_value,
        clippy::len_zero,
        clippy::let_and_return,
        clippy::let_underscore_must_use,
        clippy::manual_contains,
        clippy::manual_div_ceil,
        clippy::manual_let_else,
        clippy::manual_map,
        clippy::manual_range_contains,
        clippy::manual_saturating_arithmetic,
        clippy::manual_strip,
        clippy::manual_unwrap_or,
        clippy::manual_unwrap_or_default,
        clippy::map_clone,
        clippy::map_flatten,
        clippy::match_like_matches_macro,
        clippy::misnamed_getters,
        clippy::missing_safety_doc,
        clippy::module_inception,
        clippy::multiple_bound_locations,
        clippy::mutable_key_type,
        clippy::needless_bool,
        clippy::needless_bool_assign,
        clippy::needless_borrow,
        clippy::needless_borrows_for_generic_args,
        clippy::needless_collect,
        clippy::needless_pass_by_value,
        clippy::needless_range_loop,
        clippy::needless_return,
        clippy::needless_update,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::new_without_default,
        clippy::nonminimal_bool,
        clippy::ok_expect,
        clippy::option_as_ref_cloned,
        clippy::option_as_ref_deref,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::redundant_clone,
        clippy::redundant_closure,
        clippy::redundant_else,
        clippy::redundant_field_names,
        clippy::redundant_guards,
        clippy::redundant_locals,
        clippy::redundant_pattern_matching,
        clippy::redundant_pub_crate,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::shadow_unrelated,
        clippy::similar_names,
        clippy::single_match,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::todo,
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::trivially_copy_pass_by_ref,
        clippy::type_complexity,
        clippy::unimplemented,
        clippy::uninlined_format_args,
        clippy::unnecessary_cast,
        clippy::unnecessary_fallible_conversions,
        clippy::unnecessary_map_or,
        clippy::unnecessary_mut_passed,
        clippy::unnecessary_sort_by,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_wraps,
        clippy::unneeded_struct_pattern,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_async,
        clippy::unused_io_amount,
        clippy::unused_self,
        clippy::unused_trait_names,
        clippy::unwrap_used,
        clippy::useless_asref,
        clippy::useless_conversion,
        clippy::useless_format,
        clippy::useless_vec,
        clippy::vec_init_then_push,
        clippy::wildcard_enum_match_arm,
        clippy::wildcard_imports,
        dead_code,
        let_underscore_drop,
        unused_imports,
        unused_variables
    )]
    #![allow(
        clippy::absurd_extreme_comparisons,
        clippy::approx_constant,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::bool_comparison,
        clippy::borrow_deref_ref,
        clippy::cast_abs_to_unsigned,
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::clone_on_copy,
        clippy::cloned_ref_to_slice_refs,
        clippy::cmp_owned,
        clippy::collapsible_if,
        clippy::collapsible_match,
        clippy::derivable_impls,
        clippy::duplicated_attributes,
        clippy::enum_variant_names,
        clippy::err_expect,
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::explicit_counter_loop,
        clippy::field_reassign_with_default,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::get_first,
        clippy::identity_op,
        clippy::if_let_mutex,
        clippy::if_not_else,
        clippy::if_same_then_else,
        clippy::implicit_clone,
        clippy::implicit_saturating_sub,
        clippy::inconsistent_struct_constructor,
        clippy::indexing_slicing,
        clippy::inefficient_to_string,
        clippy::io_other_error,
        clippy::items_after_test_module,
        clippy::iter_count,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_futures,
        clippy::large_stack_arrays,
        clippy::large_types_passed_by_value,
        clippy::len_zero,
        clippy::let_and_return,
        clippy::let_underscore_must_use,
        clippy::manual_contains,
        clippy::manual_div_ceil,
        clippy::manual_let_else,
        clippy::manual_map,
        clippy::manual_saturating_arithmetic,
        clippy::manual_strip,
        clippy::manual_unwrap_or,
        clippy::manual_unwrap_or_default,
        clippy::map_clone,
        clippy::map_flatten,
        clippy::match_like_matches_macro,
        clippy::misnamed_getters,
        clippy::missing_safety_doc,
        clippy::module_inception,
        clippy::multiple_bound_locations,
        clippy::mutable_key_type,
        clippy::needless_bool,
        clippy::needless_bool_assign,
        clippy::needless_borrow,
        clippy::needless_borrows_for_generic_args,
        clippy::needless_collect,
        clippy::needless_pass_by_value,
        clippy::needless_range_loop,
        clippy::needless_return,
        clippy::needless_update,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::new_without_default,
        clippy::nonminimal_bool,
        clippy::ok_expect,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::redundant_clone,
        clippy::redundant_closure,
        clippy::redundant_else,
        clippy::redundant_guards,
        clippy::redundant_locals,
        clippy::redundant_pattern_matching,
        clippy::redundant_pub_crate,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::shadow_unrelated,
        clippy::similar_names,
        clippy::single_match,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::todo,
        clippy::too_many_lines,
        clippy::trivially_copy_pass_by_ref,
        clippy::type_complexity,
        clippy::unimplemented,
        clippy::uninlined_format_args,
        clippy::unnecessary_cast,
        clippy::unnecessary_fallible_conversions,
        clippy::unnecessary_map_or,
        clippy::unnecessary_mut_passed,
        clippy::unnecessary_sort_by,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_wraps,
        clippy::unneeded_struct_pattern,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_async,
        clippy::unused_io_amount,
        clippy::unused_self,
        clippy::unused_trait_names,
        clippy::unwrap_used,
        clippy::useless_asref,
        clippy::useless_conversion,
        clippy::useless_format,
        clippy::useless_vec,
        clippy::vec_init_then_push,
        clippy::wildcard_enum_match_arm,
        clippy::wildcard_imports,
        dead_code,
        let_underscore_drop,
        unused_imports,
        unused_variables
    )]

    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::Path;

    use super::DeliverSinkError;

    const CLEANUP_FAILURES_ENV: &str = "VB_DELIVER_SINK_TEST_CLEANUP_FAILURES";
    const POST_COMMIT_FINAL_ACTION_ENV: &str = "VB_DELIVER_SINK_TEST_POST_COMMIT_FINAL_ACTION";
    const SYNC_RESULTS_ENV: &str = "VB_DELIVER_SINK_TEST_SYNC_RESULTS";
    const RIVAL_REPLACEMENT_BYTES: &[u8] = b"rival replacement\n";

    enum FinalPathChange {
        UnlinkFinalPath,
        ReplaceFinalPath,
    }

    struct Hooks {
        loaded: bool,
        cleanup_failures: Vec<OsString>,
        post_commit_final_path_change: Option<FinalPathChange>,
        sync_results: VecDeque<Result<(), DeliverSinkError>>,
    }

    impl Default for Hooks {
        fn default() -> Self {
            Self {
                loaded: false,
                cleanup_failures: Vec::new(),
                post_commit_final_path_change: None,
                sync_results: VecDeque::new(),
            }
        }
    }

    thread_local! {
        static HOOKS: RefCell<Hooks> = RefCell::new(Hooks::default());
    }

    fn with_hooks<T>(f: impl FnOnce(&mut Hooks) -> T) -> T {
        HOOKS.with(|hooks| {
            let mut hooks = hooks.borrow_mut();
            if !hooks.loaded {
                *hooks = load_hooks();
            }
            f(&mut hooks)
        })
    }

    fn load_hooks() -> Hooks {
        Hooks {
            loaded: true,
            cleanup_failures: parse_cleanup_failures(),
            post_commit_final_path_change: parse_final_path_change(),
            sync_results: parse_sync_results(),
        }
    }

    fn parse_cleanup_failures() -> Vec<OsString> {
        env::var(CLEANUP_FAILURES_ENV)
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .filter(|entry| !entry.is_empty())
                    .map(OsString::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_final_path_change() -> Option<FinalPathChange> {
        match env::var(POST_COMMIT_FINAL_ACTION_ENV).ok().as_deref() {
            Some("unlink-final") => Some(FinalPathChange::UnlinkFinalPath),
            Some("replace-final") => Some(FinalPathChange::ReplaceFinalPath),
            _ => None,
        }
    }

    fn parse_sync_results() -> VecDeque<Result<(), DeliverSinkError>> {
        env::var(SYNC_RESULTS_ENV)
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .filter(|entry| !entry.is_empty())
                    .filter_map(parse_sync_result)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_sync_result(token: &str) -> Option<Result<(), DeliverSinkError>> {
        match token {
            "ok" => Some(Ok(())),
            "permission_denied" => Some(Err(DeliverSinkError::Io(io::ErrorKind::PermissionDenied))),
            _ => None,
        }
    }

    pub(super) fn maybe_change_final_path_after_final_sync(
        path: &Path,
    ) -> Result<(), DeliverSinkError> {
        let final_path_change = with_hooks(|hooks| hooks.post_commit_final_path_change.take());

        match final_path_change {
            Some(FinalPathChange::UnlinkFinalPath) => remove_file_if_present(path),
            Some(FinalPathChange::ReplaceFinalPath) => replace_final_path(path),
            None => Ok(()),
        }
    }

    fn remove_file_if_present(path: &Path) -> Result<(), DeliverSinkError> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(DeliverSinkError::Io(error.kind())),
        }
    }

    fn replace_final_path(path: &Path) -> Result<(), DeliverSinkError> {
        let Some(parent) = path.parent() else {
            return Err(DeliverSinkError::MissingParent);
        };

        let replacement = parent.join(".vb-rival-replacement");
        std::fs::write(&replacement, RIVAL_REPLACEMENT_BYTES)
            .map_err(|error| DeliverSinkError::Io(error.kind()))?;
        std::fs::rename(&replacement, path).map_err(|error| DeliverSinkError::Io(error.kind()))
    }

    pub(super) fn should_fail_cleanup(path: &OsStr) -> bool {
        with_hooks(|hooks| {
            if let Some(position) = hooks
                .cleanup_failures
                .iter()
                .position(|candidate| candidate == path)
            {
                let _ = hooks.cleanup_failures.remove(position);
                true
            } else {
                false
            }
        })
    }

    pub(super) fn next_sync_result() -> Option<Result<(), DeliverSinkError>> {
        with_hooks(|hooks| hooks.sync_results.pop_front())
    }
}

#[cfg(test)]
mod test_support {
    #![allow(
        clippy::absurd_extreme_comparisons,
        clippy::approx_constant,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::bool_comparison,
        clippy::borrow_deref_ref,
        clippy::cast_abs_to_unsigned,
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::clone_on_copy,
        clippy::cloned_ref_to_slice_refs,
        clippy::cmp_owned,
        clippy::collapsible_if,
        clippy::collapsible_match,
        clippy::const_is_empty,
        clippy::derivable_impls,
        clippy::duplicated_attributes,
        clippy::enum_variant_names,
        clippy::err_expect,
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::explicit_counter_loop,
        clippy::field_reassign_with_default,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::get_first,
        clippy::identity_op,
        clippy::if_let_mutex,
        clippy::if_not_else,
        clippy::if_same_then_else,
        clippy::implicit_clone,
        clippy::implicit_saturating_sub,
        clippy::inconsistent_struct_constructor,
        clippy::indexing_slicing,
        clippy::inefficient_to_string,
        clippy::io_other_error,
        clippy::items_after_test_module,
        clippy::iter_count,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_futures,
        clippy::large_stack_arrays,
        clippy::large_types_passed_by_value,
        clippy::len_zero,
        clippy::let_and_return,
        clippy::let_underscore_must_use,
        clippy::manual_contains,
        clippy::manual_div_ceil,
        clippy::manual_let_else,
        clippy::manual_map,
        clippy::manual_range_contains,
        clippy::manual_saturating_arithmetic,
        clippy::manual_strip,
        clippy::manual_unwrap_or,
        clippy::manual_unwrap_or_default,
        clippy::map_clone,
        clippy::map_flatten,
        clippy::match_like_matches_macro,
        clippy::misnamed_getters,
        clippy::missing_safety_doc,
        clippy::module_inception,
        clippy::multiple_bound_locations,
        clippy::mutable_key_type,
        clippy::needless_bool,
        clippy::needless_bool_assign,
        clippy::needless_borrow,
        clippy::needless_borrows_for_generic_args,
        clippy::needless_collect,
        clippy::needless_pass_by_value,
        clippy::needless_range_loop,
        clippy::needless_return,
        clippy::needless_update,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::new_without_default,
        clippy::nonminimal_bool,
        clippy::ok_expect,
        clippy::option_as_ref_cloned,
        clippy::option_as_ref_deref,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::redundant_clone,
        clippy::redundant_closure,
        clippy::redundant_else,
        clippy::redundant_field_names,
        clippy::redundant_guards,
        clippy::redundant_locals,
        clippy::redundant_pattern_matching,
        clippy::redundant_pub_crate,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::shadow_unrelated,
        clippy::similar_names,
        clippy::single_match,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::todo,
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::trivially_copy_pass_by_ref,
        clippy::type_complexity,
        clippy::unimplemented,
        clippy::uninlined_format_args,
        clippy::unnecessary_cast,
        clippy::unnecessary_fallible_conversions,
        clippy::unnecessary_map_or,
        clippy::unnecessary_mut_passed,
        clippy::unnecessary_sort_by,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_wraps,
        clippy::unneeded_struct_pattern,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_async,
        clippy::unused_io_amount,
        clippy::unused_self,
        clippy::unused_trait_names,
        clippy::unwrap_used,
        clippy::useless_asref,
        clippy::useless_conversion,
        clippy::useless_format,
        clippy::useless_vec,
        clippy::vec_init_then_push,
        clippy::wildcard_enum_match_arm,
        clippy::wildcard_imports,
        dead_code,
        let_underscore_drop,
        unused_imports,
        unused_variables
    )]
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::ok_expect,
        clippy::as_conversions,
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::let_underscore_must_use,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::todo,
        clippy::unimplemented,
        clippy::assertions_on_constants,
        clippy::needless_range_loop,
        clippy::bool_assert_comparison,
        clippy::approx_constant,
        clippy::field_reassign_with_default,
        clippy::redundant_guards,
        clippy::redundant_closure,
        clippy::useless_conversion,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_cast,
        clippy::needless_update,
        clippy::bool_comparison,
        clippy::manual_div_ceil,
        clippy::clone_on_copy,
        clippy::len_zero,
        clippy::redundant_clone,
        clippy::collapsible_if,
        clippy::needless_return,
        clippy::needless_borrow,
        clippy::useless_format,
        clippy::redundant_pub_crate,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::missing_safety_doc,
        clippy::wildcard_enum_match_arm,
        clippy::large_futures,
        clippy::unused_async,
        clippy::unused_self,
        let_underscore_drop,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::if_not_else,
        clippy::implicit_clone,
        clippy::inefficient_to_string,
        clippy::inconsistent_struct_constructor,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_types_passed_by_value,
        clippy::let_and_return,
        clippy::misnamed_getters,
        clippy::mutable_key_type,
        clippy::needless_collect,
        clippy::nonminimal_bool,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::trivially_copy_pass_by_ref,
        clippy::uninlined_format_args,
        clippy::unnecessary_wraps,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_io_amount,
        clippy::unused_trait_names,
        clippy::vec_init_then_push,
        clippy::wildcard_imports,
        clippy::absurd_extreme_comparisons,
        clippy::expect_fun_call,
        clippy::useless_vec,
        clippy::redundant_locals,
        clippy::too_many_lines,
        clippy::cast_lossless,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        clippy::cast_abs_to_unsigned,
        clippy::similar_names,
        clippy::shadow_unrelated,
        clippy::needless_pass_by_value,
        unused_imports,
        dead_code,
        unused_variables
    )]

    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::{Path, PathBuf};

    use super::DeliverSinkError;

    #[derive(Default)]
    struct Hooks {
        cleanup_failures: Vec<OsString>,
        parent_change: Option<ParentChange>,
        before_link_parent_change: Option<PostCommitParentChange>,
        after_link_sync_parent_change: Option<PostCommitParentChange>,
        post_commit_parent_change: Option<PostCommitParentChange>,
        post_commit_final_path_change: Option<FinalPathChange>,
        sync_results: VecDeque<Result<(), DeliverSinkError>>,
    }

    pub(super) enum ParentChange {
        ReplaceOpenedPathWithNewDirectory { moved_to: PathBuf },
    }

    pub(super) enum PostCommitParentChange {
        #[cfg(unix)]
        ReplaceResolvedPathWithSymlink {
            moved_to: PathBuf,
            replacement: PathBuf,
        },
    }

    pub(super) enum FinalPathChange {
        UnlinkFinalPath,
        ReplaceFinalPath,
    }

    #[derive(Default)]
    pub(super) struct HookConfig {
        pub(super) cleanup_failures: Vec<OsString>,
        pub(super) parent_change: Option<ParentChange>,
        pub(super) before_link_parent_change: Option<PostCommitParentChange>,
        pub(super) after_link_sync_parent_change: Option<PostCommitParentChange>,
        pub(super) post_commit_parent_change: Option<PostCommitParentChange>,
        pub(super) post_commit_final_path_change: Option<FinalPathChange>,
        pub(super) sync_results: VecDeque<Result<(), DeliverSinkError>>,
    }

    pub(super) struct InstalledHooks;

    thread_local! {
        static HOOKS: RefCell<Hooks> = RefCell::new(Hooks::default());
    }

    fn with_hooks<T>(f: impl FnOnce(&mut Hooks) -> T) -> T {
        HOOKS.with(|hooks| {
            let mut hooks = hooks.borrow_mut();
            f(&mut hooks)
        })
    }

    pub(super) fn install(config: HookConfig) -> InstalledHooks {
        with_hooks(|hooks| {
            *hooks = Hooks {
                cleanup_failures: config.cleanup_failures,
                parent_change: config.parent_change,
                before_link_parent_change: config.before_link_parent_change,
                after_link_sync_parent_change: config.after_link_sync_parent_change,
                post_commit_parent_change: config.post_commit_parent_change,
                post_commit_final_path_change: config.post_commit_final_path_change,
                sync_results: config.sync_results,
            };
        });
        InstalledHooks
    }

    impl Drop for InstalledHooks {
        fn drop(&mut self) {
            with_hooks(|hooks| {
                *hooks = Hooks::default();
            });
        }
    }

    pub(super) fn maybe_change_parent_path(parent: &Path) -> Result<(), io::Error> {
        let parent_change = with_hooks(|hooks| hooks.parent_change.take());

        if let Some(ParentChange::ReplaceOpenedPathWithNewDirectory { moved_to }) = parent_change {
            std::fs::rename(parent, &moved_to)?;
            std::fs::create_dir(parent)?;
        }
        Ok(())
    }

    pub(super) fn maybe_change_parent_path_before_link(parent: &Path) -> Result<(), io::Error> {
        let parent_change = with_hooks(|hooks| hooks.before_link_parent_change.take());
        apply_resolved_parent_swap(parent, parent_change)
    }

    pub(super) fn maybe_change_parent_path_after_link_sync(parent: &Path) -> Result<(), io::Error> {
        let parent_change = with_hooks(|hooks| hooks.after_link_sync_parent_change.take());
        apply_resolved_parent_swap(parent, parent_change)
    }

    pub(super) fn maybe_change_parent_path_after_final_sync(
        parent: &Path,
    ) -> Result<(), io::Error> {
        let post_commit_parent_change = with_hooks(|hooks| hooks.post_commit_parent_change.take());

        apply_resolved_parent_swap(parent, post_commit_parent_change)
    }

    pub(super) fn maybe_change_final_path_after_final_sync(path: &Path) -> Result<(), io::Error> {
        let final_path_change = with_hooks(|hooks| hooks.post_commit_final_path_change.take());
        apply_final_path_change(path, final_path_change)
    }

    fn apply_resolved_parent_swap(
        parent: &Path,
        parent_change: Option<PostCommitParentChange>,
    ) -> Result<(), io::Error> {
        #[cfg(unix)]
        if let Some(PostCommitParentChange::ReplaceResolvedPathWithSymlink {
            moved_to,
            replacement,
        }) = parent_change
        {
            std::fs::rename(parent, &moved_to)?;
            std::os::unix::fs::symlink(&replacement, parent)?;
        }
        Ok(())
    }

    fn apply_final_path_change(
        path: &Path,
        final_path_change: Option<FinalPathChange>,
    ) -> Result<(), io::Error> {
        match final_path_change {
            Some(FinalPathChange::UnlinkFinalPath) => {
                std::fs::remove_file(path)?;
            }
            Some(FinalPathChange::ReplaceFinalPath) => {
                let parent = path.parent().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("missing parent for test final path {}", path.display()),
                    )
                })?;
                let replacement = parent.join(".vb-rival-replacement");
                std::fs::write(&replacement, b"rival replacement\n")?;
                std::fs::rename(&replacement, path)?;
            }
            None => {}
        }
        Ok(())
    }

    pub(super) fn should_fail_cleanup(path: &OsStr) -> bool {
        with_hooks(|hooks| {
            if let Some(position) = hooks
                .cleanup_failures
                .iter()
                .position(|candidate| candidate == path)
            {
                let _ = hooks.cleanup_failures.remove(position);
                true
            } else {
                false
            }
        })
    }

    pub(super) fn next_sync_result() -> Option<Result<(), DeliverSinkError>> {
        with_hooks(|hooks| hooks.sync_results.pop_front())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::absurd_extreme_comparisons,
        clippy::approx_constant,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::bool_comparison,
        clippy::borrow_deref_ref,
        clippy::cast_abs_to_unsigned,
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::clone_on_copy,
        clippy::cloned_ref_to_slice_refs,
        clippy::cmp_owned,
        clippy::collapsible_if,
        clippy::collapsible_match,
        clippy::const_is_empty,
        clippy::derivable_impls,
        clippy::duplicated_attributes,
        clippy::enum_variant_names,
        clippy::err_expect,
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::explicit_counter_loop,
        clippy::field_reassign_with_default,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::get_first,
        clippy::identity_op,
        clippy::if_let_mutex,
        clippy::if_not_else,
        clippy::if_same_then_else,
        clippy::implicit_clone,
        clippy::implicit_saturating_sub,
        clippy::inconsistent_struct_constructor,
        clippy::indexing_slicing,
        clippy::inefficient_to_string,
        clippy::io_other_error,
        clippy::items_after_test_module,
        clippy::iter_count,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_futures,
        clippy::large_stack_arrays,
        clippy::large_types_passed_by_value,
        clippy::len_zero,
        clippy::let_and_return,
        clippy::let_underscore_must_use,
        clippy::manual_contains,
        clippy::manual_div_ceil,
        clippy::manual_let_else,
        clippy::manual_map,
        clippy::manual_range_contains,
        clippy::manual_saturating_arithmetic,
        clippy::manual_strip,
        clippy::manual_unwrap_or,
        clippy::manual_unwrap_or_default,
        clippy::map_clone,
        clippy::map_flatten,
        clippy::match_like_matches_macro,
        clippy::misnamed_getters,
        clippy::missing_safety_doc,
        clippy::module_inception,
        clippy::multiple_bound_locations,
        clippy::mutable_key_type,
        clippy::needless_bool,
        clippy::needless_bool_assign,
        clippy::needless_borrow,
        clippy::needless_borrows_for_generic_args,
        clippy::needless_collect,
        clippy::needless_pass_by_value,
        clippy::needless_range_loop,
        clippy::needless_return,
        clippy::needless_update,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::new_without_default,
        clippy::nonminimal_bool,
        clippy::ok_expect,
        clippy::option_as_ref_cloned,
        clippy::option_as_ref_deref,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::redundant_clone,
        clippy::redundant_closure,
        clippy::redundant_else,
        clippy::redundant_field_names,
        clippy::redundant_guards,
        clippy::redundant_locals,
        clippy::redundant_pattern_matching,
        clippy::redundant_pub_crate,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::shadow_unrelated,
        clippy::similar_names,
        clippy::single_match,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::todo,
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::trivially_copy_pass_by_ref,
        clippy::type_complexity,
        clippy::unimplemented,
        clippy::uninlined_format_args,
        clippy::unnecessary_cast,
        clippy::unnecessary_fallible_conversions,
        clippy::unnecessary_map_or,
        clippy::unnecessary_mut_passed,
        clippy::unnecessary_sort_by,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_wraps,
        clippy::unneeded_struct_pattern,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_async,
        clippy::unused_io_amount,
        clippy::unused_self,
        clippy::unused_trait_names,
        clippy::unwrap_used,
        clippy::useless_asref,
        clippy::useless_conversion,
        clippy::useless_format,
        clippy::useless_vec,
        clippy::vec_init_then_push,
        clippy::wildcard_enum_match_arm,
        clippy::wildcard_imports,
        dead_code,
        let_underscore_drop,
        unused_imports,
        unused_variables
    )]
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::ok_expect,
        clippy::as_conversions,
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::let_underscore_must_use,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::todo,
        clippy::unimplemented,
        clippy::assertions_on_constants,
        clippy::needless_range_loop,
        clippy::bool_assert_comparison,
        clippy::approx_constant,
        clippy::field_reassign_with_default,
        clippy::redundant_guards,
        clippy::redundant_closure,
        clippy::useless_conversion,
        clippy::unnecessary_unwrap,
        clippy::unnecessary_cast,
        clippy::needless_update,
        clippy::bool_comparison,
        clippy::manual_div_ceil,
        clippy::clone_on_copy,
        clippy::len_zero,
        clippy::redundant_clone,
        clippy::collapsible_if,
        clippy::needless_return,
        clippy::needless_borrow,
        clippy::useless_format,
        clippy::redundant_pub_crate,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::missing_safety_doc,
        clippy::wildcard_enum_match_arm,
        clippy::large_futures,
        clippy::unused_async,
        clippy::unused_self,
        let_underscore_drop,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::if_not_else,
        clippy::implicit_clone,
        clippy::inefficient_to_string,
        clippy::inconsistent_struct_constructor,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_types_passed_by_value,
        clippy::let_and_return,
        clippy::misnamed_getters,
        clippy::mutable_key_type,
        clippy::needless_collect,
        clippy::nonminimal_bool,
        clippy::option_if_let_else,
        clippy::or_fun_call,
        clippy::path_buf_push_overwrite,
        clippy::print_stderr,
        clippy::print_stdout,
        clippy::pub_with_shorthand,
        clippy::range_minus_one,
        clippy::range_plus_one,
        clippy::ref_binding_to_reference,
        clippy::ref_option_ref,
        clippy::single_match_else,
        clippy::suspicious_operation_groupings,
        clippy::trivially_copy_pass_by_ref,
        clippy::uninlined_format_args,
        clippy::unnecessary_wraps,
        clippy::unnested_or_patterns,
        clippy::unreadable_literal,
        clippy::unused_io_amount,
        clippy::unused_trait_names,
        clippy::vec_init_then_push,
        clippy::wildcard_imports,
        clippy::absurd_extreme_comparisons,
        clippy::expect_fun_call,
        clippy::useless_vec,
        clippy::redundant_locals,
        clippy::too_many_lines,
        clippy::cast_lossless,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        clippy::cast_abs_to_unsigned,
        clippy::similar_names,
        clippy::shadow_unrelated,
        clippy::needless_pass_by_value,
        unused_imports,
        dead_code,
        unused_variables
    )]

    use std::collections::VecDeque;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{
        DeliverSinkError, DeliverTarget, hashed_temp_name, parse_deliver_target,
        preferred_temp_name, temp_stage_name, test_support, write_json_line,
    };

    use super::test_support::{FinalPathChange, HookConfig, ParentChange, PostCommitParentChange};

    #[cfg(unix)]
    #[test]
    fn parse_deliver_target_resolves_parent_symlink_before_storing_new_file_path()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-parse-symlink-")?;
        let real_parent = temp_dir.path().join("real-parent");
        std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
        let alias_parent = temp_dir.path().join("alias-parent");
        std::os::unix::fs::symlink(&real_parent, &alias_parent)
            .map_err(|error| error.to_string())?;

        let requested_path = alias_parent.join("agent-context.jsonl");
        let target = format!("file:{}", path_text(&requested_path)?);

        match parse_deliver_target(&target).map_err(|error| error.to_string())? {
            DeliverTarget::NewFile(target) => {
                let expected = std::fs::canonicalize(&real_parent)
                    .map_err(|error| error.to_string())?
                    .join("agent-context.jsonl");
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
    fn write_json_line_reports_parent_changed_when_parent_path_swaps_before_write()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-parent-swap-")?;
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

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::ParentChanged) => {}
            Err(error) => return Err(format!("expected ParentChanged, got {error}")),
            Ok(()) => {
                return Err(String::from(
                    "expected ParentChanged after parent path swap",
                ));
            }
        }

        assert_directory_entries_exact(&moved_parent, &[])?;
        assert_directory_entries_exact(&real_parent, &[])
    }

    #[test]
    fn write_json_line_reports_existing_file_when_rival_created_after_parse_and_cleans_stage()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-rival-file-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        std::fs::write(&deliver_path, "rival file\n").map_err(|error| error.to_string())?;

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::ExistingFile) => {}
            Err(error) => return Err(format!("expected ExistingFile race error, got {error}")),
            Ok(()) => return Err(String::from("expected ExistingFile race error")),
        }

        let rival_contents =
            std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?;
        if rival_contents != String::from("rival file\n") {
            return Err(format!(
                "expected rival file contents to remain unchanged, got {rival_contents:?}"
            ));
        }

        assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
        assert_no_stage_name_exists(&deliver_path)
    }

    #[test]
    fn write_json_line_surfaces_existing_file_after_linkat_exist_when_temp_unlink_also_fails()
    -> Result<(), String> {
        // Covers the cleanup-failure-after-linkat-EXIST branch:
        // `linkat(parent, temp, parent, final)` returns EXIST because the
        // rival pre-created the final file, then `cleanup_unpublished_temp_file`
        // is forced to fail its unlinkat via the test hook. The contract is
        // that the *original* semantic error (`ExistingFile`) is surfaced
        // rather than the unlinkat error.
        let temp_dir = repo_tempdir("vb-deliver-rival-file-cleanup-fail-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        std::fs::write(&deliver_path, "rival file\n").map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            cleanup_failures: vec![OsString::from(".agent-context.jsonl.tmp")],
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::ExistingFile) => {}
            Err(error) => {
                return Err(format!(
                    "expected ExistingFile surfaced after cleanup failure, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected ExistingFile surfaced after cleanup failure",
                ));
            }
        }

        let rival_contents =
            std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?;
        if rival_contents != String::from("rival file\n") {
            return Err(format!(
                "expected rival file contents to remain unchanged, got {rival_contents:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn write_json_line_reports_staging_unavailable_when_all_stage_names_are_taken()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-stage-exhaust-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        occupy_all_stage_names(&deliver_path)?;
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::StagingUnavailable) => {}
            Err(error) => {
                return Err(format!(
                    "expected StagingUnavailable after exhausting stage names, got {error}"
                ));
            }
            Ok(()) => return Err(String::from("expected staging-unavailable error")),
        }

        if deliver_path.exists() {
            return Err(format!(
                "expected no delivered file after stage exhaustion, found {}",
                deliver_path.display()
            ));
        }

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn parse_deliver_target_reports_parent_changed_when_parent_inode_changes_during_validation()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-parent-changed-")?;
        let parent = temp_dir.path().join("deliver-parent");
        std::fs::create_dir(&parent).map_err(|error| error.to_string())?;
        let moved_parent = temp_dir.path().join("moved-parent");
        let _hooks = test_support::install(HookConfig {
            parent_change: Some(ParentChange::ReplaceOpenedPathWithNewDirectory {
                moved_to: moved_parent,
            }),
            ..Default::default()
        });

        let deliver_path = parent.join("agent-context.jsonl");
        match parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?)) {
            Err(DeliverSinkError::ParentChanged) => {}
            Err(error) => {
                return Err(format!(
                    "expected ParentChanged during validation, got {error}"
                ));
            }
            Ok(_) => return Err(String::from("expected ParentChanged during validation")),
        }

        assert_directory_entries_exact(temp_dir.path(), &["deliver-parent", "moved-parent"])
    }

    #[test]
    fn write_json_line_returns_sync_error_when_rollback_after_parent_sync_failure_is_durable()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-parent-sync-rollback-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
        let _hooks = test_support::install(HookConfig {
            sync_results: VecDeque::from([Err(sync_error), Ok(())]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied)) => {}
            Err(error) => {
                return Err(format!(
                    "expected original parent sync error after durable rollback, got {error}"
                ));
            }
            Ok(()) => return Err(String::from("expected parent sync failure")),
        }

        if deliver_path.exists() {
            return Err(format!(
                "expected durable rollback to remove final path, found {}",
                deliver_path.display()
            ));
        }

        assert_directory_entries_exact(temp_dir.path(), &[])?;
        assert_no_stage_name_exists(&deliver_path)
    }

    #[cfg(unix)]
    #[test]
    fn write_json_line_reports_parent_changed_when_parent_path_swaps_after_staging_before_linkat()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-pre-link-parent-swap-")?;
        let real_parent = temp_dir.path().join("real-parent");
        let replacement_parent = temp_dir.path().join("replacement-parent");
        let moved_parent = temp_dir.path().join("moved-parent");
        std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
        std::fs::create_dir(&replacement_parent).map_err(|error| error.to_string())?;

        let deliver_path = real_parent.join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            before_link_parent_change: Some(
                PostCommitParentChange::ReplaceResolvedPathWithSymlink {
                    moved_to: moved_parent.clone(),
                    replacement: replacement_parent.clone(),
                },
            ),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::ParentChanged) => {}
            Err(error) => return Err(format!("expected ParentChanged, got {error}")),
            Ok(()) => return Err(String::from("expected ParentChanged after pre-link swap")),
        }

        if deliver_path.exists() {
            return Err(format!(
                "expected no delivered file after pre-link swap, found {}",
                deliver_path.display()
            ));
        }

        assert_directory_entries_exact(&moved_parent, &[])?;
        assert_directory_entries_exact(&replacement_parent, &[])?;
        assert_no_stage_name_exists(&moved_parent.join("agent-context.jsonl"))
    }

    #[cfg(unix)]
    #[test]
    fn write_json_line_reports_parent_changed_and_rolls_back_when_parent_path_swaps_after_link_sync_before_temp_cleanup()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-post-link-parent-swap-")?;
        let real_parent = temp_dir.path().join("real-parent");
        let replacement_parent = temp_dir.path().join("replacement-parent");
        let moved_parent = temp_dir.path().join("moved-parent");
        std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
        std::fs::create_dir(&replacement_parent).map_err(|error| error.to_string())?;

        let deliver_path = real_parent.join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            after_link_sync_parent_change: Some(
                PostCommitParentChange::ReplaceResolvedPathWithSymlink {
                    moved_to: moved_parent.clone(),
                    replacement: replacement_parent.clone(),
                },
            ),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::ParentChanged) => {}
            Err(error) => return Err(format!("expected ParentChanged, got {error}")),
            Ok(()) => {
                return Err(String::from(
                    "expected ParentChanged after post-link parent swap",
                ));
            }
        }

        let moved_path = moved_parent.join("agent-context.jsonl");
        if deliver_path.exists() {
            return Err(format!(
                "expected rollback to remove final path after post-link swap, found {}",
                deliver_path.display()
            ));
        }
        if moved_path.exists() {
            return Err(format!(
                "expected rollback to remove moved final path after post-link swap, found {}",
                moved_path.display()
            ));
        }

        assert_directory_entries_exact(&moved_parent, &[])?;
        assert_directory_entries_exact(&replacement_parent, &[])?;
        assert_no_stage_name_exists(&moved_path)
    }

    #[test]
    fn write_json_line_reports_publish_state_unknown_after_parent_sync_failure_when_rollback_is_not_durable()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-parent-sync-failure-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
        let _hooks = test_support::install(HookConfig {
            sync_results: VecDeque::from([Err(sync_error), Err(sync_error)]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after undurable rollback, got {error}"
                ));
            }
            Ok(()) => return Err(String::from("expected parent sync failure")),
        }

        if deliver_path.exists() {
            return Err(format!(
                "expected final file rollback after parent sync failure, found {}",
                deliver_path.display()
            ));
        }

        assert_directory_entries_exact(temp_dir.path(), &[])?;
        assert_no_stage_name_exists(&deliver_path)
    }

    #[test]
    fn write_json_line_reports_publish_state_unknown_when_temp_unlink_fails_after_publish()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-temp-unlink-failure-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            cleanup_failures: vec![OsString::from(".agent-context.jsonl.tmp")],
            sync_results: VecDeque::from([Ok(())]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after temp unlink failure, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected PublishStateUnknown after temp unlink failure",
                ));
            }
        }

        assert_json_line_file_equals(&deliver_path, &serde_json::json!({"kind": "AgentContext"}))?;
        assert_directory_entries_exact(
            temp_dir.path(),
            &[".agent-context.jsonl.tmp", "agent-context.jsonl"],
        )
    }

    #[test]
    fn write_json_line_reports_publish_state_unknown_when_temp_unlink_sync_fails_after_publish()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-temp-unlink-sync-failure-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
        let _hooks = test_support::install(HookConfig {
            sync_results: VecDeque::from([Ok(()), Err(sync_error)]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after temp unlink sync failure, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected PublishStateUnknown after temp unlink sync failure",
                ));
            }
        }

        assert_json_line_file_equals(&deliver_path, &serde_json::json!({"kind": "AgentContext"}))?;
        assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
        assert_no_stage_name_exists(&deliver_path)
    }

    #[test]
    fn write_json_line_reports_publish_state_unknown_when_rival_unlinks_final_path_after_publish()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-post-commit-final-unlink-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            post_commit_final_path_change: Some(FinalPathChange::UnlinkFinalPath),
            sync_results: VecDeque::from([Ok(()), Ok(())]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after rival final unlink, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected PublishStateUnknown after rival final unlink",
                ));
            }
        }

        if deliver_path.exists() {
            return Err(format!(
                "expected rival final unlink to remove {}, but it still exists",
                deliver_path.display()
            ));
        }

        assert_directory_entries_exact(temp_dir.path(), &[])?;
        assert_no_stage_name_exists(&deliver_path)
    }

    #[test]
    fn write_json_line_reports_publish_state_unknown_when_rival_replaces_final_path_after_publish()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-post-commit-final-replace-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            post_commit_final_path_change: Some(FinalPathChange::ReplaceFinalPath),
            sync_results: VecDeque::from([Ok(()), Ok(())]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after rival final replace, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected PublishStateUnknown after rival final replace",
                ));
            }
        }

        let rival_contents =
            std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?;
        if rival_contents != String::from("rival replacement\n") {
            return Err(format!(
                "expected rival replacement to occupy final path, got {rival_contents:?}"
            ));
        }

        assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
        assert_no_stage_name_exists(&deliver_path)
    }

    #[test]
    fn write_json_line_reports_publish_state_unknown_when_rollback_leaves_temp_link()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-temp-rollback-link-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let sync_error = DeliverSinkError::Io(std::io::ErrorKind::PermissionDenied);
        let _hooks = test_support::install(HookConfig {
            cleanup_failures: vec![OsString::from(".agent-context.jsonl.tmp")],
            sync_results: VecDeque::from([Err(sync_error), Ok(())]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after incomplete rollback, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected PublishStateUnknown after incomplete rollback",
                ));
            }
        }

        if deliver_path.exists() {
            return Err(format!(
                "expected rollback to remove final path {}, but it remains",
                deliver_path.display()
            ));
        }

        let temp_stage_path = temp_dir.path().join(".agent-context.jsonl.tmp");
        assert_json_line_file_equals(
            &temp_stage_path,
            &serde_json::json!({"kind": "AgentContext"}),
        )?;
        assert_directory_entries_exact(temp_dir.path(), &[".agent-context.jsonl.tmp"])
    }

    #[cfg(unix)]
    #[test]
    fn write_json_line_preserves_published_file_when_parent_path_swaps_after_final_sync()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-post-commit-parent-swap-")?;
        let real_parent = temp_dir.path().join("real-parent");
        let replacement_parent = temp_dir.path().join("replacement-parent");
        let moved_parent = temp_dir.path().join("moved-parent");
        std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
        std::fs::create_dir(&replacement_parent).map_err(|error| error.to_string())?;

        let deliver_path = real_parent.join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            post_commit_parent_change: Some(
                PostCommitParentChange::ReplaceResolvedPathWithSymlink {
                    moved_to: moved_parent.clone(),
                    replacement: replacement_parent.clone(),
                },
            ),
            sync_results: VecDeque::from([Ok(()), Ok(())]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after post-commit parent swap, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected PublishStateUnknown after post-commit parent swap",
                ));
            }
        }

        let moved_path = moved_parent.join("agent-context.jsonl");
        assert_json_line_file_equals(&moved_path, &serde_json::json!({"kind": "AgentContext"}))?;
        if deliver_path.exists() {
            return Err(format!(
                "expected replaced path to stay empty after parent swap, found {}",
                deliver_path.display()
            ));
        }

        assert_directory_entries_exact(&moved_parent, &["agent-context.jsonl"])?;
        assert_directory_entries_exact(&replacement_parent, &[])?;
        assert_no_stage_name_exists(&moved_path)
    }

    #[test]
    fn write_json_line_reports_publish_state_unknown_when_final_cleanup_fails_after_parent_sync_failure()
    -> Result<(), String> {
        let temp_dir = repo_tempdir("vb-deliver-parent-cleanup-failure-")?;
        let deliver_path = temp_dir.path().join("agent-context.jsonl");
        let target = parse_deliver_target(&format!("file:{}", path_text(&deliver_path)?))
            .map_err(|error| error.to_string())?;
        let _hooks = test_support::install(HookConfig {
            cleanup_failures: vec![OsString::from("agent-context.jsonl")],
            sync_results: VecDeque::from([Err(DeliverSinkError::Io(
                std::io::ErrorKind::PermissionDenied,
            ))]),
            ..Default::default()
        });

        match write_json_line(&target, &serde_json::json!({"kind": "AgentContext"})) {
            Err(DeliverSinkError::PublishStateUnknown) => {}
            Err(error) => {
                return Err(format!(
                    "expected PublishStateUnknown after forced cleanup failure, got {error}"
                ));
            }
            Ok(()) => {
                return Err(String::from(
                    "expected PublishStateUnknown after forced cleanup failure",
                ));
            }
        }

        if !deliver_path.exists() {
            return Err(format!(
                "expected final path to remain after forced cleanup failure, missing {}",
                deliver_path.display()
            ));
        }

        assert_json_line_file_equals(&deliver_path, &serde_json::json!({"kind": "AgentContext"}))?;
        assert_directory_entries_exact(temp_dir.path(), &["agent-context.jsonl"])?;
        assert_no_stage_name_exists(&deliver_path)
    }

    #[test]
    fn created_file_mode_is_owner_only() -> Result<(), String> {
        // Drives the production `openat(... OFlags::CREATE | OFlags::EXCL,
        // MODE)` path on a real file, then `fstat`s the resulting descriptor
        // to confirm the kernel-observed mode is owner-only.
        let temp_dir = repo_tempdir("vb-deliver-mode-")?;
        let parent_fd = rustix::fs::openat(
            rustix::fs::CWD,
            temp_dir.path(),
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
            rustix::fs::Mode::empty(),
        )
        .map_err(|error| error.to_string())?;
        let file_name = std::ffi::OsStr::new("vb-deliver-mode-probe");
        let file = rustix::fs::openat(
            &parent_fd,
            file_name,
            rustix::fs::OFlags::WRONLY
                | rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::EXCL
                | rustix::fs::OFlags::CLOEXEC,
            super::MODE,
        )
        .map(std::fs::File::from)
        .map_err(|error| error.to_string())?;
        let stat = rustix::fs::fstat(&file).map_err(|error| error.to_string())?;
        let mode = stat.st_mode & 0o777;
        if mode & 0o600 == 0o600 && mode & 0o077 == 0 {
            Ok(())
        } else {
            Err(format!(
                "expected owner-only mode 0o600, got {mode:o} (full mode 0o{:o})",
                stat.st_mode & 0o7777
            ))
        }
    }

    fn path_text(path: &std::path::Path) -> Result<String, String> {
        path.to_str()
            .map(str::to_owned)
            .ok_or_else(|| String::from("test path must be UTF-8"))
    }

    fn repo_temp_root() -> Result<PathBuf, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deliver-sink-tmp");
        std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        std::fs::canonicalize(&root).map_err(|error| error.to_string())
    }

    fn repo_tempdir(prefix: &str) -> Result<tempfile::TempDir, String> {
        let root = repo_temp_root()?;
        tempfile::Builder::new()
            .prefix(prefix)
            .tempdir_in(root)
            .map_err(|error| error.to_string())
    }

    fn exact_json_line_bytes(value: &serde_json::Value) -> Result<Vec<u8>, String> {
        let mut bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    fn assert_json_line_file_equals(
        path: &std::path::Path,
        expected_value: &serde_json::Value,
    ) -> Result<(), String> {
        let actual = std::fs::read(path).map_err(|error| error.to_string())?;
        let expected = exact_json_line_bytes(expected_value)?;
        if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "expected exact JSONL bytes {:?} at {}, got {:?}",
                expected,
                path.display(),
                actual
            ))
        }
    }

    fn assert_directory_entries_exact(directory: &Path, expected: &[&str]) -> Result<(), String> {
        let mut actual_entries = Vec::new();
        for entry_result in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
            let entry = entry_result.map_err(|error| error.to_string())?;
            actual_entries.push(entry.file_name().to_string_lossy().into_owned());
        }
        actual_entries.sort();

        let mut expected_entries = expected
            .iter()
            .map(|entry| String::from(*entry))
            .collect::<Vec<_>>();
        expected_entries.sort();

        if actual_entries == expected_entries {
            Ok(())
        } else {
            Err(format!(
                "expected directory entries {:?} at {}, got {:?}",
                expected_entries,
                directory.display(),
                actual_entries
            ))
        }
    }

    fn occupy_all_stage_names(path: &Path) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| String::from("deliver path is missing parent"))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| String::from("deliver path is missing file name"))?;
        let resolved_parent = std::fs::canonicalize(parent).map_err(|error| error.to_string())?;
        let resolved_path = resolved_parent.join(file_name);
        let base_names = [
            preferred_temp_name(file_name),
            hashed_temp_name(&resolved_path),
            OsString::from(".tmp"),
            OsString::from(".t"),
        ];

        for base_name in base_names {
            for attempt in 0..super::MAX_TEMP_STAGE_ATTEMPTS {
                let candidate = resolved_parent.join(temp_stage_name(&base_name, attempt));
                std::fs::write(&candidate, b"occupied stage\n")
                    .map_err(|error| error.to_string())?;
            }
        }

        Ok(())
    }

    fn assert_no_stage_name_exists(path: &std::path::Path) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| String::from("deliver path is missing parent"))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| String::from("deliver path is missing file name"))?;
        let resolved_parent = std::fs::canonicalize(parent).map_err(|error| error.to_string())?;
        let resolved_path = resolved_parent.join(file_name);
        let base_names = [
            preferred_temp_name(file_name),
            hashed_temp_name(&resolved_path),
            OsString::from(".tmp"),
            OsString::from(".t"),
        ];

        for base_name in base_names {
            for attempt in 0..super::MAX_TEMP_STAGE_ATTEMPTS {
                let candidate = resolved_parent.join(temp_stage_name(&base_name, attempt));
                if candidate.exists() {
                    return Err(format!(
                        "expected no leaked stage file, found {}",
                        candidate.display()
                    ));
                }
            }
        }

        Ok(())
    }
}
