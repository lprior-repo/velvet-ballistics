# Boundary Map — vb-pcu4h

- bead_id: vb-pcu4h
- artifact_owner: rust-contract
- scope: boundaries touched (none) and boundaries observed (test-only) by this bead.

## Boundary touch summary

| Boundary | Touched by bead? | Justification |
|----------|------------------|----------------|
| Pure core (replay reducer) | NO | `crates/vb_storage/src/recovery/replay/summary/{derive.rs,accumulator.rs}` is read-only for this bead. |
| Storage boundary | NO | Journal-backed alias `recover_runtime_frame_seed` (`crates/vb_storage/src/recovery/recover.rs`) is read-only. |
| Codec boundary | NO | No event variant or kind range is added or changed. |
| Async boundary | NO | Recovery is synchronous; no tokio / async runtime involvement. |
| FFI / unsafe boundary | NO | The bead does not introduce or modify any `unsafe`, `extern`, or FFI surface. |
| Time / clock boundary | NO | No time-dependent assertion. |
| Network boundary | NO | No HTTP / RPC / serialization payload is asserted beyond in-process `JournalEvent` literals. |
| Parser boundary | NO | No external string / YAML / JSON / bytes are parsed at the test boundary. |
| Test boundary (this bead's surface) | YES | Three PRIMARY test bodies in `crates/vb_storage/src/recovery/replay/summary/tests.rs` are rewritten in their assertion region. Two SECONDARY targets in `crates/vb_runtime/tests/recovery_hydration_tests.rs` are RECOMMENDED for uplift in the same patch but treated as optional. |

## Test-side boundary (this bead's actual edit surface)

The bead lives entirely inside the test boundary. No production code, no fixture file, no schema file, no proof artifact is edited. The edit surface is:

```
[ Test body (lines 437-454) ]  ← assert region rewritten
[ Test body (lines 621-672) ]  ← assert region rewritten
[ Test body (lines 743-809) ]  ← assert region rewritten (one assertion only; rest of body preserved)
```

Each test's setup region (event construction) is unchanged. Each test's other assertions (`slot_count`, `step_count`, `step.state`, `summary.actions_scheduled`, `unsupported.pending_actions`, the redundant recovery call in Test C) are preserved.

## Boundary diagram (textual)

```
+-----------------------------------------------------------+
| Test body (per PRIMARY target)                            |
|                                                           |
|  Setup region (RunId, events)  ─── unchanged              |
|                                                           |
|  Boundary: Recovery API call                               |
|     |                                                      |
|     v                                                      |
|  recover_runtime_frame_seed_from_events(&events)          |
|     |                                                      |
|     v                                                      |
|  RecoveryFrameSeed { ..., pending_actions: Vec<...>, ... }|
|     |                                                      |
|     v                                                      |
|  Assertion region  ─── REWRITTEN                           |
|     |  .expect("…")                                         |
|     |  assert_eq!(pending_actions, vec![RecoveredPendingAction{…}]) |
|     |  (Test A only) assert!(unsupported.pending_actions) |
|                                                           |
+-----------------------------------------------------------+
```

## Boundary invariants preserved

- BI-1 — Type identity. `RecoveredPendingAction` remains `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`. The struct literal `RecoveredPendingAction { step, action }` is legal at the test boundary because both fields are public.
- BI-2 — Sort order. `derive.rs:294` sorts by `(step, action)`. The constructed literal vec is single-element so sort order is canonical; multi-element vec equality under this contract is implicitly sorted-canonical because production guarantees sort.
- BI-3 — Drift gate. The Verus mirror at `verification/verus/production_inner/replay_invariants_production.rs:253-256` matches production byte-for-byte. Drift gate (`scripts/check-production-inner-drift.sh`) runs as a closure gate; the bead does not edit the mirror.
- BI-4 — Production-binding gate. `scripts/check-verus-production-binding.sh` enforces `#[path = "..."]` STRONG binding on the mirror; the mirror is already STRONG-bound, so the gate is expected to pass unchanged.
- BI-5 — No nightly feature use. The replacement uses only `assert_eq!` and `vec![...]`, which are stable Rust. No `allocator_api` / `generic_const_exprs` / `portable_simd` / `try_blocks` needed.

## Boundary dependencies that the test must respect

- DEP-1 — `RecoveredPendingAction` is in scope at the test boundary because `summary::tests` imports `crate::recovery::replay::summary::*` (`tests.rs:2`), which re-exports `RecoveryFrameSeed` and `RecoveredPendingAction` via `summary/mod.rs`. If the import line ever changes, the test will fail to compile; the contract RECOMMENDS the holzman-rust agent preserve `use crate::recovery::replay::summary::*;` verbatim.
- DEP-2 — `StepIdx::new(u32)` and `ActionId::new(u32)` constructors are infallible newtype wraps. Their use at the test boundary is unambiguous and does not require additional imports beyond the existing `use vb_core::{..., StepIdx, ActionId, ...}` at `tests.rs:7`.

## Boundary out of scope

- `vb_compile` / `vb_cli` / `vb_dispatch` — no compile-time or CLI hooks; the recovery tests are library-only.
- `fuzz/` — no fuzz harness for `RecoveredPendingAction` shape; the bead does not require fuzz coverage.
- `verification/verus/**` mirrors — STRONG-bound to production; no edit needed; drift gate runs as gate only.
- `verification/flux/**` — Flux refinements target `UnsupportedRecoveryState::pending_actions` (a different type), not `RecoveredPendingAction`; no Flux update needed.
- `kani/` — no `RecoveredPendingAction` Kani harness observed; no new harness required for this bead.
- `proptest/` — `RecoveredPendingAction` is a deterministic struct of two newtype fields; proptest would be tautological with `Arbitrary` deriving. Optional but not required for the bead.