# Proof-To-Rust Bridge Map: vb-fzgdn

## Metadata

- **bead**: `vb-fzgdn`
- **refreshed_by**: `vb-hkedh-state11-holzman-rust-20260604`
- **refresh_scope**: document bridge-map refresh only
- **runtime_behavior_claim**: none; this document does not prove runtime behavior
- **source_checkout_for_this_refresh**: `/home/lewis/isolated/go-skill-batch-20260604/vb-hkedh`

## Refresh Summary

This map replaces the stale State 7 closure-truth view with a current document
map for the numeric timer migration evidence now present in this checkout. It
records the numeric seam that exists, the remaining wall-clock `Instant` surfaces
that still exist, and the accepted evidence gaps that must not be inflated into
runtime proof.

The accepted gap artifacts are:

- `.beads/vb-fzgdn/final-evidence-decision.md`
- `.beads/vb-fzgdn/assurance-bundle.md`

F-S12-001 and `.beads/vb-fzgdn/proof-review-state12-waiver.md` are missing,
unavailable, absent, not present, and not imported in this checkout. The accepted
decision is documented by the final decision and assurance bundle above; the
specific F-S12-001 waiver artifact must not be claimed available unless a real
file is later imported.

## Numeric Timer Migration Source Map

| Area | Current source refs | Current truth |
|---|---|---|
| Timer newtypes and remaining wall-clock timer | `crates/vb_runtime/src/shard/timer.rs` | Defines `TimerTick`, `TimerDuration`, and `TimerDeadline` for the numeric seam. Also retains `PendingTimer` with `deadline: Instant`, so wall-clock timer authority remains as a legacy surface. |
| Shard clock field | `crates/vb_runtime/src/shard/config.rs` | `Shard` carries `current_tick`, typed as `TimerTick`, alongside `pending_timers`. |
| Shard initialization | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` | `Shard::new_with_journal_and_artifact_store` initializes `current_tick` with `TimerTick::new(0)`. |
| Numeric clock methods | `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs` | `advance_clock_to` rejects backward movement and `next_pending_timer_generation` uses checked generation advancement. |
| Timer registration transition | `crates/vb_runtime/src/shard/transitions.rs` | `await_timer` still creates `PendingTimer` with `Instant::now`, so this is not a no-`Instant` runtime closure. |
| Timer wheel | `crates/vb_runtime/src/shard/timer_wheel.rs` | `TimerWheel` stores `TimerEntry` values keyed by `Instant`; `next_deadline` returns the next wall-clock deadline. |

## Evidence Commands And Current Result Classes

These are the current evidence commands recorded by the accepted evidence
package and refresh inputs. This document lists evidence classes only; it does
not assert new verifier execution beyond the document-integrity commands run for
`vb-hkedh`.

| Lane | Command or artifact | Current result class | Honesty note |
|---|---|---|---|
| behavior | timer behavior suites summarized in `.beads/vb-fzgdn/assurance-bundle.md` | accepted behavior evidence | Behavior tests are not a formal proof. |
| Flux | `cargo flux -p vb_runtime` | smoke evidence | Flux is a crate-level smoke pass, not per-obligation refinement closure. |
| Verus | `verus --crate-type=lib verification/verus/vb-fzgdn/PS-*-proof.rs` | disconnected under GOD RULE 2 | Verus local models are disconnected from production bindings and must not be cited as production proof. |
| Kani | `cargo kani -p vb_runtime --harness ps_*` | BLOCKED | Harnesses exist but are not discoverable/wired in the crate tree. |
| Proptest | `cargo test -p vb_runtime --test proptest -- ps_*` | BLOCKED | Property files exist but the Cargo test target is missing. |
| Fuzz | `cargo fuzz run ps_006_fuzz -- -max_total_time=300` | BLOCKED | Fuzz target exists; build is blocked by the recorded sanitizer/toolchain environment. |
| Loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib -- timer_fired_cancel` | partial | Loom has a partial model pass; per-obligation Loom evidence remains incomplete. |

The accepted evidence package separates accepted gaps from missing artifacts. A
missing artifact remains missing even when a controller accepts the delivery with
documented gaps.

## Non-Closure Legacy And Hazard Notes

- Legacy `Instant` surfaces are retained in the current source map as hazards and
  compatibility facts, not as closure truth.
- `crates/vb_runtime/src/shard/timer.rs` still imports `std::time::Instant` and
  keeps `PendingTimer.deadline` as `deadline: Instant`.
- `crates/vb_runtime/src/shard/transitions.rs` still calls `Instant::now` in
  `await_timer`.
- `crates/vb_runtime/src/shard/timer_wheel.rs` still indexes `TimerEntry` values
  by `Instant` and exposes `next_deadline`.
- Any root `formal-verification-report.md` mention from older packages is
  wrong-bead or not vb-fzgdn closure truth; do not cite it as proof of this bead.

## Bridge Matrix

| Proof ID | Claim mapped by this document | Behavior affecting | Source refs | Evidence command | Current document status |
|---|---|---|---|---|---|
| POB-001..005 | Deadline arithmetic and no-panic obligations map to both numeric `TimerDeadline` constructors and remaining `Instant` timer-wheel surfaces. | true | `crates/vb_runtime/src/shard/timer.rs`; `crates/vb_runtime/src/shard/timer_wheel.rs`; `crates/vb_runtime/src/shard/transitions.rs` | See behavior, Flux, Verus, Kani, Proptest, Fuzz, Loom table above. | document refreshed; runtime proof not claimed |
| POB-006..010 | Numeric-only timer-state intent maps to `TimerTick`/`TimerDuration`/`TimerDeadline`, `current_tick`, and remaining `PendingTimer` wall-clock hazard. | true | `crates/vb_runtime/src/shard/timer.rs`; `crates/vb_runtime/src/shard/config.rs`; `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`; `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs` | See behavior, Flux, Verus, Kani, Proptest, Fuzz, Loom table above. | document refreshed; runtime proof not claimed |
| POB-011..014 | Authority validation maps to current `PendingTimer` authority fields and compatibility wall-clock kind/deadline matching. | true | `crates/vb_runtime/src/shard/timer.rs`; `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs` | See behavior, Flux, Verus, Kani, Proptest, Fuzz, Loom table above. | document refreshed; runtime proof not claimed |
| POB-015..018 | Generation exhaustion maps to checked numeric generation advancement. | true | `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs`; `crates/vb_runtime/src/shard/timer_wheel.rs` | See behavior, Flux, Verus, Kani, Proptest, Fuzz, Loom table above. | document refreshed; runtime proof not claimed |
| POB-019..046 | Duplicate-key, slot-validation, clock-advance, capacity, zero-duration, and fire/enqueue obligations map to the numeric seam plus remaining wall-clock timer-wheel and transition surfaces. | true | `crates/vb_runtime/src/shard/timer.rs`; `crates/vb_runtime/src/shard/config.rs`; `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`; `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs`; `crates/vb_runtime/src/shard/transitions.rs`; `crates/vb_runtime/src/shard/timer_wheel.rs` | See behavior, Flux, Verus, Kani, Proptest, Fuzz, Loom table above. | document refreshed; runtime proof not claimed |

## vb-hkedh Document Integrity Obligations

The downstream document refresh is checked by:

```bash
python3 .beads/vb-hkedh/test_document_integrity.py
python3 .beads/vb-hkedh/check_document_integrity.py
```

Expected document-integrity result after this refresh: all `PO-HKEDH-DOC-001`
through `PO-HKEDH-DOC-004` checks pass. Passing these checks proves only the
integrity of this evidence map against the checker predicates. It does not prove
runtime behavior, verifier closure, or absence of remaining wall-clock timer
surfaces.
