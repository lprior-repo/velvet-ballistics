use std::fs;

use serde_json::Value;

use super::types::{
    HelperError, ManifestSelection, Operation, ReportedHarnessTotal, RequestedPackage,
    ValidatedJson,
};

pub(super) fn read_json(path: &str) -> Result<Value, HelperError> {
    let bytes = fs::read(path).map_err(|source| HelperError::Read {
        path: path.to_owned(),
        source,
    })?;
    parse_json(&bytes)
}

pub(super) fn parse_json(bytes: &[u8]) -> Result<Value, HelperError> {
    // serde_json intentionally rejects Python's nonstandard NaN and Infinity constants.
    serde_json::from_slice(bytes).map_err(HelperError::Json)
}

fn package_name(package: &Value) -> Result<&str, HelperError> {
    package
        .as_object()
        .ok_or(HelperError::MetadataShape("package entry"))?
        .get("name")
        .and_then(Value::as_str)
        .ok_or(HelperError::MetadataShape("package name"))
}

fn matched_manifest(package: &Value, name: &str) -> Result<String, HelperError> {
    let path = package
        .as_object()
        .and_then(|object| object.get("manifest_path"))
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| HelperError::InvalidManifestPath(name.to_owned()))?;
    Ok(path.to_owned())
}

fn matching_packages(
    packages: &[Value],
    requested: &RequestedPackage,
) -> Result<(usize, Option<String>), HelperError> {
    let (count, path) = packages
        .iter()
        .try_fold((0usize, None), |(count, path), package| {
            let name = package_name(package)?;
            if name != requested.0 {
                return Ok((count, path));
            }
            let next_count = count.checked_add(1).ok_or(HelperError::CountOverflow)?;
            Ok((next_count, Some(matched_manifest(package, name)?)))
        })?;
    Ok((count, path))
}

fn select_package(
    metadata: &Value,
    requested: &RequestedPackage,
) -> Result<ManifestSelection, HelperError> {
    let packages = metadata
        .as_object()
        .and_then(|object| object.get("packages"))
        .and_then(Value::as_array)
        .ok_or(HelperError::MetadataShape("packages array"))?;
    let (count, path) = matching_packages(packages, requested)?;
    if count != 1 {
        return Err(HelperError::PackageMatches {
            requested: requested.0.clone(),
            count,
        });
    }
    path.map(ManifestSelection)
        .ok_or_else(|| HelperError::PackageMatches {
            requested: requested.0.clone(),
            count,
        })
}

fn validate_json(_value: Value) -> ValidatedJson {
    ValidatedJson
}

fn harness_total(listing: &Value) -> Result<ReportedHarnessTotal, HelperError> {
    let root = listing.as_object().ok_or(HelperError::ListingRoot)?;
    let Some(totals) = root.get("totals") else {
        return Ok(ReportedHarnessTotal("0".to_owned()));
    };
    let totals = totals.as_object().ok_or(HelperError::TotalsShape)?;
    let Some(count) = totals.get("standard-harnesses") else {
        return Ok(ReportedHarnessTotal("0".to_owned()));
    };
    match count {
        Value::Number(number) => Ok(ReportedHarnessTotal(number.to_string())),
        Value::String(text) => Ok(ReportedHarnessTotal(text.clone())),
        _ => Err(HelperError::HarnessCountShape),
    }
}

pub(super) fn parse_operation(args: &[String]) -> Result<Operation, HelperError> {
    let operation = args.first().map(String::as_str);
    let file = args.get(1);
    match (operation, file, args.get(2), args.len()) {
        (Some("manifest-path"), Some(file), Some(package), 3) => Ok(Operation::ManifestPath(
            file.clone(),
            RequestedPackage(package.clone()),
        )),
        (Some("validate-json"), Some(file), None, 2) => Ok(Operation::ValidateJson(file.clone())),
        (Some("standard-harnesses"), Some(file), None, 2) => {
            Ok(Operation::StandardHarnesses(file.clone()))
        }
        _ => Err(HelperError::Usage),
    }
}

pub(super) fn execute(operation: Operation) -> Result<Option<String>, HelperError> {
    match operation {
        Operation::ManifestPath(path, requested) => {
            let selected = select_package(&read_json(&path)?, &requested)?;
            Ok(Some(selected.0))
        }
        Operation::ValidateJson(path) => {
            validate_json(read_json(&path)?);
            Ok(None)
        }
        Operation::StandardHarnesses(path) => Ok(Some(harness_total(&read_json(&path)?)?.0)),
    }
}

#[cfg(test)]
mod tests {
    include!("tests.rs");
}
