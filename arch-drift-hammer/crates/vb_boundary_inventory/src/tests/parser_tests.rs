//! Parser tests for vb_boundary_inventory
//!
//! Tests: parse_inventory

use crate::boundary_inventory::{BoundaryClass, BoundaryInventoryError, parse_inventory};

// =============================================================================
// parse_inventory tests - valid inputs
// =============================================================================

#[test]
fn parse_inventory_empty_boundaries() {
    let json = r#"{"schema_version": 1, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
    let inventory = result.unwrap();
    assert_eq!(inventory.schema_version, Some(1));
    assert!(inventory.records.is_empty());
}

#[test]
fn parse_inventory_single_boundary() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {
                "id": "vb-y1zq-CAbi-crates/test/src/lib.rs",
                "source_path": "crates/test/src/lib.rs",
                "class": "c_abi"
            }
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
    let inventory = result.unwrap();
    assert_eq!(inventory.records.len(), 1);
    let record = &inventory.records[0];
    assert_eq!(record.id, "vb-y1zq-CAbi-crates/test/src/lib.rs");
    assert_eq!(record.class, BoundaryClass::CAbi);
}

#[test]
fn parse_inventory_all_boundary_classes() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-CAbi-1", "source_path": "crates/test/src/lib.rs", "class": "c_abi"},
            {"id": "vb-y1zq-Ffi-1", "source_path": "fuzz/test.rs", "class": "ffi"},
            {"id": "vb-y1zq-Ipc-1", "source_path": "scripts/run.sh", "class": "ipc"},
            {"id": "vb-y1zq-ExternalBinary-1", "source_path": "crates/bin/src/main.rs", "class": "external_binary"},
            {"id": "vb-y1zq-Decoder-1", "source_path": "crates/decoder/src/lib.rs", "class": "decoder"},
            {"id": "vb-y1zq-GeneratedCode-1", "source_path": "crates/gen/src/lib.rs", "class": "generated_code"},
            {"id": "vb-y1zq-UnsafeAdjacentDependency-1", "source_path": "crates/unsafe_dep/src/lib.rs", "class": "unsafe_adjacent_dependency"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
    let inventory = result.unwrap();
    assert_eq!(inventory.records.len(), 7);
}

#[test]
fn parse_inventory_unknown_class_rejected() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-Unknown-1", "source_path": "crates/test/src/lib.rs", "class": "unknown"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::UnknownBoundaryClass
    );
}

#[test]
fn parse_inventory_multiple_boundaries() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-CAbi-1", "source_path": "crates/test1/src/lib.rs", "class": "c_abi"},
            {"id": "vb-y1zq-Ffi-1", "source_path": "fuzz/test1.rs", "class": "ffi"},
            {"id": "vb-y1zq-Ipc-1", "source_path": "scripts/run1.sh", "class": "ipc"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().records.len(), 3);
}

// =============================================================================
// parse_inventory tests - invalid inputs
// =============================================================================

#[test]
fn parse_inventory_invalid_json() {
    let json = "not valid json {{{";
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_not_an_object() {
    let json = "123";
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_missing_schema_version() {
    let json = r#"{"boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_wrong_schema_version() {
    let json = r#"{"schema_version": 99, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_zero_schema_version() {
    let json = r#"{"schema_version": 0, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_missing_boundaries() {
    let json = r#"{"schema_version": 1}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_boundaries_not_array() {
    let json = r#"{"schema_version": 1, "boundaries": "not an array"}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_boundary_not_object() {
    let json = r#"{"schema_version": 1, "boundaries": ["not an object"]}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_missing_id() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"source_path": "crates/test/src/lib.rs", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_missing_source_path() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-CAbi-test", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_empty_source_path() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-CAbi-test", "source_path": "", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_missing_class() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-test", "source_path": "crates/test/src/lib.rs"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_invalid_class() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-test", "source_path": "crates/test/src/lib.rs", "class": "not_a_class"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_whitespace_id() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "   ", "source_path": "crates/test/src/lib.rs", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok()); // Whitespace is allowed in ID
}

#[test]
fn parse_inventory_utf8_id() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-CAbi-日本語", "source_path": "crates/test/src/lib.rs", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
}

#[test]
fn parse_inventory_large_boundaries_array() {
    let mut json = String::from(r#"{"schema_version": 1, "boundaries": ["#);
    for i in 0..100 {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"id": "vb-y1zq-CAbi-{}", "source_path": "crates/test{}/src/lib.rs", "class": "c_abi"}}"#,
            i, i
        ));
    }
    json.push_str("]}");
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().records.len(), 100);
}

// =============================================================================
// parse_inventory tests — empty and minimal inputs
// =============================================================================

#[test]
fn parse_inventory_empty_bytes_rejected() {
    let result = parse_inventory(b"");
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_whitespace_only_json() {
    let json = "   ";
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_empty_object() {
    let json = "{}";
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

// =============================================================================
// parse_inventory tests — schema version edge cases
// =============================================================================

#[test]
fn parse_inventory_schema_version_as_string_rejected() {
    let json = r#"{"schema_version": "1", "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_schema_version_as_float_rejected() {
    let json = r#"{"schema_version": 1.0, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_schema_version_negative_rejected() {
    let json = r#"{"schema_version": -1, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    // Negative number can't convert to u64 via as_u64
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_schema_version_as_null_rejected() {
    let json = r#"{"schema_version": null, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_schema_version_as_bool_rejected() {
    let json = r#"{"schema_version": true, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

// =============================================================================
// parse_inventory tests — boundary field edge cases
// =============================================================================

#[test]
fn parse_inventory_null_id_rejected() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": null, "source_path": "crates/test/src/lib.rs", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_numeric_id_handled() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": 12345, "source_path": "crates/test/src/lib.rs", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    // Numeric id is not a string, so required_str fails
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_null_class_rejected() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "test-id", "source_path": "crates/test/src/lib.rs", "class": null}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_numeric_class_rejected() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "test-id", "source_path": "crates/test/src/lib.rs", "class": 42}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_bool_class_rejected() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "test-id", "source_path": "crates/test/src/lib.rs", "class": false}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_null_source_path_rejected() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "test-id", "source_path": null, "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_numeric_source_path_rejected() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "test-id", "source_path": 999, "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_source_path_only_whitespace() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "test-id", "source_path": "   ", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    // Whitespace-only source_path is not empty, so parse should succeed
    // But after PathBuf::from, as_os_str().is_empty() is false for whitespace
    assert!(result.is_ok());
}

#[test]
fn parse_inventory_second_boundary_malformed() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {"id": "vb-y1zq-CAbi-1", "source_path": "crates/test/src/lib.rs", "class": "c_abi"},
            {"id": "vb-y1zq-test", "source_path": "", "class": "c_abi"}
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::InventoryParseFailure
    );
}

#[test]
fn parse_inventory_valid_record_with_extra_fields() {
    let json = r#"{
        "schema_version": 1,
        "boundaries": [
            {
                "id": "vb-y1zq-CAbi-test",
                "source_path": "crates/test/src/lib.rs",
                "class": "c_abi",
                "extra_field": "ignored",
                "another_extra": 42
            }
        ]
    }"#;
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().records.len(), 1);
}

#[test]
fn parse_inventory_very_long_id_and_path() {
    let long_id = "vb-y1zq-CAbi-".to_string() + &"a".repeat(500);
    let long_path = "crates/".to_string() + &"a".repeat(500) + "/src/lib.rs";
    let json = format!(
        r#"{{"schema_version": 1, "boundaries": [{{"id": "{}", "source_path": "{}", "class": "c_abi"}}]}}"#,
        long_id, long_path
    );
    let result = parse_inventory(json.as_bytes());
    assert!(result.is_ok());
    let inventory = result.unwrap();
    assert_eq!(inventory.records.len(), 1);
    assert_eq!(inventory.records[0].id, long_id);
    assert_eq!(
        inventory.records[0].source_path,
        std::path::PathBuf::from(long_path)
    );
}

#[test]
fn parse_inventory_schema_version_2_rejected() {
    let json = r#"{"schema_version": 2, "boundaries": []}"#;
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}

#[test]
fn parse_inventory_schema_version_u32_max_rejected() {
    let json = format!(r#"{{"schema_version": {}, "boundaries": []}}"#, u32::MAX);
    let result = parse_inventory(json.as_bytes());
    assert_eq!(
        result.unwrap_err(),
        BoundaryInventoryError::SchemaVersionUnsupported
    );
}
