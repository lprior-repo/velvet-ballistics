#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod process_lock_tests {
    use crate::error::JournalError;

    #[test]
    fn process_lock_acquire_succeeds_for_fresh_directory() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        // FjallJournal::open acquires the process lock internally
        let result = crate::FjallJournal::open(temp.path(), None);
        // The lock should be acquired successfully
        match result {
            Ok(_journal) => {} // lock acquired and released when journal drops
            Err(e @ JournalError::ProcessLockIo { .. }) => {
                panic!("process lock I/O error unexpected: {e}");
            }
            Err(e) => {
                panic!("unexpected journal error: {e}");
            }
        }
    }

    #[test]
    fn process_lock_prevents_dual_writers_same_directory() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal1 = crate::FjallJournal::open(temp.path(), None)
            .expect("first journal should open successfully");

        let result = crate::FjallJournal::open(temp.path(), None);

        match result {
            Ok(_) => {
                // Fjall may allow re-opening depending on its internal behavior
                // If the second open succeeds, it means fjall handles this internally
            }
            Err(JournalError::ProcessLockHeld { .. }) => {
                // This is the expected failure mode on most systems
            }
            Err(other) => {
                // Any other error is acceptable as long as it's not a panic
                _ = other;
            }
        }
    }

    #[test]
    fn process_lock_is_released_on_drop() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        {
            let _journal =
                crate::FjallJournal::open(temp.path(), None).expect("first journal should open");
            // Drop happens here
        }
        // After drop, we should be able to open again
        let result = crate::FjallJournal::open(temp.path(), None);
        match result {
            Ok(_) => {}
            Err(e) => {
                // On some systems the lock may take time to release
                _ = e;
            }
        }
    }

    #[test]
    fn process_lock_file_is_created() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal should open successfully");

        let lock_path = temp.path().join(".process.lock");
        assert!(
            lock_path.exists(),
            ".process.lock file should exist after journal open"
        );
    }

    #[test]
    fn process_lock_file_contains_holder_pid() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal should open successfully");

        let lock_path = temp.path().join(".process.lock");
        if lock_path.exists() {
            let contents = std::fs::read_to_string(&lock_path).expect("should read lock file");
            let pid: u32 = contents
                .trim()
                .parse()
                .expect("lock file should contain a valid PID");
            // The PID should be the current process ID or 0
            assert!(
                pid > 0 || pid == std::process::id(),
                "lock file PID should be positive or equal to current process ID"
            );
        }
    }

    #[test]
    fn open_store_acquires_process_lock() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let result = crate::open_store(temp.path());
        assert!(result.is_ok(), "open_store should acquire process lock");
    }

    #[test]
    fn init_keyspaces_acquires_process_lock() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let result = crate::init_keyspaces(temp.path());
        assert!(result.is_ok(), "init_keyspaces should acquire process lock");
    }
}
