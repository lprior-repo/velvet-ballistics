use crate::TestSetupError;
use std::path::PathBuf;

/// An isolated Fjall database that cleans up its temporary directory on drop.
///
/// Field drop order is declaration order (Rust Reference §"Drop"):
///   1. `database` (declared first) is dropped first — releases Fjall file locks.
///   2. `_temp_dir` (declared second) is dropped second — removes the directory.
///
/// Storing the `temp_dir` directly avoids the `mem::forget` leak that the
/// previous `Drop` impl relied on, so AddressSanitizer no longer flags the
/// test fixtures for leaking `PathBuf` allocations.
pub struct TempKeyspace {
    database: fjall::Database,
    _temp_dir: tempfile::TempDir,
}

impl TempKeyspace {
    /// Open a new isolated Fjall database in a temporary directory.
    ///
    /// # Errors
    ///
    /// Returns `TestSetupError::TempDirError` or `TestSetupError::FjallOpenError`
    /// on failure.
    pub fn open() -> Result<Self, TestSetupError> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            TestSetupError::TempDirError(format!("failed to create temp dir: {}", e))
        })?;
        let path: PathBuf = temp_dir.path().to_path_buf();
        let database = fjall::Database::builder(&path).open().map_err(|e| {
            TestSetupError::FjallOpenError(format!("failed to open database: {}", e))
        })?;
        Ok(Self {
            database,
            _temp_dir: temp_dir,
        })
    }

    /// Return the filesystem path of the temporary database.
    pub fn path(&self) -> &std::path::Path {
        self._temp_dir.path()
    }

    /// Access the underlying `fjall::Database`.
    pub fn database(&self) -> &fjall::Database {
        &self.database
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn temp_keyspace_cleanup() {
        let temp = TempKeyspace::open().unwrap();
        let path = temp.path().to_path_buf();
        drop(temp);
        assert!(!path.exists());
    }

    #[test]
    fn temp_keyspace_uniqueness() {
        let mut paths = HashSet::new();
        for _ in 0..100 {
            let temp = TempKeyspace::open().unwrap();
            let path = temp.path().to_path_buf();
            assert!(paths.insert(path));
        }
    }

    #[test]
    fn temp_keyspace_concurrent_uniqueness() {
        use std::thread;

        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let mut paths = HashSet::new();
                    for _ in 0..10 {
                        let temp = TempKeyspace::open().unwrap();
                        let path = temp.path().to_path_buf();
                        assert!(paths.insert(path));
                    }
                    paths
                })
            })
            .collect();

        let mut all_paths = HashSet::new();
        for h in handles {
            let paths = h.join().unwrap();
            for p in paths {
                assert!(all_paths.insert(p));
            }
        }
    }
}
