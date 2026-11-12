use std::path::Path;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn parse_members(text: &str) -> TestResult<Vec<String>> {
    let parsed: toml::Value = toml::from_str(text)?;
    let members = parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
        .ok_or("workspace.members not found or not an array")?;
    Ok(member_values(members)?)
}

fn member_values(values: &[toml::Value]) -> Result<Vec<String>, &'static str> {
    if values.is_empty() {
        return Err("workspace.members is empty after parsing");
    }
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("workspace member is not a string")
        })
        .collect()
}

pub(super) fn read_packages(root: &Path) -> TestResult<Vec<(String, String)>> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let packages = json
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("packages array not found in cargo metadata")?;
    packages
        .iter()
        .map(|package| package_pair(root, package))
        .collect()
}

fn package_fields(package: &serde_json::Value) -> Result<(&str, &str), &'static str> {
    let name = package
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or("package missing name")?;
    let manifest = package
        .get("manifest_path")
        .and_then(serde_json::Value::as_str)
        .ok_or("package missing manifest_path")?;
    Ok((name, manifest))
}

fn package_pair(root: &Path, package: &serde_json::Value) -> TestResult<(String, String)> {
    let (name, manifest) = package_fields(package)?;
    let relative = Path::new(manifest).strip_prefix(root)?;
    let path = relative.to_str().ok_or("manifest path is not UTF-8")?;
    Ok((name.to_owned(), path.to_owned()))
}

#[test]
fn non_string_member_is_not_silently_discarded() {
    assert_eq!(
        member_values(&[toml::Value::Integer(12)]),
        Err("workspace member is not a string")
    );
}

#[test]
fn empty_member_array_is_rejected() {
    assert_eq!(
        member_values(&[]),
        Err("workspace.members is empty after parsing")
    );
}

#[test]
fn missing_package_identity_fields_are_rejected_exactly() {
    assert_eq!(
        package_fields(&serde_json::json!({"manifest_path":"reference/Cargo.toml"})),
        Err("package missing name")
    );
    assert_eq!(
        package_fields(&serde_json::json!({"name":"reference"})),
        Err("package missing manifest_path")
    );
}

#[test]
fn non_string_package_identity_fields_are_rejected_exactly() {
    assert_eq!(
        package_fields(&serde_json::json!({"name":12,"manifest_path":"reference/Cargo.toml"})),
        Err("package missing name")
    );
    assert_eq!(
        package_fields(&serde_json::json!({"name":"reference","manifest_path":12})),
        Err("package missing manifest_path")
    );
}
