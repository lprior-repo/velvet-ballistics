use serde_json::json;

use super::super::types::{HelperError, ManifestSelection, Operation, RequestedPackage};
use super::{harness_total, parse_json, parse_operation, read_json, select_package};

fn requested(name: &str) -> RequestedPackage {
    RequestedPackage(name.to_owned())
}

#[test]
fn selects_exact_package_name_and_manifest_path() {
    let metadata = json!({"packages":[
        {"name":"other","manifest_path":"/other/Cargo.toml"},
        {"name":"pkg","manifest_path":"/pkg/Cargo.toml"}
    ]});
    assert!(
        matches!(select_package(&metadata, &requested("pkg")), Ok(ManifestSelection(path)) if path == "/pkg/Cargo.toml")
    );
}

#[test]
fn rejects_zero_and_duplicate_exact_package_matches() {
    let none = json!({"packages":[{"name":"pkg-extra","manifest_path":"/x"}]});
    let twice = json!({"packages":[
        {"name":"pkg","manifest_path":"/one"},
        {"name":"pkg","manifest_path":"/two"}
    ]});
    assert!(matches!(
        select_package(&none, &requested("pkg")),
        Err(HelperError::PackageMatches { count: 0, .. })
    ));
    assert!(matches!(
        select_package(&twice, &requested("pkg")),
        Err(HelperError::PackageMatches { count: 2, .. })
    ));
}

#[test]
fn rejects_missing_or_wrong_metadata_shapes() {
    assert!(matches!(
        select_package(&json!({}), &requested("pkg")),
        Err(HelperError::MetadataShape("packages array"))
    ));
    assert!(matches!(
        select_package(&json!({"packages":{}}), &requested("pkg")),
        Err(HelperError::MetadataShape("packages array"))
    ));
    assert!(matches!(
        select_package(&json!({"packages":[null]}), &requested("pkg")),
        Err(HelperError::MetadataShape("package entry"))
    ));
    assert!(matches!(
        select_package(&json!({"packages":[{}]}), &requested("pkg")),
        Err(HelperError::MetadataShape("package name"))
    ));
    assert!(matches!(
        select_package(&json!({"packages":[{"name":3}]}), &requested("pkg")),
        Err(HelperError::MetadataShape("package name"))
    ));
}

#[test]
fn rejects_matching_package_without_nonempty_manifest_path() {
    let absent = json!({"packages":[{"name":"pkg"}]});
    let empty = json!({"packages":[{"name":"pkg","manifest_path":""}]});
    let wrong_type = json!({"packages":[{"name":"pkg","manifest_path":4}]});
    assert!(matches!(
        select_package(&absent, &requested("pkg")),
        Err(HelperError::InvalidManifestPath(_))
    ));
    assert!(matches!(
        select_package(&empty, &requested("pkg")),
        Err(HelperError::InvalidManifestPath(_))
    ));
    assert!(matches!(
        select_package(&wrong_type, &requested("pkg")),
        Err(HelperError::InvalidManifestPath(_))
    ));
}

#[test]
fn parses_complete_json_roots_and_rejects_trailing_junk_or_invalid_utf8() {
    assert!(matches!(parse_json(b"null"), Ok(value) if value == json!(null)));
    assert!(matches!(parse_json(b"[1,true,\"x\"]"), Ok(value) if value == json!([1,true,"x"])));
    assert!(matches!(
        parse_json(b"{} trailing"),
        Err(HelperError::Json(_))
    ));
    assert!(matches!(parse_json(&[0xff]), Err(HelperError::Json(_))));
}

#[test]
fn reports_unreadable_input_with_its_path() {
    let path = std::env::temp_dir()
        .join(format!("kani-json-helper-{}", std::process::id()))
        .join("missing.json");
    let path_text = path.to_string_lossy().into_owned();
    assert!(
        matches!(read_json(&path_text), Err(HelperError::Read { path, .. }) if path == path_text)
    );
}

#[test]
fn reports_missing_zero_numeric_and_text_harness_counts() {
    assert_total(&json!({}), "0");
    assert_total(&json!({"totals":{}}), "0");
    assert_total(&json!({"totals":{"standard-harnesses":0}}), "0");
    assert_total(&json!({"totals":{"standard-harnesses":7}}), "7");
    assert_total(&json!({"totals":{"standard-harnesses":"07"}}), "07");
}

fn assert_total(value: &serde_json::Value, expected: &str) {
    assert!(matches!(harness_total(value), Ok(total) if total.0 == expected));
}

#[test]
fn rejects_nonobject_totals_and_non_scalar_counts() {
    assert!(matches!(
        harness_total(&json!([])),
        Err(HelperError::ListingRoot)
    ));
    assert!(matches!(
        harness_total(&json!({"totals":[]})),
        Err(HelperError::TotalsShape)
    ));
    for count in [json!(true), json!(null), json!([]), json!({})] {
        assert!(matches!(
            harness_total(&json!({"totals":{"standard-harnesses":count}})),
            Err(HelperError::HarnessCountShape)
        ));
    }
}

#[test]
fn accepts_only_the_three_declared_operation_shapes() {
    assert!(matches!(
        parse_operation(&[
            "manifest-path".to_owned(),
            "m.json".to_owned(),
            "pkg".to_owned()
        ]),
        Ok(Operation::ManifestPath(_, _))
    ));
    assert!(matches!(
        parse_operation(&["validate-json".to_owned(), "f.json".to_owned()]),
        Ok(Operation::ValidateJson(_))
    ));
    assert!(matches!(
        parse_operation(&["standard-harnesses".to_owned(), "f.json".to_owned()]),
        Ok(Operation::StandardHarnesses(_))
    ));
    assert!(matches!(parse_operation(&[]), Err(HelperError::Usage)));
    assert!(matches!(
        parse_operation(&["extra".to_owned()]),
        Err(HelperError::Usage)
    ));
}
