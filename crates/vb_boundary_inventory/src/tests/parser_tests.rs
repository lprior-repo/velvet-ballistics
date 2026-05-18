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
