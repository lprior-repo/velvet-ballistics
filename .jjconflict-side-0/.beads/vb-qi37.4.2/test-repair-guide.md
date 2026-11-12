# Test Repair Guide - vb-qi37.4.2

## Routing

Route back to State 8 test-writer after implementation/API repair, not directly to landing.

## Keep

- Keep the current exact red tests in `tests/vb_qi37_4_2_strict_runtime_admission.rs` for missing artifact, malformed/decode failure, gate mismatch, durable flag, digest mismatch, stale certificate, capability exactness, valid admission record, budget capacity, and P02 gate singleton.
- Keep exact assertions. Do not replace them with `is_ok()`, `is_err()`, or unbound `matches!`.

## Implementation/API Preconditions Before Retrying State 9

1. Add exact runtime/public taxonomy for digest mismatch preserving requested, record, and envelope identities.
2. Add exact stale certificate/evidence metadata and diagnostic preservation.
3. Revalidate accepted-artifact gate count and all required proof flags in the runtime admission boundary, not only inside the store.
4. Prove denial happens before frame/run/runnable/drive/journal accepted-event allocation.
5. Prove strict/journaled constructors cannot route through `AlwaysPresentArtifactStore` or existence-only `compiled_ir_exists` paths.

## Missing Test Additions Required Before Resubmission

1. B08: public RuntimeError/API/CLI/IPC diagnostic preservation for ERR-001 through ERR-008 with exact category, digest, and semantic cause assertions.
2. B11: denial state-invariance checks for every ERR-001 through ERR-007: no frame taken, no run inserted, no runnable state, no `drive_run`, no `RunAccepted`.
3. B12/B14: strict/journaled production constructor tests that reject missing storage-backed accepted-artifact loader and prove dummy/existence-only stores are relaxed/test-only.
4. B13: static or executable guard proving strict admission does not parse YAML/JSON/raw `WorkflowParts`.
5. B02 matrix: raw `WorkflowParts`, YAML, JSON, empty bytes, truncated postcard, and malformed bytes.
6. B03 matrix: unsupported schema/version, missing acceptance fields, unsupported proof status, absent required proof marker, and each required proof flag false.
7. P01, P03, P04, P05, P06 proptest invariants from the approved plan.
8. Fuzz/Kani/mutation/CI evidence must be run or recorded as downstream WAIVED/DEFERRED evidence with owner, reason, expiry, limitation, and compensating evidence before any pass claim.

## Resubmission Gate

- Rerun State 9 from Tier 0 after repairs. Do not rerun only the failing focused tests.
