use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(crate) fn configured_prebuilt_cli() -> Option<OsString> {
    std::env::var_os("CARGO_BIN_EXE_velvet-ballistics")
}

pub(crate) fn resolve_prebuilt_cli(path: OsString, root: &Path) -> PathBuf {
    let path_buf = PathBuf::from(path);
    if path_buf.is_absolute() {
        path_buf
    } else {
        root.join(path_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;

    const ROOT: &str = "/owned/workspace";

    #[test]
    fn resolve_bare_relative_name() {
        let root = Path::new(ROOT);
        let result = resolve_prebuilt_cli(OsString::from("cli"), root);
        assert_eq!(result, root.join("cli"));
    }

    #[test]
    fn resolve_relative_with_parent() {
        let root = Path::new(ROOT);
        let result = resolve_prebuilt_cli(OsString::from("../configured-cli"), root);
        assert_eq!(result, root.join("../configured-cli"));
    }

    #[test]
    fn resolve_absolute_path_unchanged() {
        let root = Path::new(ROOT);
        let result = resolve_prebuilt_cli(OsString::from("/absolute/missing-cli"), root);
        assert_eq!(result, Path::new("/absolute/missing-cli"));
    }

    #[test]
    fn resolve_empty_string() {
        let root = Path::new(ROOT);
        let result = resolve_prebuilt_cli(OsString::from(""), root);
        assert_eq!(result, root);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_non_utf8_relative() {
        let root = Path::new(ROOT);
        let path = OsString::from_vec(vec![b'c', 255]);
        let result = resolve_prebuilt_cli(path, root);
        let expected = root.join(OsString::from_vec(vec![b'c', 255]));
        assert_eq!(result, expected);
    }
}
