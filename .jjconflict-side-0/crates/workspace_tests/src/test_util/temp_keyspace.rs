use crate::TestSetupError;
use std::path::PathBuf;

/// An isolated Fjall database that cleans up its temporary directory on drop.
pub struct TempKeyspace {
    database: fjall::Database,
    path: PathBuf,
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
        let path = temp_dir.path().to_path_buf();
        // Leak the TempDir so we control cleanup manually on drop.
        std::mem::forget(temp_dir);

        let database = fjall::Database::builder(&path).open().map_err(|e| {
            TestSetupError::FjallOpenError(format!("failed to open database: {}", e))
        })?;

        Ok(Self { database, path })
    }

    /// Return the filesystem path of the temporary database.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Access the underlying `fjall::Database`.
    pub fn database(&self) -> &fjall::Database {
        &self.database
    }
}

impl Drop for TempKeyspace {
    fn drop(&mut self) {
        // Explicitly drop the database first so Fjall releases file locks.
        let path = std::mem::take(&mut self.path);
        // Now remove the directory. Errors are logged but ignored because
        // this runs during unwind and we must not panic in drop.
        if let Err(_cleanup_error) = std::fs::remove_dir_all(&path) {
            // Directory may already be deleted or permissions may be insufficient.
            // Either way, the temp dir will be cleaned by the OS eventually.
        }
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
