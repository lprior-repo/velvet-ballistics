# verifier-lane-matrix.md — vb-pcu4h

- bead_id: vb-pcu4h
- planner_state: 4
- produced_by: proof-planner (State 4)
- schema_version: verifier-lane-matrix/v1
- machine-readable companion: `verifier-lane-decisions.jsonl` (37 rows)
- planner_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`

## 1. Symbol key

- **R** — required (closure gate must emit `0` and clean PASS evidence)
- **NA** — not_applicable (deferred with concrete evidence refs; per EARS rule, every default-profile verifier with applicability=not_applicable must carry `non_applicability_evidence_refs` with concrete paths)
- **O** — optional / required_if_applied (decision flips to R when the SECONDARY uplift is applied)
- **blank** — not demanded by the seed's risk profile

Default-profile verifiers per the skill (`Default-profile verifiers (Verus, Kani, Flux, proptest) with applicability: not_applicable`) all carry `non_applicability_evidence_refs` in the JSONL companion.

## 2. Seed × verifier matrix

Verifiers across: cargo-test (CT), source-lint (SL), drift-gate (DG), proptest (PT), Verus (VR), Kani (KN), Flux (FL), fuzz (FZ), loom (LM), Miri (MI).

`drift-gate` is the canonical rust project gate runner (`scripts/check-production-inner-drift.sh` and `scripts/check-verus-production-binding.sh`). `source-lint` is the umbrella covering `moon run :lint-src`, `cargo fmt --all -- --check`, and (for the seed-007/008 lanes) the drift-gate pre-flight checks.

### Primary obligation seeds (cargo-test closure)

| Seed | Target site (path::test) | CT | SL | DG | PT | VR | KN | FL | FZ | LM | MI |
|------|--------------------------|----|----|----|----|----|----|----|----|----|----|
| seed-001 | crates/vb_storage/src/recovery/replay/summary/tests.rs:437-454 :: unresolved_action_marks_pending_action_recovery_unsupported | R | R | — | NA | NA | NA | NA | NA | NA | NA |
| seed-002 | crates/vb_storage/src/recovery/replay/summary/tests.rs:621-672 :: action_scheduled_ticket_advances_max_slot_and_step_dimensions | R | R | — | NA | NA | NA | NA | NA | NA | NA |
| seed-003 | crates/vb_storage/src/recovery/replay/summary/tests.rs:743-809 :: crash_after_schedule_then_recover_hydrates_resume_queue | R | R | — | NA | NA | NA | NA | NA | NA | NA |
| seed-004 (optional SECONDARY) | crates/vb_runtime/tests/recovery_hydration_tests.rs:1899-1905 and :2031-2037 | O | O | — | NA | NA | NA | NA | NA | NA | NA |

### Cross-cutting / folded seeds

| Seed | Concern | CT | SL | DG | PT | VR | KN | FL | FZ | LM | MI |
|------|---------|----|----|----|----|----|----|----|----|----|----|
| seed-005 | Err-panic-on-Err (Test A outer pattern rewrite) | R | R | — | NA | NA | NA | NA | NA | NA | NA |
| seed-006 | unsupported-flag preservation (Test A only) | R | R | — | NA | NA | NA | NA | NA | NA | NA |
| seed-007 | Mirror drift gate (production-inner-drift + production-binding) | — | R | R | NA | NA | NA | NA | NA | NA | NA |
| seed-008 | Pure / no hostile input (test fixtures use struct literals) | — | R | — | NA | NA | NA | NA | NA | NA | NA |

## 3. Per-cell rationale summary

### cargo-test (CT) — Required for seeds 001-006

Each row is a single targeted `cargo test -p vb_storage --lib -- --nocapture <test-name>` invocation listed in `contract.md#acceptance-commands`. The expected evidence is the test PANICKED marker per scenario:

- Test A: panic on `expect("…")` if `Err(_)`, panic on Vec-equality if length drift or per-element drift, panic on boolean if flag flips.
- Test B / Test C: panic on Vec-equality (length or per-element), passing assertion site for the existing `slot_count` / `step_count` / `summary.actions_scheduled == 1` invariants.

Source-lint (SL) folds `cargo fmt`; passing implies the new assertion block reads in project style.

### source-lint (SL) — Required for all rows

- `moon run :lint-src` (zero-tolerance source lint) — must exit 0.
- `cargo fmt --all -- --check` — must report no diff.
- Both gates are folded per the contract's "source lint zero tolerance" requirement.

### drift-gate (DG) — Required for seed-007

- `bash scripts/check-production-inner-drift.sh` — checks that `verification/verus/production_inner/replay_invariants_production.rs:253-256` matches production byte-for-byte.
- `bash scripts/check-verus-production-binding.sh` — checks STRONG `#[path]` binding on the Verus mirror.
- Both must exit 0. The bead does not edit either mirror; the gates verify passivity.

### proptest (PT) — not_applicable for all rows

Circular-shape argument: a single-element expected Vec requires constructing exactly the vec that the reducer produces; any property-test harness reduces to the same fixture plus a deterministic comparison, with no incremental coverage gained. The three existing PRIMARY fixtures already exhaust all single-event shapes. Concrete evidence refs in JSONL.

### Verus (VR) — not_applicable for all rows

GOD RULE 4 (formal verification mandates) prohibits altering the mathematical contract to make a test turn green. The bead's audit finding is closed by a test-assertion edit; the production contract is unchanged. Adding a new Verus row would:

1. Require inventing a new production-bound claim — none exists for this bead (production struct unchanged).
2. Force a new STRONG `#[path]` binding or `ALLOWED_EXCEPTIONS` entry, neither of which is meaningful for a test-strength uplift.
3. Drift the verifier profile away from the contract's "test-only" classification.

### Kani (KN) — not_applicable for all rows

The user's forbidden list explicitly forbids new Kani harnesses for this bead. GOD RULE 5 (no blind verification mutations): Kani would be outside the bead's blast radius. Existing `crates/vb_storage/src/recovery/kani.rs` has no `RecoveredPendingAction` harness observed.

### Flux (FL) — not_applicable for all rows

The forbidden list forbids new Flux refinements. Existing Flux refinements (`vb_rpch_flux_r8.rs`, `vb_rpch_flux_r9.rs`) target `UnsupportedRecoveryState::pending_actions:bool`, not `RecoveredPendingAction`. Drift gate covers the surface that already exists.

### fuzz (FZ) — not_applicable for all rows

Test fixtures use Rust struct literals (`JournalEvent::ActionScheduled { ... }`). No parser, codec, or bytes/string boundary exists at the test surface. `fuzz/Cargo.toml` has no `RecoveredPendingAction` target. `seed-008` explicitly classifies hostile-input as not-required.

### loom (LM) — not_applicable for all rows

Recovery reducer is synchronous, idempotent, single-threaded. No shared state, no atomics, no thread spawn at the test surface. `workflow-model.md` and `hazard-analysis.md` confirm.

### Miri (MI) — not_applicable for all rows

`recovery/mod.rs:1` has `#[forbid(unsafe_code)]`. `RecoveredPendingAction` derives `Copy`. The test edit uses safe Rust only. AGENTS.md Engineering Rules forbid `unsafe`.

## 4. Closure-command consolidation

The full closure surface for `moon ci` (canonical):

```bash
# Targeted primary tests
cargo test -p vb_storage --lib -- --nocapture \
    unresolved_action_marks_pending_action_recovery_unsupported \
    action_scheduled_ticket_advances_max_slot_and_step_dimensions \
    crash_after_schedule_then_recover_hydrates_resume_queue

# Optional secondary uplift
cargo test -p vb_runtime --test recovery_hydration_tests -- --nocapture \
    pending_action_persisted_restart_via_appends_with_syncall

# Source lint zero tolerance
moon run :lint-src
cargo fmt --all -- --check

# Mirror drift + binding gates
bash scripts/check-production-inner-drift.sh
bash scripts/check-verus-production-binding.sh

# Canonical moon ci
moon ci
```

## 5. Mapping to proof-writer (State 5) and proof-to-implementation (State 7)

- **State 5 (proof-writer)**: authoring work for this bead is the test edit at `tests.rs:437-454, 621-672, 743-809`. No model/proof/harness file is created. The proof-writer's `expected_evidence` per obligation is captured in `proof-obligations.planned.jsonl`.
- **State 7 (proof-to-implementation)**: bridge map is trivial (no proof-side claims to bridge; the obligation target is the test file). Bridge is captured implicitly in the obligation rows' `artifact` field pointing at the test paths.

No claim of disposition. Approval lives in `proof-plan-reviewer`'s `verifier-lane-review.jsonl` / `proof-plan-review.md` at State 4b.
