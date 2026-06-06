use crate::TestSetupError;
use tempfile::TempDir;

/// An isolated Fjall database that cleans up its temporary directory on drop.
pub struct TempKeyspace {
    database: Option<fjall::Database>,
    temp_dir: TempDir,
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

        let database = fjall::Database::builder(&path).open().map_err(|e| {
            TestSetupError::FjallOpenError(format!("failed to open database: {}", e))
        })?;

        Ok(Self {
            database: Some(database),
            temp_dir,
        })
    }

    /// Return the filesystem path of the temporary database.
    pub fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Access the underlying `fjall::Database`.
    pub fn database(&self) -> Option<&fjall::Database> {
        self.database.as_ref()
    }
}

impl Drop for TempKeyspace {
    fn drop(&mut self) {
        // Explicitly drop the database first so Fjall releases file locks.
        let _database = self.database.take();
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
