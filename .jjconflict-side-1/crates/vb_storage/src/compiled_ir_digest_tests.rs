use super::*;

#[test]
fn digest_mismatch_stale_digest() -> TestResult {
    let valid = stamp_v1(parts(0))?;
    assert_ne!(valid.digest, WorkflowDigest::from_bytes([0; 32]));
    let mut stale = valid;
    stale.digest = WorkflowDigest::from_bytes([0; 32]);
    let workflow = CompiledWorkflow::try_from_parts(stale)?;
    assert_checksum(encode_compiled_ir(&workflow))
}

#[test]
fn decode_rejects_stale_v1_digest() -> TestResult {
    assert_checksum(decode_compiled_ir(&postcard::to_allocvec(&parts(0))?, None))
}

#[test]
fn same_length_symbol_mutation_changes_only_digest_binding() -> TestResult {
    let original = bound_ir(&["key", "é", " "])?;
    let mut changed = original.clone();
    *changed
        .symbol_payloads
        .first_mut()
        .ok_or("missing symbol")? = Box::from("kez");
    assert_eq!(changed.parts, original.parts);
    assert_eq!(wire_v2(&changed)?.len(), wire_v2(&original)?.len());
    assert_ne!(
        stamp_v2(changed.clone())?.parts.digest,
        original.parts.digest
    );
    assert_checksum(decode_compiled_ir(&wire_v2(&changed)?, None))
}

#[test]
fn symbol_order_changes_canonical_digest() -> TestResult {
    let original = bound_ir(&["key", "é", " "])?;
    let mut changed = original.clone();
    changed.symbol_payloads.rotate_left(1);
    assert_eq!(changed.parts, original.parts);
    assert_ne!(
        stamp_v2(changed.clone())?.parts.digest,
        original.parts.digest
    );
    assert_checksum(decode_compiled_ir(&wire_v2(&changed)?, None))
}
