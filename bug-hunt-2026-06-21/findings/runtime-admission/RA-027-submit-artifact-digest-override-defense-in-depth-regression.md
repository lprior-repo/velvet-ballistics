# RA-027: `submit_artifact` decodes `WorkflowParts` then mutates `parts.digest` to mask deserialization-integrity mismatch

- **Severity**: Low
- **Category**: correctness (integrity in depth)
- **Location**: `crates/vb_runtime/src/runtime/admission/admission_check.rs:361-380`
- **Confidence**: confirmed

## Description

This is the same-line finding as RA-005, scoped to the security implication: the override removes the only check that the deserialized `WorkflowParts.digest` field agrees with the IR bytes. An attacker who can write to the `compiled_ir` keyspace can craft an artifact whose IR bytes hash to `artifact.digest` (the lookup key) but whose deserialized `WorkflowParts.digest` field is anything — that field is then silently overwritten. Downstream consumers that cache by `parts.digest` see the artifact's digest, not the attacker-chosen one, but any consumer that compares the deserialized digest to a separately-recorded workflow digest loses the binding.

## Evidence

See RA-005 for the full code excerpt and trace. The downstream consumer chain is:

1. `submit_artifact` calls `decode_artifact_workflow` (`admission_check.rs:220`).
2. `decode_artifact_workflow` deserializes `WorkflowParts` from `artifact.ir` and overrides `parts.digest = artifact.digest` (`admission_check.rs:376`).
3. `CompiledWorkflow::try_from_parts(parts)` (`vb_core/src/workflow/mod.rs:46-62`) takes `parts.digest` as authoritative — no re-hash.
4. The resulting `CompiledWorkflow::digest()` is `artifact.digest` regardless of what the deserialized `parts.digest` was.

## Adversarial Check

RA-005's adversarial check already covers the "craftable artifact" argument. The narrower concern here is that the override is a defense-in-depth regression: even if no current consumer depends on `parts.digest == blake3(ir)`, future consumers will assume the binding is enforced (because the rest of the codebase treats `digest` as a content hash). The fix in RA-005 (replace override with explicit check) is the right move; this finding exists separately so the security review can track the integrity-binding loss independently.

## Suggested Fix

Apply RA-005's fix. Replace `parts.digest = artifact.digest;` with:

```rust
if parts.digest != artifact.digest {
    return Err(RuntimeError::AdmissionArtifactDigestMismatch {
        requested: artifact.digest,
        found: parts.digest,
    });
}
```

Then `CompiledWorkflow::try_from_parts(parts)` proceeds with the verified `parts.digest`.
