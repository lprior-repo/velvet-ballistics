use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use super::Failure;

pub(super) fn sources(root: &Path) -> Result<Vec<PathBuf>, Failure> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    std::iter::from_fn(|| {
        pending
            .pop()
            .map(|dir| collect_directory(&dir, &mut pending, &mut files))
    })
    .try_for_each(|result| result)?;
    files.sort();
    Ok(files)
}

fn collect_directory(
    dir: &Path,
    pending: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> Result<(), Failure> {
    std::fs::read_dir(dir)?.try_for_each(|entry| collect_entry(entry?, pending, files))
}

fn collect_entry(
    entry: DirEntry,
    pending: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> Result<(), Failure> {
    let file = entry.path();
    if entry.file_type()?.is_dir() {
        pending.push(file.clone());
    }
    if file
        .file_name()
        .is_some_and(|name| name.as_encoded_bytes().ends_with(b".rs"))
    {
        files.push(file);
    }
    Ok(())
}
