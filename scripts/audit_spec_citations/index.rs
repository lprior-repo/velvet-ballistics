use std::fs;
use std::path::{Path, PathBuf};

pub fn build(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut index = Vec::new();
    append_matches(root, root, &mut index)?;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for (path, file_type) in read_entries(&directory)? {
            if file_type.is_dir() && !path.file_name().is_some_and(excluded_directory) {
                append_matches(root, &path, &mut index)?;
                pending.push(path);
            }
        }
    }
    Ok(index)
}

fn read_entries(directory: &Path) -> Result<Vec<(PathBuf, fs::FileType)>, String> {
    fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        .map(|entry| {
            let entry = entry.map_err(|error| {
                format!(
                    "cannot read directory entry in {}: {error}",
                    directory.display()
                )
            })?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
            Ok((path, file_type))
        })
        .collect()
}

fn append_matches(root: &Path, directory: &Path, index: &mut Vec<PathBuf>) -> Result<(), String> {
    for (path, _) in read_entries(directory)? {
        if is_rust_match(&path) {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("cannot index {}: {error}", path.display()))?;
            index.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn is_rust_match(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.as_encoded_bytes().ends_with(b".rs"))
}

fn excluded_directory(name: &std::ffi::OsStr) -> bool {
    name == ".git" || name == "target"
}

pub fn discover_specs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let directory = root.join("verification/verus");
    let mut files = Vec::new();
    for (path, _) in read_entries(&directory)? {
        if is_rust_match(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub fn read_lines(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    split_lines(&text)
}

fn split_lines(text: &str) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut characters = text.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if !is_line_ending(character) {
            continue;
        }
        let mut end = index
            .checked_add(character.len_utf8())
            .ok_or_else(|| "line boundary overflow".to_string())?;
        if character == '\r'
            && matches!(characters.peek(), Some((_, '\n')))
            && characters.next().is_some()
        {
            end = end
                .checked_add(1)
                .ok_or_else(|| "line boundary overflow".to_string())?;
        }
        if let Some(line) = text.get(start..index) {
            lines.push(line.to_string());
        }
        start = end;
    }
    if start < text.len()
        && let Some(line) = text.get(start..)
    {
        lines.push(line.to_string());
    }
    Ok(lines)
}

fn is_line_ending(character: char) -> bool {
    matches!(
        character,
        '\n' | '\r'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{001c}'
            | '\u{001d}'
            | '\u{001e}'
            | '\u{0085}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

#[cfg(test)]
mod tests {
    use super::split_lines;

    #[test]
    fn splits_python_line_boundaries_without_trailing_empty_line() -> Result<(), String> {
        assert_eq!(
            split_lines("first\r\nsecond\rthird\u{2028}fourth\n")?,
            vec!["first", "second", "third", "fourth"]
        );
        assert_eq!(split_lines("")?, Vec::<String>::new());
        assert_eq!(split_lines("\n")?, vec![""]);
        Ok(())
    }
}
