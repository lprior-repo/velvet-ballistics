use std::fs;
use std::path::{Component, Path};

fn version_core_is_valid(version: &str) -> bool {
    let mut parts = version.splitn(3, '.');
    let (major, minor, patch) = match (parts.next(), parts.next(), parts.next()) {
        (Some(major), Some(minor), Some(patch)) => (major, minor, patch),
        _ => return false,
    };
    !major.is_empty()
        && major.bytes().all(|byte| byte.is_ascii_digit())
        && !minor.is_empty()
        && minor.bytes().all(|byte| byte.is_ascii_digit())
        && patch
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
}

fn crate_identity(directory: &str) -> Option<(&str, &str)> {
    directory
        .match_indices('-')
        .rev()
        .find_map(|(separator, _)| {
            let version_start = separator.checked_add(1)?;
            let name = directory.get(..separator)?;
            let version = directory.get(version_start..)?;
            (!name.is_empty() && version_core_is_valid(version)).then_some((name, version))
        })
}

fn lock_version(line: &str) -> Option<&str> {
    let value = line.strip_prefix("version = \"")?.split_once('"')?.0;
    (!value.is_empty()).then_some(value)
}

fn lock_name_matches(line: &str, name: &str) -> bool {
    line.split("name = \"")
        .skip(1)
        .any(|value| matches!(value.split_once('"'), Some((found, "")) if found == name))
}

fn locked_version<'a>(lock: &'a str, name: &str) -> Option<&'a str> {
    lock.lines()
        .zip(lock.lines().skip(1))
        .find_map(|(name_line, version_line)| {
            if lock_name_matches(name_line, name) {
                lock_version(version_line)
            } else {
                None
            }
        })
}

fn crate_directory<'a>(resolved_path: &'a Path, cargo_home: &Path) -> Result<&'a str, String> {
    let registry = cargo_home.join("registry/src");
    let relative = resolved_path.strip_prefix(&registry).map_err(|_| {
        format!(
            "registry path outside $CARGO_HOME registry: {}",
            resolved_path.display()
        )
    })?;
    let mut components = relative.components();
    match components.next() {
        Some(Component::Normal(_)) => {}
        Some(_) => return Err("malformed registry index path".to_string()),
        None => return Err("registry path missing index directory".to_string()),
    }
    let crate_component = match components.next() {
        Some(Component::Normal(component)) => component,
        Some(_) => return Err("malformed registry crate path".to_string()),
        None => return Err("registry path missing crate directory".to_string()),
    };
    match components.next() {
        Some(Component::Normal(_)) => {}
        Some(_) => return Err("malformed registry source tail".to_string()),
        None => return Err("registry path missing source tail".to_string()),
    }
    crate_component
        .to_str()
        .ok_or_else(|| "non-UTF-8 registry crate directory".to_string())
}

fn check_lock_pin(root: &Path, name: &str, version: &str) -> Option<String> {
    let lock_path = root.join("Cargo.lock");
    let lock = match fs::read_to_string(&lock_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(format!(
                "no Cargo.lock to verify registry pin for {name} {version}"
            ));
        }
        Err(error) => {
            return Some(format!(
                "cannot read Cargo.lock for {name} {version}: {error}"
            ));
        }
    };
    match locked_version(&lock, name) {
        Some(pinned) if pinned == version => None,
        Some(pinned) => Some(format!(
            "registry cite pins {name} {version} but Cargo.lock pins {pinned}"
        )),
        None => Some(format!(
            "crate {name} not pinned in Cargo.lock (cited {version})"
        )),
    }
}

pub fn registry_pin_error(
    root: &Path,
    _raw_path: &str,
    resolved_path: &Path,
    cargo_home: &Path,
) -> Option<String> {
    let crate_dir = match crate_directory(resolved_path, cargo_home) {
        Ok(directory) => directory,
        Err(error) => return Some(error),
    };
    let (name, version) = match crate_identity(crate_dir) {
        Some(identity) => identity,
        None => return Some(format!("unparsable registry crate directory: {crate_dir}")),
    };
    check_lock_pin(root, name, version)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
