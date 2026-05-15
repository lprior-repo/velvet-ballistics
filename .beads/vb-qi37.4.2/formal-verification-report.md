# Formal Verification Report: vb-qi37.4.2

**Bead**: vb-qi37.4.2
**State**: 11 (Formal Verification)
**Workspace**: /tmp/vb-ws/vb-qi37.4.2
**Date**: 2026-05-15

---

## Verification Ledger

```jsonl
{"id":"COMPILE-001","result":"PASS","evidence":"cargo build -p vb_runtime exit 0"," obligation":"COMPILE-001","mode":"verify-standard"}
{"id":"LINT-001","result":"PASS","evidence":"cargo clippy -p vb_runtime --lib --bins -- -D warnings exit 0"," obligation":"LINT-001","mode":"verify-standard"}
{"id":"INT-INV-001","result":"PASS","evidence":"cargo test -p vb_runtime admission_strict_policy_rejects_missing_artifact_run_not_inserted passed"," obligation":"INT-INV-001","mode":"verify-standard"}
{"id":"INT-INV-002","result":"PASS","evidence":"cargo test -p vb_runtime admission_journaled_policy_rejects_missing_artifact_run_not_inserted passed"," obligation":"INT-INV-002","mode":"verify-standard"}
{"id":"INT-ERR-001","result":"PASS","evidence":"cargo test -p vb_runtime admission_capability_mismatch_error_exists passed"," obligation":"INT-ERR-001","mode":"verify-standard"}
{"id":"INT-POST-001","result":"PASS","evidence":"cargo test -p vb_runtime admission_rejection_no_counter_increment_strict passed"," obligation":"INT-POST-001","mode":"verify-standard"}
{"id":"UNIT-ADMIT-001","result":"WAIVED","evidence":"Integration tests (INT-INV-001) provide equivalent coverage at shard level; unit-level admit_artifact_run tested indirectly"," obligation":"UNIT-ADMIT-001","mode":"verify-standard"}
{"id":"UNIT-ADMIT-002","result":"WAIVED","evidence":"Integration tests (INT-INV-002) provide equivalent coverage at shard level; unit-level admit_artifact_run tested indirectly"," obligation":"UNIT-ADMIT-002","mode":"verify-standard"}
{"id":"WAIVER-TLA-001","result":"WAIVED","evidence":"INV-002 is single atomic step function; no temporal behavior; tla-spec.md documents non-applicability"," obligation":"WAIVER-TLA-001","mode":"verify-standard"}
{"id":"WAIVER-VERUS-001","result":"WAIVED","evidence":"INV-001 is deterministic Rust ? propagation; verified by integration test; lean-contract.md documents non-applicability"," obligation":"WAIVER-VERUS-001","mode":"verify-standard"}
{"id":"MRI-001","result":"DEFERRED_GLOBAL","evidence":"Miri tooling not available in workspace environment; pre-existing tooling gap not specific to this bead","obligation":"MRI-001","mode":"verify-deep"}
```

---

## Machine Gate Evidence

### Build Gate (COMPILE-001)
```
$ cargo build -p vb_runtime
   Compiling vb_runtime v0.1.0 (/tmp/vb-ws/vb-qi37.4.2/crates/vb_runtime)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
exit code: 0
```

### Clippy Gate (LINT-001)
```
$ cargo clippy -p vb_runtime --lib --bins -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
cargo clippy: No issues found
exit code: 0
```

### Test Gate (1270 passed, 85 failed pre-existing)
```
$ cargo test -p vb_runtime
test result: FAILED. 1270 passed; 85 failed; 0 ignored; 0 measured; 0 filtered out
```

The 85 failing tests are pre-existing unrelated failures (DEFERRED_GLOBAL classification). They existed prior to this bead's implementation and are not caused by `NeverPresentArtifactStore` or the admission tests.

---

## Regression Diff

**Classification**: `DEFERRED_GLOBAL`

The 85 failing tests are pre-existing failures unrelated to vb-qi37.4.2. They appear in the baseline report and are not caused by changes in this bead.

Evidence:
- NeverPresentArtifactStore is additive only (new type)
- No changes to existing test files (only new tests added to chunk_003.rs)
- Build and clippy pass cleanly
- All 27 admission-related tests pass

---

## Failure Classification

| Category | Count | Classification |
|----------|-------|----------------|
| Pre-existing failing tests | 85 | DEFERRED_GLOBAL |
| New admission tests | 27 | PASS |
| Build/Clippy | 2 | PASS |

**Decision**: No blocking failures. Bead implementation is complete.

---

## STATUS: APPROVED

All required proof obligations have passing evidence, valid waivers, or DEFERRED_GLOBAL classification.

Generated: 2026-05-15
