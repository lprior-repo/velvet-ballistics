use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;

use rustix::fs::{AtFlags, FileType, Mode, fstat, statat};

// Linux path strings are effectively capped at 4095 bytes because the final
// NUL terminator consumes the last PATH_MAX byte.
pub(crate) const MAX_PATH_BYTES: usize = 4095;
pub(crate) const MAX_TEMP_STAGE_ATTEMPTS: usize = 8;
pub(crate) const MINIMUM_STAGE_BASE_NAME: &str = ".t";
// Bare (no envelope) message returned by `Display` for `PublishStateUnknown`;
// the binary prefixes `deliver failed: ` before writing it to stderr. Exposed
// as `pub` so the integration test can assert against the single source of
// truth instead of duplicating the literal.
pub const PUBLISH_STATE_UNKNOWN_MESSAGE: &str =
    "deliver publish outcome is unknown after post-commit state could not be confirmed";
// Owner-only read+write mode for newly created delivery files. Computed via
// `Mode::RUSR.union(Mode::WUSR)` at compile time so the value avoids a runtime
// call and the per-bit assembly stays in the constant evaluator.
pub(crate) const MODE: Mode = Mode::RUSR.union(Mode::WUSR);

#[derive(Debug)]
pub(crate) enum TempStageCreation {
    Created((OsString, std::fs::File)),
    Exhausted,
    NameTooLong { had_occupied_candidate: bool },
}

#[derive(Clone, Copy)]
pub(crate) struct PublishedPathIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

/// Domain errors for the deliver sink pipeline.
///
/// Every expected failure in the publish lifecycle is represented as an enum
/// variant so callers can match exhaustively and decide whether the published
/// state is known or ambiguous.
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

pub(crate) fn path_is_absent(parent_dir: &std::os::fd::OwnedFd, path: &OsStr) -> bool {
    match statat(parent_dir, path, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(_) => false,
        Err(error) => error == rustix::io::Errno::NOENT,
    }
}

pub(crate) fn published_path_identity_for_file(
    file: &std::fs::File,
) -> Result<PublishedPathIdentity, DeliverSinkError> {
    let stat = fstat(file).map_err(to_rustix_io_error)?;
    Ok(PublishedPathIdentity {
        device: stat.st_dev,
        inode: stat.st_ino,
    })
}

pub(crate) fn to_io_error(error: io::Error) -> DeliverSinkError {
    DeliverSinkError::Io(error.kind())
}

pub(crate) fn to_rustix_io_error(error: rustix::io::Errno) -> DeliverSinkError {
    if error == rustix::io::Errno::NAMETOOLONG {
        DeliverSinkError::OverlongPath
    } else {
        DeliverSinkError::Io(io::Error::from(error).kind())
    }
}
