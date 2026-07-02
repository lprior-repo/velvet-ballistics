# trusted-base-plan.md — vb-pcu4h

- bead_id: vb-pcu4h
- planner_state: 4
- schema_version: trusted-base-plan/v1
- produced_by: proof-planner (State 4)
- planner_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`

## 1. Trusted-base thesis

The bead vb-pcu4h is a pure test-assertion-strength uplift. The trusted base is the *unchanged production code that the tests exercise*. Specifically:

1. The recovery reducer (`recover_runtime_frame_seed_from_events` and the journal-backed alias `recover_runtime_frame_seed`).
2. The accumulator (`FrameSeedAccumulator.pending_actions: HashSet<(ActionId, StepIdx)>` and its init).
3. The sort-and-assemble site (`recovered_pending_actions` in `derive.rs:287-296`).
4. The production struct `RecoveredPendingAction` and its derives.
5. The Verus mirror and STRONG `#[path]` binding.

These are the regions whose behaviour the tests depend on; the tests do not assert reducer *correctness* beyond length and field equality of the recovered Vec. Trusted-base attestation is what the drift gates (`scripts/check-production-inner-drift.sh` and `scripts/check-verus-production-binding.sh`) prove at closure.

## 2. Trusted-base inventory

| ID | Path | Symbol | Lines | Role | Trust basis |
|----|------|--------|-------|------|-------------|
| TB-001 | `crates/vb_storage/src/recovery/types.rs` | `RecoveredPendingAction` | 644-650 | Production struct under assertion; `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` | Source-of-truth struct definition; unchanged by bead. Drift gate proves mirror parity. |
| TB-002 | `crates/vb_storage/src/recovery/types.rs` | `UnsupportedRecoveryState::pending_actions` | 661-662 | Boolean preserved in Test A | Derivation path from accumulator's `is_empty()` check; covered by retained assertion. |
| TB-003 | `crates/vb_storage/src/recovery/replay/summary/derive.rs` | `recover_runtime_frame_seed_from_events` | 69-73 | Recovery entry point for Tests A/B/C | Public fn returning `RecoveryResult<RecoveryFrameSeed>`; unchanged. Tested by the three PRIMARY fixtures returning `Ok(_)`. |
| TB-004 | `crates/vb_storage/src/recovery/replay/summary/derive.rs` | `recovered_pending_actions` | 287-296 | Sort + Vec assembly | Sorts ascending by `(step, action)`; for single-element input the sort is trivial but canonical. |
| TB-005 | `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` | `FrameSeedAccumulator::pending_actions` field | 35 | `HashSet<(ActionId, StepIdx)>` collecting unresolved actions | Uniqueness basis; `Vec::eq` length-1 invariant depends on it. |
| TB-006 | `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` | `pending_actions: HashSet::new()` init | 68 | Initial empty set | Init preserves "drop-all" failure mode (vec length 0) when no event lands. |
| TB-007 | `crates/vb_storage/src/recovery/recover.rs` | `recover_runtime_frame_seed(&journal, run)` | (alias) | Journal-backed entry for SECONDARY targets | Re-export; same return type as TB-003. |
| TB-008 | `crates/vb_storage/src/recovery/mod.rs` | `pub use RecoveredPendingAction` | 42 | Re-export chain for `summary::tests.rs` imports | Surface that lets the test file reference the type via `crate::recovery::RecoveredPendingAction` without modifying `summary/mod.rs`. |
| TB-009 | `verification/verus/production_inner/replay_invariants_production.rs` | `RecoveredPendingAction` mirror | 253-256 | STRONG `#[path]`-bound production mirror | Drift gate verifies byte-for-byte parity with TB-001. |
| TB-010 | `verification/verus/extern_vb_rpch_replay_invariants.rs` | `pub use prod_src::RecoveredPendingAction;` | 191 | Re-export of TB-009 | STRONG binding surface; binding gate verifies. |
| TB-011 | `verification/verus/production_inner/recovery_verification_production.rs` | provenance comments | 25, 45 | Reference to production struct | Comments only; drift gate verifies file integrity. |
| TB-012 | `crates/vb_storage/src/recovery/replay/summary/tests.rs` | import block | 1-9 | Existing `use` lines | The replacement test relies on `use crate::recovery::replay::summary::*;` for reducer/summary and the new `use crate::recovery::RecoveredPendingAction;` line for the struct literal. |

## 3. Trusted-base boundary at the test file

The test file at `crates/vb_storage/src/recovery/replay/summary/tests.rs` is **NOT** in the trusted base — it is the artifact under test for State 9. The test file's import block (TB-012) is the only anchor, and the planner explicitly recommends that the implementation add `use crate::recovery::RecoveredPendingAction;` to the existing import block (because `summary::*` does not currently re-export the type — see `summary/mod.rs:36-42`).

`summary/mod.rs` itself is **NOT** in the trusted base either; it is marked `status: read-only-for-bead` in `delivery-scope.jsonl#4`. Modifying it would re-export `RecoveredPendingAction` and obviate the test-side `use` line, but the contract forbids that edit.

## 4. Out-of-trusted-base (must NOT be assumed)

These are assumptions the planner makes about the runtime environment — they are **not** part of the production trusted base:

- The test runner is `cargo test --lib` with the project's pinned nightly (per `docs/rust-governance.md`).
- `cargo fmt --all -- --check` is run with the same project rustfmt config.
- `moon run :lint-src` delegates to the configured lint task and exits 0 on a clean tree.
- Drift and binding gates are bash scripts read from `scripts/check-production-inner-drift.sh` and `scripts/check-verus-production-binding.sh`; the planner verifies their presence in the workdir at planning time but does not execute them.
- The `-p vb_storage` and `-p vb_runtime` package names are stable (verified by `Cargo.toml` and `Cargo.lock` presence).

## 5. New trusted-base rows (zero)

The bead adds **zero** new trusted-base rows. No proof-spec is added (the forbidden list forbids new Kani/Flux/Verus harnesses). The closure surface is `cargo-test + moon lint + drift gates + cargo fmt`, all on the existing trusted base.

## 6. Trusted-base attestation commands

Closure evidence that the trusted base is unchanged:

| Gate | Command | Purpose |
|------|---------|---------|
| Production-inner drift | `bash scripts/check-production-inner-drift.sh` | Confirms TB-001 matches TB-009 byte-for-byte. |
| Production-binding | `bash scripts/check-verus-production-binding.sh` | Confirms TB-010 `#[path]` is intact. |
| Source lint | `moon run :lint-src` | Confirms tests compile clean under project lint. |
| Format | `cargo fmt --all -- --check` | Confirms tests conform to project format. |

All four must exit 0 for the trusted base to be considered attestable; the closure ledger at State 12 captures their raw command outputs.

## 7. Reused invariants from `master`

The bead does not introduce invariants outside the per-test replacement contract. Master invariants in play (for reviewer convenience, not as new trusted-base items):

- Master INV — Source-lint zero-tolerance (every PR).
- Master INV — Type-driven design (struct-level equality via derived `PartialEq, Eq`).
- Master INV — Functional-core/imperative-shell boundary (the reducer is pure; the test is shell).

## 8. Hand-off

The trusted base is closed at this bead's end. State 12 (`formal-verifier`) is responsible for capturing the raw `bash scripts/check-production-inner-drift.sh` exit status and the raw `bash scripts/check-verus-production-binding.sh` exit status into the closure ledger, plus the `cargo test --lib -- --nocapture` exit status and the `cargo fmt --all -- --check` exit status.

No claim of attestation. State 12 is the authority for "trusted base unchanged + verified."
