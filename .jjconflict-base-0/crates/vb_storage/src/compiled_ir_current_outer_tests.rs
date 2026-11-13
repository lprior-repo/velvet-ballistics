use crate::compat::SchemaVersion;
use crate::{decode_compiled_ir, encode_compiled_ir};
use vb_core::CompiledWorkflow;

#[path = "compiled_ir_current_outer_boundaries.rs"]
mod boundaries;

#[path = "compiled_ir_current_outer_metadata.rs"]
mod metadata;

fn check_name(name: &str, prefix: &[u8]) -> super::support::TestResult {
    let mut parts = super::support::parts(0);
    parts.name = Box::from(name);
    let stamped = super::support::stamp_v1(parts)?;
    let workflow = CompiledWorkflow::try_from_parts(stamped)?;
    let bytes = encode_compiled_ir(&workflow)?;
    let raw = postcard::to_allocvec(&workflow.to_parts())?;
    assert_eq!(bytes.as_ref(), raw.as_slice());
    assert!(bytes.starts_with(prefix));
    assert!(!bytes.starts_with(&[1, 255]));
    let decoded = decode_compiled_ir(&bytes, Some(SchemaVersion::new(2, 0, 0)))?;
    assert_eq!(decoded, workflow);
    assert_eq!(decoded.symbol_payloads(), None);
    Ok(())
}

#[test]
fn current_outer_raw_roundtrip() -> super::support::TestResult {
    super::support::assert_v1(0, Some(SchemaVersion::new(2, 0, 0)))
}

#[test]
fn current_outer_raw_nonzero_symbols() -> super::support::TestResult {
    super::support::assert_v1(1, Some(SchemaVersion::new(2, 0, 0)))
}

#[test]
fn current_outer_bound_empty() -> super::support::TestResult {
    super::support::assert_v2(&[], Some(SchemaVersion::new(2, 0, 0)))
}

#[test]
fn current_outer_bound_ordered() -> super::support::TestResult {
    super::support::assert_v2(&["key", "é", " "], Some(SchemaVersion::new(2, 0, 0)))
}

#[test]
fn current_outer_raw_name_utf8_and_varint() -> super::support::TestResult {
    check_name("a", &[1, 97])?;
    check_name("é", &[2, 195, 169])?;
    check_name(&"a".repeat(127), &[127])?;
    check_name(&"a".repeat(128), &[128, 1])?;
    check_name(&"a".repeat(255), &[255, 1])?;
    check_name(&"a".repeat(256), &[128, 2])?;
    Ok(())
}
