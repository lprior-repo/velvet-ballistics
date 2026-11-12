# vb-w25-runtime-a2 — Review Blocker Closure Notes

Round: latest black-hat review blockers (FINDING-DRIFT-001, FINDING-TEST-WEAKNESS-001, FINDING-DOC-001).
Workspace (isolated): `/home/lewis/src/isoloated/velvet-ballistics-w25-runtime-a2`
`git rev-parse --show-toplevel` == `jj root` == isolated workspace (verified before editing).
Fjall left untouched and mandatory.

## FINDING-DRIFT-001 (CRITICAL) — production-inner drift — CLOSED

Root cause: `verification/verus/production_inner/digest_check_production.rs` carried stale
per-section `Source:` ranges. The enum section claimed
`crates/vb_storage/src/recovery/types.rs:1635-1643`, which now spans the production
`impl Default for ActionReplayTracker` block (lines 1636-1640). The drift gate extracts the
`ActionReplayTracker` identifier from that production range and found it missing from the mirror.

Baseline (before fix):
```
DRIFT: production identifiers in crates/vb_storage/src/recovery/types.rs missing from mirror:
  - ActionReplayTracker
... Drift findings: 1 ... EXIT=1
```

Fix: regenerated the mirror with accurate per-section annotations:
- New verbatim section `Source: ...:1636-1640` mirroring
  `impl Default for ActionReplayTracker { fn default() -> Self { Self::new() } }` verbatim,
  backed by a minimal local `ActionReplayTracker` unit-struct stub (+ inherent `new()`) so the
  `#[path]`-included mirror type-checks under Verus. `ActionReplayTracker` is now in the mirror
  identifier set.
- Enum section corrected to `Source: ...:1642-1652`.
- Impl-block section corrected to `Source: ...:1654-1688`.
- Header DRIFT POLICY range updated to `1636-1688`; documented a 4th substitution (the stub).

Evidence:
- `raw/production-inner-drift.log` — `bash scripts/check-production-inner-drift.sh` → EXIT=0, Drift findings: 0.
- `raw/verus-digest-check.log` — `verus --crate-type=lib verification/verus/extern_vb_rpch_digest_check.rs`
  → `verification results:: 1 verified, 0 errors`, EXIT=0 (mirror still compiles/binds).
- `raw/verus-production-binding.log` — `bash scripts/check-verus-production-binding.sh` → EXIT=0
  (STRONG 0 / WEAK 72 / VACUUM 0).

## FINDING-TEST-WEAKNESS-001 (MEDIUM) — tautological store_missing assertion — CLOSED

Root cause: the storage `RecoveryFrameSeed` classifier
(`RecoveryCannotResumeState::from_seed`) calls
`mark_missing_components(MissingRunStateComponents::ALL)` unconditionally, so the storage typed
witness reports `store_missing = true` for ANY recovered seed. The old test asserted that constant
on both the runtime product and the storage witness for a single `List`-valued run — tautological.

Honest code behavior (verified by reading production, not assumed):
- Storage witness (`recover_runtime_frame_seed_from_events`, no workflow): `store_missing` is
  ALWAYS true (conservative "whole RunState missing"); it is INSENSITIVE to slot value type in the
  pending-action scenario (no succeeded step → `RecoveredSlots::supported`, so `slot_values` stays
  false too). No flag in the storage witness distinguishes `List` from `I64` here.
- Runtime layer REFINES this: `vb_runtime::recovery::full::classify_full_recovery_resume` sets
  `store_missing` only when `recovered_slots_require_value_store(seed)` is true
  (`SlotValue::List | Object | Blob`).

Because the storage witness cannot itself distinguish the two sets, the reviewer's literal recipe
("storage witness store_missing set vs not set") is not achievable through slot value type. The
strengthened test instead proves the genuine, non-tautological property: the runtime product tracks
the storage-recovered value-store evidence.

New test (`runtime_recover_product_cannot_resume_witness_carries_through_for_store_missing`) drives
two journal-event sets identical except for the single seeded slot value, and inspects the
independently storage-recovered seed's slot payloads:
- `SlotValue::List` → storage-recovered seed carries a value-store-required slot value; runtime
  `recover_product` → `CannotResume { reason: "store_missing" }` and the typed
  `RecoveryCannotResumeState.store_missing` flag is set.
- `SlotValue::I64(101)` (byte-identical otherwise) → storage-recovered seed carries NO
  value-store-required slot value; runtime refines `store_missing` away → product is `Resumable`.

Non-tautology guarantee: if `store_missing` were the old unconditional constant, the `I64` run would
be `CannotResume`, not `Resumable`, and the test would fail. The `Resumable` branch of the scalar
case is corroborated by the existing happy-path
`recover_and_resume_rehydrates_fjall_pending_action_ticket_after_reopen` (uses `I64(0)` → Ok) and
the `List` branch by `recover_and_resume_fails_closed_for_store_dependent_fjall_pending_action_after_reopen`.

Evidence:
- `raw/test-runtime-recovery.log` — full `runtime_fjall_pending_action_recovery` suite: 19 passed, EXIT=0.

## FINDING-DOC-001 (LOW) — recover_and_resume doc-comment — CLOSED

`Runtime::recover_and_resume` doc-comment updated: "reconstruct a `RecoveryFrameSeed`" →
"reconstruct a typed `RecoveryFrameSeedProduct`", and added a paragraph naming the
FINDING-001/FINDING-002 typestate boundary and the parallel
`crate::recovery::RuntimeRecoveryProduct` (`SummaryOnly`/`CannotResume { reason }`/`Resumable`).

## Gate statuses (this round)

| Gate | Command | Status |
|---|---|---|
| production-inner drift | `bash scripts/check-production-inner-drift.sh` | PASS (EXIT=0) |
| Verus binding | `bash scripts/check-verus-production-binding.sh` | PASS (EXIT=0) |
| Verus digest-check | `verus --crate-type=lib extern_vb_rpch_digest_check.rs` | PASS (1 verified, 0 errors) |
| panic-surface | `bash scripts/check-panic-surface.sh` | PASS (EXIT=0) |
| hot-cold | `bash scripts/check-hot-cold-forbidden-apis.sh` | PASS (EXIT=0) |
| ignored-fallible | `bash scripts/check-ignored-fallible-results.sh` | PASS (EXIT=0) |
| nightly-features | `bash scripts/check-nightly-features.sh` | PASS (EXIT=0) |
| source-length | `bash scripts/check-source-length.sh` | PASS (EXIT=0) |
| fmt | `cargo fmt --all --check` | PASS (EXIT=0) |
| check | `cargo check --workspace --all-targets --all-features` | PASS (EXIT=0) |
| strict source clippy | canonical `lint-src` (nightly-2026-04-28, `-Dwarnings` + Holzman set) | PASS (EXIT=0) |
| runtime recovery tests | `--test runtime_fjall_pending_action_recovery` | PASS (19 passed) |
| vb_storage recovery tests | `cargo test -p vb_storage recovery` | PASS (254 passed) |
| Fjall reopen tests | `integration_strict_persist_drop_reopen`, `integration_subprocess_wal_crash_recovery` | PASS (3+3) |
| integration/recovery contract | `integration_storage_runtime_recovery`, `integration_runtime_storage_fault_tolerance`, `vb_jpq7_3_*`, `vb_qi37_1_1_*`, `fjall_keyspace_manifest_tests`, `integration_storage_runtime_validate_pipeline` | PASS (13/18/11/19/23/15) |

Raw logs: `evidence/runtime-a2/raw/`.

## Residual risk / disclosure

- The storage typed witness `RecoveryFrameSeedProduct::cannot_resume_state()` reports
  `store_missing = true` unconditionally (conservative storage layer). The runtime layer legitimately
  refines it. The strengthened test therefore proves the runtime store_missing decision is DERIVED
  from storage-recovered slot evidence, rather than asserting the two layers agree bit-for-bit (they
  do not agree for scalar-only runs, by design). This divergence is intended, not a defect.
- No performance claims made in this round (doc/test/mirror changes only); no benchmark evidence
  required or produced.
- `moon ci` was not run end-to-end this round; the individual canonical gates above were run directly
  (including the exact canonical `lint-src` clippy command from `.moon/tasks/all.yml`).
