# SA-007: `validate_compiled_ir_record` ignores `record.metadata_hash` — accepts records with stale or absent metadata binding

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/record.rs:40-44`
- **Confidence**: confirmed

## Description

`validate_compiled_ir_record` decodes the artifact envelope and validates its digest, but never reads `record.metadata_hash`. The `metadata_hash` field exists specifically to prevent same-digest metadata mutation attacks (per the docstring on `CompiledIrRecord` in `crates/vb_storage/src/records/entities.rs:25-37`), and the test-only batch path (`put_compiled_ir` in `crates/vb_storage/src/batch/write_compiled_ir.rs:27-34`) does enforce it. The production validation path silently skips the check.

## Evidence

```rust
// crates/vb_storage/src/admission/record.rs:40-44
pub fn validate_compiled_ir_record(record: &CompiledIrRecord) -> Result<(), JournalError> {
    super::bytes::reject_oversized_compiled_ir_value(record.ir.len())?;
    let artifact = decode_accepted_artifact_envelope(&record.ir)?;
    validate_accepted_artifact_digest(&artifact, record.digest)
    // <-- record.metadata_hash is never inspected
}
```

The metadata-hash invariant is documented in `CompiledIrRecord`:

```rust
// crates/vb_storage/src/records/entities.rs:30-37
/// BLAKE3 hash of the artifact metadata fields that must remain immutable
/// after admission. `None` indicates a pre-mutation-protection record
/// (backward compatibility); such records are accepted on read but any
/// subsequent write to the same digest must provide a matching hash.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub metadata_hash: Option<[u8; 32]>,
```

The contract says "any subsequent write to the same digest must provide a matching hash" — but the read-side validation in `validate_compiled_ir_record` does not verify that the stored hash matches the recomputed artifact metadata. A read can therefore return an artifact whose `metadata_hash` field disagrees with the artifact bytes themselves.

## Adversarial Check

The check is enforced on the write path (`crates/vb_storage/src/batch/write_compiled_ir.rs:27-34`) which is `#[cfg(test)]`-only, so production writes via `persist_accepted_artifact_ir` (`crates/vb_storage/src/admission/persistence.rs:12-24`) DO compute the hash and store it. The defect is on the read/validation side: a tampered Fjall cell (corruption, bit-rot, hostile snapshot restore) where the artifact bytes have been mutated but the digest is left intact — e.g. by a flaw in the digest computation or a bug in `validate_accepted_artifact_metadata` — would not be caught by `validate_compiled_ir_record` even though `metadata_hash` is right there to be checked against `compute_artifact_metadata_hash(&artifact)`. Defense in depth requires both write- and read-side enforcement.

## Suggested Fix

Add a metadata-hash check inside `validate_compiled_ir_record`:

```rust
pub fn validate_compiled_ir_record(record: &CompiledIrRecord) -> Result<(), JournalError> {
    super::bytes::reject_oversized_compiled_ir_value(record.ir.len())?;
    let artifact = decode_accepted_artifact_envelope(&record.ir)?;
    validate_accepted_artifact_digest(&artifact, record.digest)?;

    let computed_hash = super::metadata::compute_artifact_metadata_hash(&artifact);
    match record.metadata_hash {
        Some(stored) if stored == computed_hash => Ok(()),
        Some(stored) => Err(JournalError::MetadataMutation { digest: record.digest }),
        None => Ok(()), // backward-compat: pre-mutation-protection records
    }
}
```
