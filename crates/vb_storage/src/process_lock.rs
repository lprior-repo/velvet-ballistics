#![forbid(unsafe_code)]
//! Exclusive process lock using POSIX flock.
//!
//! Prevents dual writers from corrupting the Fjall LSM-tree by acquiring an
//! exclusive advisory lock on `<db_path>/.process.lock`.  The lock is released
//! automatically when the file descriptor is closed (on Drop or process exit).

use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use rustix::fs::FlockOperation;

use crate::error::JournalError;

/// Name of the lock file placed inside the database directory.
const LOCK_FILENAME: &str = ".process.lock";

/// Exclusive flock guard.  Dropping this struct closes the underlying file
/// descriptor, which releases the lock automatically.
pub(crate) struct ProcessLock {
    _file: File,
    _path: PathBuf,
}

impl ProcessLock {
    /// Acquires an exclusive flock on `<db_path>/.process.lock`.
    ///
    /// On contention (EWOULDBLOCK) returns `JournalError::ProcessLockHeld`
    /// with the holder PID if it can be read from the lock file.
    /// On other I/O failures returns `JournalError::ProcessLockIo`.
    pub(crate) fn acquire(db_path: &Path) -> Result<Self, JournalError> {
        let lock_path = db_path.join(LOCK_FILENAME);

        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| JournalError::ProcessLockIo {
                path: lock_path.clone().into_boxed_path(),
                source,
            })?;

        // Try a non-blocking exclusive flock first so we can provide a
        // descriptive error on contention.
        let exclusive_nb = rustix::fs::flock(&file, FlockOperation::NonBlockingLockExclusive);

        match exclusive_nb {
            Ok(()) => {
                // Write our own PID into the lock file so a later contender
                // can report who holds the lock.
                let pid = std::process::id();
                write_best_effort_holder_pid(&mut file, pid);

                Ok(Self {
                    _file: file,
                    _path: lock_path,
                })
            }
            Err(errno) => {
                // On Linux EWOULDBLOCK == EAGAIN, so just match WOULDBLOCK.
                #[cfg(target_os = "linux")]
                let would_block = matches!(errno, rustix::io::Errno::WOULDBLOCK);
                #[cfg(not(target_os = "linux"))]
                let would_block = matches!(
                    errno,
                    rustix::io::Errno::WOULDBLOCK | rustix::io::Errno::AGAIN
                );

                if would_block {
                    let holder_pid = read_holder_pid(&file);
                    Err(JournalError::ProcessLockHeld {
                        path: lock_path.into_boxed_path(),
                        source: errno,
                        holder_pid,
                    })
                } else {
                    Err(JournalError::ProcessLockIo {
                        path: lock_path.into_boxed_path(),
                        source: errno.into(),
                    })
                }
            }
        }
    }
}

/// Best-effort read of the PID stored in the lock file by the current holder.
fn read_holder_pid(file: &File) -> Option<u32> {
    let mut buf = String::new();
    // Shadow to obtain &mut for Read+Seek traits (File does not impl Copy).
    let mut file = file;
    if file.rewind().is_err() {
        return None;
    }
    if file.read_to_string(&mut buf).is_err() {
        return None;
    }
    buf.trim().parse::<u32>().ok()
}

fn write_best_effort_holder_pid(file: &mut File, pid: u32) {
    // PID metadata is diagnostic only; the flock itself is the authority.
    let _metadata_write_failed = file
        .set_len(0)
        .and_then(|()| write!(file, "{pid}"))
        .is_err();
}
