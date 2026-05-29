# Proof Coverage Matrix (Reduced Scope) — vb-aoah

| Requirement | Proof seeds | Required verifier obligations | Behavior coverage intent |
|---|---|---|---|
| R6 | vb-aoah-seed-001 | PO-R01 (kani), PO-R08 (proptest), PO-R15 (fuzz) | Runtime open of old supported store returns MigrationRequired and performs no migration side effects. |
| R3 | vb-aoah-seed-002 | PO-R02 (kani), PO-R09 (proptest) | Every supported old storage version maps to exactly one named migration entry. |
| R4 | vb-aoah-seed-003 | PO-R03 (kani), PO-R10 (proptest) | Manifest/version advancement is impossible before verification succeeds. |
| R5 | vb-aoah-seed-004 | PO-R04 (kani), PO-R11 (proptest), PO-R16 (fuzz) | Cleanup-required migration reports success only after the old keyspace is empty. |
| R7 | vb-aoah-seed-005 | PO-R05 (kani), PO-R12 (proptest) | Reopen after successful migration reads current records without invoking migration. |
| R9 | vb-aoah-seed-006 | PO-R06 (kani), PO-R13 (proptest), PO-R17 (fuzz) | Empty old-keyspace behavior is explicit no-op and cannot silently claim an unverified migration. |
| R11 | vb-aoah-seed-007 | PO-R07 (kani), PO-R14 (proptest), PO-R18 (fuzz) | Migration counters and byte limits use checked bounded arithmetic and cannot overflow into success. |

## Lane coverage summary

| Verifier | Obligations | Seeds covered |
|---|---|---|
| kani | 7 (PO-R01–PO-R07) | 001–007 (all) |
| proptest | 7 (PO-R08–PO-R14) | 001–007 (all) |
| cargo-fuzz | 4 (PO-R15–PO-R18) | 001, 004, 006, 007 |
| tla-plus | 0 (excluded) | — |
| verus | 0 (excluded) | — |
| flux-rs | 0 (excluded) | — |
| loom | 0 (not applicable) | — |
| miri | 0 (not applicable) | — |

**Total obligations**: 18 (reduced from 36 over-scoped).

## Excluded lane rationale

- **TLA+**: Test-first bead. No production temporal behavior exists. See proof-plan-review.md §Excluded Lane Rationale.
- **Verus**: Test-first bead. No production Rust implementation to bind Verus specs to. GOD RULE "No Vacuum Verus Proofs" applies. See proof-plan-review.md §Excluded Lane Rationale.
- **Flux**: Test-first bead. No refinement type-level enforcement needed at skeleton stage. See proof-plan-review.md §Excluded Lane Rationale.
- **Loom**: No concurrency scope. boundary-map.md and hazard-analysis.md confirm pure-core, no async/shared-state. See proof-plan-review.md:84.
- **Miri**: No unsafe/FFI/raw-pointer scope. boundary-map.md and hazard-analysis.md confirm safe Rust only. See proof-plan-review.md:85.

Traceability source: `traceability-matrix.jsonl`. Every proof seed has decisions for all 8 verifiers in `verifier-lane-decisions.jsonl`. Required obligations are serialized in `proof-obligations.planned.jsonl`.
