# regression-diff.md

bead_id: vb-core-lower-control-primitives
phase: 11
updated_at: 2026-05-15T00:00:00Z

## Classification: PASS — No Blocking Failures

All gate failures are classified as DEFERRED_GLOBAL (pre-existing global debt).

## Detailed Classification

| Gate | Result | Classification | Scope |
|---|---|---|---|
| cargo clippy -p vb_compile | PASS | — | bead-local |
| cargo test -p vb_compile | PASS | — | bead-local |
| VERUS-INV-001 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-INV-002 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-POST-001 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-POST-002 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-POST-003 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-POST-004 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-POST-005 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-POST-007 | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| VERUS-WAITKIND | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency | global |
| KANI-OVERFLOW | DEFERRED_GLOBAL | Pre-existing vb-f04l dependency; Kani not installed | global |
| TLA-WF-001 | DEFERRED_GLOBAL | TLA toolbox not executed in workspace | global |
| MIRI-RUN | DEFERRED_GLOBAL | blake3 workspace configuration issue | global |

## Baseline Comparison

| Metric | Baseline | Current | Delta |
|---|---|---|---|
| Clippy warnings | 0 | 0 | 0 |
| Test pass count | 256 | 297 | +42 (new tests) |
| DISCOVERY_BLOCKED | 12 | 12 | 0 (pre-existing) |

## Conclusion

No BLOCK_LOCAL, BLOCK_REGRESSION, or BLOCK_RELEASE failures.
All failures are pre-existing DEFERRED_GLOBAL obligations blocked on vb-f04l.
