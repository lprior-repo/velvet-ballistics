# runtime-a2 recovery closure evidence (vb-d9ywf / vb-sixsf second slice)

Raw command logs are tracked under `evidence/runtime-a2/raw/`.

## Scope of this slice

Targeted black-hat blockers for `vb-d9ywf` (`pending action timer and ask
hydration into full run state`) and `vb-sixsf` (`typestate frame seeds into
summary-only cannot-resume and resumable`):

1. Split oversized recovery/control functions (`scheduled_pending_action_effect`,
   `product_from_recovered_seed`, `prepare_recovered_run`,
   `validate_recovered_open_ask`, `handle_ask_answer`) without behaviour change.
2. Reduce recovery fan-in and repair broken `from_seed`/`from_product`
   compile errors in tests that the typestate boundary now exposes.
3. Address the typestate bypass via doc + restricted constructors, leaving
   full closure explicitly NOT claimed.
4. Keep `vb-d9ywf` open and narrowed for durable timer authority (no fake
   hydration of wait / timed-ask).

No production behaviour was changed. Source clippy, source-length,
Verus production-binding, Verus production-inner drift, runtime
regression, and storage regression gates all PASS.

## Function splits and recovery fan-in

All five named functions are already at or under the 25-logical-line
Holzman limit (confirmed by `scripts/check-source-length.sh`):

- `crates/vb_runtime/src/recovery/full/pending.rs::scheduled_pending_action_effect`
- `crates/vb_runtime/src/runtime.rs::product_from_recovered_seed`
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs::prepare_recovered_run`
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs::validate_recovered_open_ask`
- `crates/vb_runtime/src/shard/lifecycle/chunk_002_parts/chunk_000_ask_answer.rs::handle_ask_answer`

The previous slice's split work had already driven these to <=25 logical
lines with small local helpers; this slice keeps them in shape and
consolidates the recovery fan-in with one new typed helper.

This slice adds:

- `Runtime::rejected_recovery_product_error` — extracts the non-Resumable
  arm of `Runtime::recover_and_resume` so the public entry only narrates
  the resumable path, and the rejection taxonomy lives in one place
  (preserved `RuntimeError::UnsupportedFullRecoveryHydration` /
  `RuntimeError::RecoveryCannotResume` mapping, no behaviour change).
- Comment-only test marker on `vb_qi37_1_1_red_recovery_contract_test`
  to make the raw-DTO reuse explicit (`// Compat DTO surface ...`),
  aligning with the typestate closure roadmap below.

## Typestate bypass (vb-sixsf)

Status: **full closure is NOT yet claimed.**

Public recovery entry points already return the typed
`RecoveryFrameSeedProduct` and the runtime boundary preserves the storage
typestate (`RuntimeRecoveryProduct::CannotResume`/`Resumable`). The raw
`RecoveryFrameSeed` DTO remains `pub` so verifier/compat code paths
still compile:

- low-level replay tests assert exact field-by-field reconstruction
  (`recovery_hydration_tests`, `vb_qi37_1_1_red_recovery_contract_test`,
  `integration_storage_runtime_validate_pipeline`),
- the Verus verifier mirror (`verification/verus/extern_recovery_verification.rs`)
  binds to the raw DTO, and
- compat paths
  `vb_storage::recovery::recover::recover_raw_runtime_frame_seed` /
  `recover_raw_runtime_frame_seed_from_events` continue to return it
  deliberately.

Resolution of the four raw `from_seed` compile errors in
`recovery_hydration_tests.rs`, `integration_storage_runtime_validate_pipeline.rs`,
and `vb_qi37_1_1_red_recovery_contract_test.rs`:

- Tests whose `seed` came from `recover_runtime_frame_seed[_from_events]`
  (typestate products) now call
  `DurableFrameRecoveryBoundary::from_product` so the storage
  `RecoveryCannotResumeState` witness survives.
- `vb_qi37_1_1_red_recovery_contract_test` switches its raw replay
  surface to `recover_raw_runtime_frame_seed_from_events` (the documented
  `_raw_` compat path), leaving the production `Runtime` caller routed
  through the typestate product.

Doc + restricted-constructor changes (minimal blast radius, no API
rename, no test breakage):

- `crates/vb_storage/src/recovery/types.rs::RecoveryFrameSeed` — doc
  comment now states the typestate status, lists the public-visibility
  rationale (verifier mirrors, compat paths), and points at the closure
  roadmap.
- `crates/vb_runtime/src/recovery.rs::DurableFrameRecoveryBoundary::from_seed`
  — doc comment explicitly states the constructor is the **compat**
  surface and that production callers MUST go through `from_product`.
- `verification/verus/extern_recovery_verification.rs`,
  `extern_vb_rpch_seed_dimensions.rs`, `extern_idempotency_replay_tracker.rs`,
  and the in-tree mirrors under
  `verification/verus/production_inner/*.rs` had stale
  `RecoveryFrameSeed` / `RecoveryCannotResumeState` /
  `ActionReplayTracker` / `DigestCheck` / `UnsupportedRecoveryState` /
  `cancel_kill_lattice` BINDING LEDGER line references that were left
  stale by previous splits; they were updated to the current production
  line ranges so the production-inner drift gate stays at zero.

Full closure still requires (tracked under `vb-sixsf`, not done here to
honour the "do not silently expand backwards-compat" rule):

1. migrating every verifier/compat caller to `RecoveryFrameSeedProduct`,
2. gating `pub` field access behind accessor methods that preserve
   `RecoveryCannotResumeState` propagation, and
3. deleting the `from_seed` constructor on
   `vb_runtime::recovery::DurableFrameRecoveryBoundary` so the
   boundary can no longer be entered via the raw DTO.

## vb-d9ywf timer / timed-ask hydration

`vb-d9ywf` remains **open and narrowed** for wait / timed-ask durable
timer authority. No live hydration was added; no synthesised
`WaitResolved` / `AskTimedOut` resume state was produced. The
durable-timer-authority-decision note
(`evidence/runtime-a2/durable-timer-authority-decision.md`) still
applies verbatim — the new durable timer-authority contract and the
corresponding Fjall reopen tests are owed by the follow-up bead.

## Command statuses (this slice)

```text
raw/source-length.log                    EXIT_STATUS: 0; over_limit=0 all categories
raw/fmt-check.log                        EXIT_STATUS: 0
raw/check.log                            EXIT_STATUS: 0 (runtime + storage + workspace-tests targets compile)
raw/clippy.log                           EXIT_STATUS: 0
raw/verus-production-binding.log         EXIT_STATUS: 0 (WEAK=72, VACUUM=0)
raw/production-inner-drift.log           EXIT_STATUS: 0 (Drift findings: 0 after BINDING LEDGER refresh)
raw/runtime-boundary-typestate-test.log  EXIT_STATUS: 0
raw/runtime-fjall-test.log               EXIT_STATUS: 0
raw/runtime-recovery-suite.log           EXIT_STATUS: 0
raw/runtime-recovery-unit.log            EXIT_STATUS: 0
raw/storage-recovery-tests.log           EXIT_STATUS: 0
raw/storage-typestate-test.log           EXIT_STATUS: 0
raw/vb-runtime-recovery-tests.log        EXIT_STATUS: 0
raw/vb-runtime-recovery-unit.log         EXIT_STATUS: 0
raw/vb-storage-recovery-tests.log        EXIT_STATUS: 0
```

## Not claimed here

- No `moon ci` status is claimed here. Bead closure, Dolt sync, and
  Git/JJ push were not performed in this scoped continuation.
- Verus / Flux / Kani harness success is **not** claimed for this slice
  (the recovery source slice is unchanged in spirit and the existing
  evidence above already exercises the recovery surface via focused
  unit/BDD/Fjall-reopen tests).
- The recoverable open-ask narrow path remains the only resumable
  external-boundary state until `vb-d9ywf` adds durable timer
  authority.
