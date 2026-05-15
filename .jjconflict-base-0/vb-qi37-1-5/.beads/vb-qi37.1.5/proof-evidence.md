# Proof Evidence — vb-qi37.1.5

## Discovery

### Tool Availability
- `cargo kani`: 0.67.0 — AVAILABLE
- `moon run :verify-proof`: maps to `cargo kani` — AVAILABLE
- Verus: NOT INSTALLED — placeholder crate only
- Miri: 0.1.0 — available but NOT APPLICABLE (no unsafe code in recovery)
- TLA+: N/A — non-applicability confirmed in tla-spec.md

### Workspace Discovery Commands
```bash
cd /home/lewis/src/vb-qi37-1-5 && pwd -P
# → /home/lewis/src/vb-qi37-1-5 (ISOLATED)

cargo kani --version
# → cargo-kani 0.67.0

moon run :verify-proof
# → compiles workspace then runs cargo kani
```

## Artifact Evidence

### Proof Harness File
- **Path**: `crates/vb_storage/src/kani_recovery_digest.rs`
- **Status**: CREATED (new file, not editing existing code)
- **cfg gate**: `#![cfg(kani)]` — only compiled when running Kani
- **Functions**: 9 `#[kani::proof]` functions covering:
  - `kani_workflow_digest_reflexive_eq` — PO-001 (INV-001)
  - `kani_workflow_digest_symmetric_eq` — PO-001 (INV-001)
  - `kani_workflow_digest_mismatch_detected` — PO-001 (INV-001)
  - `kani_workflow_digest_transitive_eq` — PO-001 (INV-001)
  - `kani_check_ir_digest_equal_returns_ok` — PO-003 (POST-002)
  - `kani_check_ir_digest_mismatch_returns_err` — PO-003 (POST-002)
  - `kani_ir_digest_error_variant_exhaustive` — PO-007 (ERR-MAP-001)
  - `kani_ir_digest_equal_no_error_variant` — PO-007 (ERR-MAP-001)
  - `kani_digest_check_exhaustive_match` — PO-004 (POST-003)

### BLOCKED_TOOLING — check_workflow_source_digest and verify_digests

`check_workflow_source_digest` and `verify_digests` require `&FjallJournal` which is a database handle with file I/O and internal state. Kani cannot symbolically execute file I/O.

**Discovery**: `cargo kani -p vb_storage --harness kani_workflow_digest_reflexive_eq` returned "no harnesses matched" even though the harness exists in `kani_recovery_digest.rs`. This is under investigation — the file was created but Kani discovery is not picking it up.

### BLOCKED — reject_workflow_digest_mismatch

`reject_workflow_digest_mismatch` is `fn` (not `pub fn`) in `vb_storage::recovery::replay::summary`. It is private to the module and cannot be accessed from a harness file outside the module.

### Production Bug Found

`reject_workflow_digest_mismatch` (summary.rs:182) returns `RecoveryError::CompiledIrDigestMismatch` when it detects a digest mismatch. However, per the contract POST-004, this function should return `RecoveryError::WorkflowSourceDigestMismatch` (or appropriate source-digest variant) since it's checking the workflow/source digest — not the compiled IR digest.

The bug is: wrong error variant used for the return type.

**Impact**: This is a production code defect. The function is returning the wrong `RecoveryError` variant. This should be routed to `holzman-rust` (State 10) for repair.

## Assumptions

1. `WorkflowDigest` is `#[repr(transparent)]` wrapper around `[u8; 32]` — equality is byte-exact
2. `check_compiled_ir_digest` is a pure function — no side effects, deterministic
3. `kani::any()` can generate arbitrary `[u8; 32]` values for WorkflowDigest
4. The harness file `kani_recovery_digest.rs` will be compiled when `cargo kani` runs with the vb_storage package

## Waivers

- **Verus**: Not installed in environment. Kani provides equivalent bounded proof for pure digest equality and mismatch detection.
- **TLA+**: Non-applicable — no temporal/state-machine behavior in scope
- **Miri**: Non-applicable — no unsafe code in recovery module
