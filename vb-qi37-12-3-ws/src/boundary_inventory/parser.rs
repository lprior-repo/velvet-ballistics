use std::path::PathBuf;

use super::inventory::BoundaryInventory;
use super::record::{BoundaryRecord, BoundaryRecordParts, FieldState};
use super::types::{BoundaryClass, BoundaryInventoryError, FreshnessMarker};

pub fn parse_inventory(bytes: &[u8]) -> Result<BoundaryInventory, BoundaryInventoryError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_error| BoundaryInventoryError::InventoryParseFailure)?;
    let object = value
        .as_object()
        .ok_or(BoundaryInventoryError::InventoryParseFailure)?;
    let schema_version = parse_schema_version(object.get("schema_version"))?;
    let boundaries = object
        .get("boundaries")
        .and_then(serde_json::Value::as_array)
        .ok_or(BoundaryInventoryError::InventoryParseFailure)?;
    let mut records = Vec::new();
    records
        .try_reserve(boundaries.len())
        .map_err(|_error| BoundaryInventoryError::InventoryParseFailure)?;
    for boundary in boundaries {
        records.push(parse_record(boundary)?);
    }
    Ok(BoundaryInventory::new(Some(schema_version), records, None))
}

fn parse_schema_version(value: Option<&serde_json::Value>) -> Result<u32, BoundaryInventoryError> {
    match value.and_then(serde_json::Value::as_u64) {
        Some(1) => Ok(1),
        _unsupported => Err(BoundaryInventoryError::SchemaVersionUnsupported),
    }
}

fn parse_record(value: &serde_json::Value) -> Result<BoundaryRecord, BoundaryInventoryError> {
    let object = value
        .as_object()
        .ok_or(BoundaryInventoryError::InventoryParseFailure)?;
    let id = required_str(object.get("id"))?.to_owned();
    let source_path = PathBuf::from(required_str(object.get("source_path"))?);
    if source_path.as_os_str().is_empty() {
        return Err(BoundaryInventoryError::InventoryParseFailure);
    }
    let class = parse_class(required_str(object.get("class"))?)?;
    Ok(BoundaryRecord::new(BoundaryRecordParts {
        id,
        class,
        source_path,
        owner: FieldState::Missing,
        threat: FieldState::Missing,
        evidence: FieldState::Missing,
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Missing,
        waiver: FieldState::Missing,
    }))
}

fn required_str(value: Option<&serde_json::Value>) -> Result<&str, BoundaryInventoryError> {
    value
        .and_then(serde_json::Value::as_str)
        .ok_or(BoundaryInventoryError::InventoryParseFailure)
}

fn parse_class(value: &str) -> Result<BoundaryClass, BoundaryInventoryError> {
    match value {
        "c_abi" => Ok(BoundaryClass::CAbi),
        "ffi" => Ok(BoundaryClass::Ffi),
        "ipc" => Ok(BoundaryClass::Ipc),
        "external_binary" => Ok(BoundaryClass::ExternalBinary),
        "decoder" => Ok(BoundaryClass::Decoder),
        "generated_code" => Ok(BoundaryClass::GeneratedCode),
        "unsafe_adjacent_dependency" => Ok(BoundaryClass::UnsafeAdjacentDependency),
        "unknown" => Err(BoundaryInventoryError::UnknownBoundaryClass),
        _unknown => Err(BoundaryInventoryError::InventoryParseFailure),
    }
}
