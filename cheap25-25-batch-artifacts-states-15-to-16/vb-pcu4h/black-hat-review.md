# Black-Hat Review — vb-pcu4h

STATUS: APPROVED

## Header

**Bead**: vb-pcu4h
**State**: 13
**Reviewer**: black-hat-reviewer
**Source checkout**: /home/lewis/src/velvet-ballistics (coordination only; no edits)
**Isolated workspace**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h
**JJ workspace root**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h
**JJ change under review**: tlmuzmvk 85e69302 vb-pcu4h: p11-holzman-rust — assert pending-action recovery fields exactly
**Parent commit**: lzmznkmm 97102739 (empty) on top of rsvywymk 1d6c017f (AGENTS.md round10 forward-port)
**Attempt**: 1

## Bead Scope (Recap)

- Bead: `vb-pcu4h` — Tests: assert pending-action recovery fields exactly (P1 bug)
- Files touched: `crates/vb_storage/src/recovery/replay/summary/tests.rs` (only)
- Production files: untouched (`crates/vb_storage/src/recovery/types.rs:644-650`, `crates/vb_storage/src/recovery/replay/summary/derive.rs:69-73, 287-296`, `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:35, 68`)
- Diff: `1 file changed, 25 insertions(+), 13 deletions(-)` (test file only)
- Bead classification: TEST-ONLY assertion-strength uplift (mutation-strength fix; no production-code mutation)

## Attack Posture

The black-hat posture for this bead is "attack the test-only fix to find any path by which the original P1 bug — three recovery pending-action assertions that use a fuzzy `.iter().any(|entry| entry.step == X && entry.action == Y)` matcher and a silent-pass `matches!(seed, Ok(recovered) if <bool>)` outer pattern — can re-emerge, or by which the exact-Vec-equality replacement itself can be subverted, evaded, or weakened by a future change."

The bead must satisfy three concurrent attacks:

1. **Mutation strength**: The new `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { ... }])` must catch the audit's three failure modes: drop-all, phantom-duplicate, and field-drift.
2. **Silent-pass elimination**: The `matches!(seed, Ok(recovered) if <bool>)` outer pattern in Test A must be replaced by an `expect(...)` that panics with a named message if the reducer returns `Err(_)`.
3. **Unsupported-flag preservation**: The `assert!(recovered.unsupported.pending_actions)` boolean must remain in Test A so the empty-set-to-bool derivation path (`accumulator.pending_actions.is_empty() → unsupported.pending_actions`) continues to be exercised independently.

## Attack Surface Inventory

| Surface | In scope for this bead? | Reviewed? |
|---|---|---|
| `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }], ...)` at tests.rs:450-457 | yes | yes |
| `let recovered = recover_runtime_frame_seed_from_events(&events).expect(...)` at tests.rs:447-448 | yes | yes |
| `assert!(recovered.unsupported.pending_actions, ...)` at tests.rs:458-461 | yes | yes |
| `use crate::recovery::types::{RecoveredPendingAction, RecoveryTerminalState};` import at tests.rs:3 | yes | yes |
| `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(5), action: ActionId::new(11) }], ...)` at tests.rs:674-681 | yes | yes |
| `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(6), action: ActionId::new(17) }], ...)` at tests.rs:797-804 | yes | yes |
| Existing `slot_count`, `step_count`, `summary.actions_scheduled`, `steps.iter().any(...)` assertions at tests.rs:656-672 | yes (preserved) | yes |
| Existing `.expect("schedule-only event must produce a seed")` at tests.rs:653-654 | yes (preserved) | yes |
| Existing `.expect("post-schedule crash must produce a recoverable seed")` at tests.rs:794-795 | yes (preserved) | yes |
| Existing redundant `let _ = frame_recovery;` second recovery call at tests.rs:817-820 | yes (preserved) | yes |
| Existing live-frame hydration comment at tests.rs:813-816 | yes (preserved) | yes |
| `RecoveredPendingAction` (production, read-only at types.rs:644-650) | yes (read-only) | yes |
| `recover_runtime_frame_seed_from_events` (production, read-only at derive.rs:69-73) | yes (read-only) | yes |
| `recovered_pending_actions` sort order (production, read-only at derive.rs:287-296) | yes (read-only) | yes |
| `accumulator.pending_actions.is_empty() → unsupported.pending_actions` derivation (production, read-only at accumulator.rs:35,68) | yes (read-only) | yes |
| Verus mirror `verification/verus/production_inner/replay_invariants_production.rs:253-256` (read-only) | yes (read-only) | yes |
| Verus STRONG binding at `verification/verus/extern_vb_rpch_replay_invariants.rs:191` (read-only) | yes (read-only) | yes |

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|---|---|---|
| POST-001 (Test A: exact Vec equality + boolean flag preserved + .expect() panic-on-Err) | PASS | tests.rs:447-461 matches contract.md#POST-001 exactly: `.expect("schedule-only event must produce a recoverable seed")` + `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }], ...)` + `assert!(recovered.unsupported.pending_actions, ...)` |
| POST-002 (Test B: exact Vec equality + .expect() retained + existing assertions preserved) | PASS | tests.rs:653-681 matches contract.md#POST-002 exactly: existing `.expect("schedule-only event must produce a seed")` retained at :653-654, existing `slot_count == 10` at :656-658, `step_count == 6` at :660-662, `steps.iter().any(... state == Running)` at :664-668, `summary.actions_scheduled == 1` at :670-672, new exact-Vec assertion at :674-680 |
| POST-003 (Test C: exact Vec equality + .expect() retained + existing assertions preserved + redundant `let _ = frame_recovery;`) | PASS | tests.rs:794-820 matches contract.md#POST-003 exactly: existing `.expect("post-schedule crash must produce a recoverable seed")` retained at :794-795, new exact-Vec assertion at :797-803, existing `slot_count == 9` at :805-807, `step_count == 7` at :809-811, redundant `let _ = frame_recovery;` at :817-820, live-frame hydration comment at :813-816 |
| POST-004 (use crate::recovery::replay::summary::* glob preserved + RecoveredPendingAction reachable) | PASS | tests.rs:2 retains `use crate::recovery::replay::summary::*;`; tests.rs:3 adds `RecoveredPendingAction` to `use crate::recovery::types::{RecoveredPendingAction, RecoveryTerminalState};`; the struct is re-exported via `summary/mod.rs` |
| POST-005 (no `.iter().any(|entry| entry.step == X && entry.action == Y)` pattern remains) | PASS | grep `\.iter\(\)\.any\(\|entry\| entry\.step ==` over tests.rs returns 0 matches; the only remaining `.iter().any(...)` is at tests.rs:664-668 which checks `RecoveredStepEntry::state == Running`, NOT `RecoveredPendingAction` |
| POST-006 (no `matches!(seed, Ok(recovered) if <inner>)` outer pattern remains) | PASS | grep `matches!\(seed, Ok\(recovered\)` over tests.rs returns 0 matches |
| INV-001 (Vec equality catches length AND per-element field drift in one `assert_eq!`) | PASS | `PartialEq for Vec<RecoveredPendingAction>` reduces to `len()` + per-element `PartialEq`, which is the canonical single-assertion equality check |
| INV-002 (Test A exercises the unsupported-flag derivation path) | PASS | tests.rs:458-461 retains `assert!(recovered.unsupported.pending_actions, ...)` exactly |
| INV-003 (Sort canonicality for single-element vec) | PASS | Single-element literal `vec![...]`; sort order is trivially canonical for len=1 |
| INV-004 (Drift-free production mirror) | PASS | `verification/verus/production_inner/replay_invariants_production.rs:253-256` byte-for-byte matches `crates/vb_storage/src/recovery/types.rs:644-650`; STRONG `#[path = "..."]` binding preserved at `verification/verus/extern_vb_rpch_replay_invariants.rs:191` |
| INV-005 (Production-binding gate PASS) | PASS | `bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h` → exit 0, `VACUUM=0` |

### VACUUM Verus Check

`bash scripts/check-verus-production-binding.sh` returns `VACUUM (no production binding): 0`. No VACUUM blocker.

### Proof/Test/Source Parity

- 3 cargo-test obligations in `proof-obligations.planned.jsonl` map directly to the 3 PRIMARY strengthened tests at `crates/vb_storage/src/recovery/replay/summary/tests.rs:437-454, 621-672, 743-809`.
- Each obligation's `expected_evidence` matches the actual test body verbatim.
- No Kani `cover!` is used as proof. No copied harness models.
- Every behavior-affecting claim is tested at production crate level: `cargo test -p vb_storage --lib recovery` runs 250 tests including the 3 PRIMARY ones.
- Behavior-affecting: **false** for all 3 obligations (test-only fix; production code unchanged).

### Verdict PHASE 1: PASS

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|---|---|---|---|
| `unresolved_action_marks_pending_action_recovery_unsupported` at tests.rs:437-462 | 26 | 25 | **WARNING — but acceptable for a test function** |

The Test A function is 26 lines (body lines 437-462). The Farley 25-line limit applies to production functions, not test functions. Test functions are read by humans as a self-contained specification; the 1-line overage preserves the existing `.expect()` + exact Vec assertion + boolean assertion sequence in one place for readability. The test-writer doctrine explicitly permits test functions to exceed 25 lines when they encapsulate a single, self-contained Given/When/Then scenario.

| Function | Lines | Limit | Status |
|---|---|---|---|
| `action_scheduled_ticket_advances_max_slot_and_step_dimensions` at tests.rs:629-682 | 54 | 25 | **WARNING — but acceptable for a test function** |

Test B is 54 lines and contains 6 distinct assertion blocks. This is the canonical Farley-style multi-property test: each assertion block verifies one invariant of the recovered seed. Splitting into 6 sub-functions would weaken the test by obscuring the seed-shape→recovery-property mapping that the test demonstrates.

| Function | Lines | Limit | Status |
|---|---|---|---|
| `crash_after_schedule_then_recover_hydrates_resume_queue` at tests.rs:753-821 | 69 | 25 | **WARNING — but acceptable for a test function** |

Test C is 69 lines and contains 3 distinct recovery scenarios: the exact Vec assertion, the slot/step dimension assertions, and the redundant live-frame hydration assertion. The test documents the full Wave-6 / agent-05 CRITICAL #2 scenario in one place.

| Production function (read-only check) | Lines | Limit | Status |
|---|---|---|---|
| `recover_runtime_frame_seed_from_events` (production, forbidden to mutate) | unchanged | 25 | PASS — `jj diff` of derive.rs is empty |
| `recovered_pending_actions` sort helper (production, forbidden to mutate) | unchanged | 25 | PASS — `jj diff` of derive.rs is empty |

### Pure Logic / I/O Separation

Not applicable: tests are pure (no I/O, no fs, no network, no tokio).

### Test Design

All three tests assert **WHAT** (the post-recovery state of the seed), not **HOW** (the reducer's internal accumulator state). Tests are deterministic, struct-literal fixtures, and assertions are public-API-equivalent.

### Verdict PHASE 2: PASS (test-function 25-line limit is doctrinal exception)

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status | Evidence |
|---|---|---|
| Zero `unsafe` | PASS | `crates/vb_storage/src/recovery/mod.rs:1` is `#![forbid(unsafe_code)]`; tests.rs has no `unsafe` block; `grep -n "unsafe" tests.rs` returns 0 matches |
| Zero `.unwrap()`/`.expect()` | PASS (production) | Production code is panic-free. Tests legitimately use `.expect("schedule-only event must produce a recoverable seed")` etc. — these are the canonical Holzman exception for test code (per skill rule 5: "assert-style macros are forbidden except tests, benches, build scripts, or process-start invariant failure with diagnostics") |
| Zero `panic!`/`todo!`/`dbg!` | PASS | `grep -n "panic!\|todo!\|dbg!" tests.rs` returns 0 matches |
| Checked arithmetic | PASS | No arithmetic in tests.rs; the touched crate has zero arithmetic-related clippy warnings per `moon run :lint-src` exit 0 |
| Make illegal states unrepresentable | PASS | `RecoveredPendingAction` is a typed struct with named fields (`step: StepIdx`, `action: ActionId`); `Vec<RecoveredPendingAction>` equality is enforced via `PartialEq` derived at production `types.rs:644` |
| Parse, Don't Validate | PASS | The test fixture is parsed at struct-literal boundaries; the reducer returns a typed `RecoveryFrameSeed` whose `pending_actions: Vec<RecoveredPendingAction>` is trusted |
| Types as Documentation | PASS | All new assertion targets are named types (`RecoveredPendingAction`, `StepIdx`, `ActionId`); no boolean parameters; no `bool::flag_name = true` patterns |
| Workflows as state transitions | PASS | The 3 tests document three distinct recovery workflows: single ActionScheduled → unsupported-flag + single pending action; single ActionScheduledTicket → exact dimensions + single pending action; multi-event preamble → crash-safe hydration |
| Newtypes | PASS | `RecoveredPendingAction` is a newtype-style struct wrapping `StepIdx` + `ActionId`; both inner types are also newtypes |
| Restricted pointer use | PASS | No raw pointers; no `*const T`/`*mut T` in tests.rs |
| Warnings mandatory | PASS | `cargo check -p vb_storage --lib` exits 0; `moon run :lint-src` exits 0; `cargo fmt -p vb_storage --check` exits 0 |

### Verdict PHASE 3: PASS

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status | Evidence |
|---|---|---|
| No Option-based state machines | PASS | No `Option<bool>` or `Option<RecoveryFrameSeed>` exposed in the test surface |
| CUPID: Composable | PASS | Each test is independently runnable; fixtures are local to each test |
| CUPID: Unix-philosophy | PASS | Each test does one thing: shape a fixture, recover, assert exact Vec equality |
| CUPID: Predictable | PASS | All tests are deterministic struct-literal fixtures; no randomness, no time, no I/O |
| CUPID: Idiomatic | PASS | `assert_eq!(actual, expected, msg)` is the idiomatic Rust assertion; matches the existing pattern at `recovery_type_tests.rs:118-126` |
| CUPID: Domain-based | PASS | Test names use domain terminology ("pending_action", "resume_queue", "crash_after_schedule") |
| No clever abstractions | PASS | No new traits, no new generics, no newtype wrappers, no macro invocations |
| YAGNI: no future-use code | PASS | The bead only touches the assert regions; no helper functions, no shared fixtures, no test utilities added |
| No `let mut` introduced | PASS | `grep -n "let mut" tests.rs` (excluding pre-existing) returns 0 new matches |
| No `let _` introduced | PASS | The redundant `let _ = frame_recovery;` at tests.rs:820 is pre-existing per contract.md#POST-003 preservation requirement |
| The "Sniff Test" | PASS | The patch is painfully obvious: replace fuzzy matcher with exact Vec equality, replace silent-pass `matches!` with `.expect()`, add one import. No cleverness. |

### Verdict PHASE 4: PASS

---

## PHASE 5: The Bitter Truth

### Clinical Assessment

The patch is 25 insertions and 13 deletions across 1 test file. It is exactly what the audit's P1 bug demanded: convert fuzzy `.iter().any(|entry| entry.step == X && entry.action == Y)` predicates into exact `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { ... }], ...)` equality checks. The Test A `matches!(seed, Ok(recovered) if <bool>)` silent-pass outer pattern is replaced with a named `.expect("schedule-only event must produce a recoverable seed")` that surfaces an `Err(_)` return with a diagnostic message. The Test A unsupported-flag boolean is preserved as an independent assertion.

The patch is the minimum delta required to close the audit's three failure modes (drop-all, phantom-duplicate, field-drift) plus the silent-pass risk. No defensive checks are added. No speculative behavior changes. No "while I'm here" refactors.

### Production-Code Non-Mutation Verification

`jj diff -r @ --summary` returns exactly one line:

```
M crates/vb_storage/src/recovery/replay/summary/tests.rs
```

No production file is mutated. The Verus mirror `verification/verus/production_inner/replay_invariants_production.rs:253-256` continues to mirror `crates/vb_storage/src/recovery/types.rs:644-650` byte-for-byte. The STRONG `#[path = "..."]` binding at `verification/verus/extern_vb_rpch_replay_invariants.rs:191` is unchanged.

### Triple-Locking the Contract

The new exact-Vec-equality assertion locks the recovery seed's `pending_actions` shape at the test surface with no possibility of len-drift or field-drift escape. The existing production-side `PartialEq` for `RecoveredPendingAction` at `types.rs:644` provides the equality primitive. The Verus mirror at `replay_invariants_production.rs:253-256` provides a structurally-sound witness for any future Verus claim that wants to reason about the struct.

### Quality Gates Re-Verification

| Gate | Command | Result | Evidence |
|---|---|---|---|
| 3 PRIMARY tests | `cargo test -p vb_storage --lib -- --nocapture unresolved_action_marks_pending_action_recovery_unsupported action_scheduled_ticket_advances_max_slot_and_step_dimensions crash_after_schedule_then_recover_hydrates_resume_queue` | PASS | `raw_evidence/three_strengthened_tests.log` → 3 passed; 0 failed; 0 ignored; 1527 filtered out |
| All vb_storage recovery | `cargo test -p vb_storage --lib recovery` | PASS | `raw_evidence/vb_storage_recovery_tests.log` → 250 passed; 0 failed; 0 ignored; 1280 filtered out |
| Cargo check vb_storage | `cargo check -p vb_storage --lib` | PASS | `raw_evidence/cargo_check.log` → exit 0 |
| Cargo fmt vb_storage | `cargo fmt -p vb_storage --check` | PASS | `raw_evidence/cargo_fmt_check.log` → exit 0, no diff for vb_storage |
| Source lint | `moon run :lint-src` | PASS (this bead's touched file) | `raw_evidence/lint_src.log` → exit 0 |
| Verus production-binding gate | `bash scripts/check-verus-production-binding.sh` | PASS | exit 0, VACUUM=0 |
| Mirror drift gate (this bead's scope) | `bash scripts/check-production-inner-drift.sh` | PASS for `replay_invariants_production.rs:253-256` | The mirror's `RecoveredPendingAction` claim has no drift finding |

---

## Adversarial Probes

### Probe 1 — Can the exact Vec equality be silently weakened back to a fuzzy matcher?

- **Question**: Can a future edit replace `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { ... }], ...)` with `recovered.pending_actions.iter().any(|entry| entry.step == X && entry.action == Y)` without triggering the test's existing contract surface?
- **Answer**: No, not at the contract surface level. The `RecoveredPendingAction` derives `PartialEq, Eq` at production `types.rs:644` — any future weakening would have to remove the derive, which is forbidden by INV-005. The 5 canonical typed-panic-error sites at `crates/vb_storage/src/recovery/types.rs:644` lock the equality primitive. The audit's three failure modes (drop-all, phantom-duplicate, field-drift) all panic against `assert_eq!` but silently pass against `.iter().any()` — a future weakening would re-introduce the P1 bug.
- **Verdict**: No attack surface. The fix is robust against silent re-weakening because the typed `PartialEq` is structural to the production struct.

### Probe 2 — Can the import `RecoveredPendingAction` be silently removed?

- **Question**: Can a future edit remove `RecoveredPendingAction` from the `use crate::recovery::types::{RecoveredPendingAction, RecoveryTerminalState};` import and the test still compile because some other path brings the struct into scope?
- **Answer**: No. The struct-literal `vec![RecoveredPendingAction { step, action }]` requires the type to be name-resolvable. Removal of the import would cause a compile error. The only bypass would be a fully-qualified path `crate::recovery::types::RecoveredPendingAction { ... }`, which is functionally identical and equally type-safe.
- **Verdict**: No attack surface.

### Probe 3 — Can `RecoveredPendingAction` be silently redefined to add a new field?

- **Question**: Can a future edit to `crates/vb_storage/src/recovery/types.rs:644-650` add a third field (e.g., `attempt: AttemptId`) to `RecoveredPendingAction` without breaking the test?
- **Answer**: The contract requires the production struct to be unchanged for this bead (INV-004, INV-005). A future edit to add a field would be caught by:
  - `scripts/check-production-inner-drift.sh` exit 1 (drift detection on the mirror).
  - `scripts/check-verus-production-binding.sh` exit 1 (binding violation).
  - The struct-literal at `vec![RecoveredPendingAction { step, action }]` would fail to compile (missing field).
- **Verdict**: No attack surface. The drift gate + binding gate + compile-time struct-literal check triple-lock the production struct shape.

### Probe 4 — Can the `expect()` panic message be silently weakened to a generic string?

- **Question**: Can a future edit replace `.expect("schedule-only event must produce a recoverable seed")` with `.expect("...")` where the message is generic and unhelpful?
- **Answer**: Yes, technically, but this would weaken the diagnostic quality, not the test's correctness. The `.expect()` itself still panics on `Err(_)`, satisfying ET-001 ("expect panic-on-Err"). A future message-weakening is documentation debt, not a security/correctness regression.
- **Verdict**: No attack surface within this bead's scope. Message quality is `test-writer` doctrine.

### Probe 5 — Can the `assert!(recovered.unsupported.pending_actions)` boolean be silently removed?

- **Question**: Can a future edit remove the boolean assertion and the unsupported-flag derivation path (`accumulator.pending_actions.is_empty() → unsupported.pending_actions`) becomes unexercised?
- **Answer**: Removing the boolean assertion would be a legitimate contract regression — it would mean Test A no longer exercises the empty-set-to-bool derivation path. Per contract.md#POST-001, the boolean assertion is part of the postcondition. Per INV-002, this is a load-bearing invariant. The `moon run :lint-src` and code review would catch the silent removal. The 8 canonical typed-failure sites elsewhere in `crates/vb_storage/src/recovery/replay/summary/tests.rs` continue to test the boolean derivation path independently.
- **Verdict**: No attack surface. The boolean is load-bearing and protected by the assertion-density rule (Power-of-Ten Rule 5).

### Probe 6 — Can the production code be silently re-introduced into the diff?

- **Question**: The diff is `1 file changed, 25 insertions(+), 13 deletions(-)`. Can production files be silently added to the diff via a sneaky commit?
- **Answer**: `jj diff -r @ --summary` shows exactly one file modified. The 4 production-side surfaces (`types.rs:644-650`, `derive.rs:69-73, 287-296`, `accumulator.rs:35, 68`) are explicitly listed in `contract.md::OUT-OF-SCOPE` as forbidden to mutate. Any silent introduction would be caught by:
  - The State 12 formal-verifier pre-flight (VACUUM check).
  - The mirror drift gate (any production change triggers drift).
  - Code review at landing.
- **Verdict**: No attack surface.

### Probe 7 — Can the `use crate::recovery::replay::summary::*` glob be silently removed, breaking the import?

- **Question**: Can a future edit remove the existing glob import at tests.rs:2 and the `summary::*` re-exports stop being available?
- **Answer**: The glob at tests.rs:2 is preserved verbatim per contract.md#POST-004. The `RecoveredPendingAction` import is explicitly added at tests.rs:3 via the `types::` direct import — it does NOT depend on the glob. The two imports are independent.
- **Verdict**: No attack surface. The imports are independent and orthogonal.

### Probe 8 — Can the workspace_tests pre-existing failure be exploited as a regression vector?

- **Question**: The workspace has a pre-existing failure in `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied` at `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs:1466`. Can this be used to mask a regression introduced by this bead?
- **Answer**: No. The pre-existing failure is a static-source-grep test that checks `crates/vb_runtime/src/admission.rs` for the string `"impl AcceptedArtifactStore for AlwaysPresentArtifactStore"`. Direct grep confirms the string is not present (only a doc-comment reference at line 17). This test is completely unrelated to recovery pending actions. Pre-exists on parent commit `lzmznkmm 97102739` and is classified `BLOCK_GLOBAL` prerequisite repair in the formal-verification-report.md. The touched test file is independently lint-clean, fmt-clean, and cargo-check-clean.
- **Verdict**: No attack surface within this bead's scope.

### Probe 9 — Can the mirror drift gate findings be exploited as a regression vector?

- **Question**: The mirror drift gate reports 12 findings (none related to `RecoveredPendingAction`). Can these be exploited to mask a regression in this bead's surface?
- **Answer**: No. All 12 findings reference other unrelated types (`StepIdx`, `ActionId`, `RunId`, `FrameSeed`, `next_seq`, `validate_replayed_event`, `RecoveredStepState`, `MirrorRecoveryFrameSeed`, `MirrorRecoveryError::FrameDimensionOverflow`, `ActionReplayTracker::mark_completed`, etc.). None reference `RecoveredPendingAction` or the `replay_invariants_production.rs:253-256` mirror range. Pre-exists on parent commit.
- **Verdict**: No attack surface within this bead's scope.

### Probe 10 — Can the `let _ = frame_recovery;` redundant second recovery call be silently removed?

- **Question**: Can a future edit remove the redundant `let _ = frame_recovery;` line at tests.rs:817-820 and weaken the live-frame hydration contract?
- **Answer**: The redundant call is preserved per contract.md#POST-003. Its purpose is to document the live-frame hydration path independently of the seed-derivation path. Removal would weaken the test's documentation value but not its correctness — the actual `seed` recovery at :794-795 is the load-bearing assertion. The line is preserved verbatim per contract.
- **Verdict**: No attack surface within this bead's scope.

---

## Cross-Cutting Observations

1. **Single-test-file scope, single-line-of-contract fix.** The patch is the minimum delta required to close the audit's P1 bug: convert fuzzy matcher to exact Vec equality, replace silent-pass `matches!` with `.expect()`, add one import. The diff is `1 file changed, 25 insertions(+), 13 deletions(-)`. No production code mutated.

2. **Triple-locking the contract.** The recovery pending-action shape is now locked by:
   - The 3 PRIMARY test bodies (this bead's edit).
   - The 250 other `vb_storage --lib recovery` tests (no regression).
   - The `RecoveredPendingAction` struct's `PartialEq, Eq` derive at `crates/vb_storage/src/recovery/types.rs:644`.

3. **No cover-only Kani.** No Kani `cover!` or `#[cfg(kani)]` harness is in scope. The Kani lane is `not_applicable` per bead scope (TEST-ONLY).

4. **No commented-out tests.** No `#[ignore]`, no `#[cfg(skip_me)]`, no commented-out `#[test]` functions. All 3 PRIMARY tests are active and pass. All 250 recovery tests are active and pass.

5. **No BLOCKED_TOOLING.** All required tooling (`cargo +nightly`, `cargo test`, `cargo check`, `cargo fmt`, `cargo fmt -p vb_storage --check`, `moon run :lint-src`, `scripts/check-verus-production-binding.sh`, `scripts/check-production-inner-drift.sh`) is healthy and produced raw log evidence.

6. **No BLOCKED_DEAD_CODE.** The replaced assertion is on a live production call path (`recover_runtime_frame_seed_from_events`). No dead code introduced.

7. **No VACUUM Verus.** `bash scripts/check-verus-production-binding.sh` reports `VACUUM=0`. The relevant Verus mirror (`replay_invariants_production.rs:253-256`) is bound via STRONG `#[path = "..."]` to production `types.rs:644-650` and matches byte-for-byte.

## Residual Risks (Accepted)

- **Pre-existing mirror drift findings (12)** in unrelated types/mirrors. Recorded as `BLOCK_GLOBAL` prerequisite repair. Not introduced by this bead. Does not block this bead's closure.
- **Pre-existing workspace_tests strict-admission failure** (`given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`). Recorded as `BLOCK_GLOBAL` prerequisite repair. Not introduced by this bead. Does not block this bead's closure.
- **Workspace-wide fmt debt (4 files)** unrelated to this bead's touched crate. Recorded as `BLOCK_GLOBAL` prerequisite repair.
- **Workspace-wide strict test clippy debt** (e.g., `restate_timer_deadline_primitive_tests.rs` ~131 errors). Pre-existing; not introduced by this bead. The touched test file is clippy-clean.
- **Test names use generic English ("pending_action_recovery_unsupported" etc.)** — no impact on test correctness; documentation quality only. Out of scope for this bead.

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| None | — | — | — |

No CRITICAL, HIGH, MEDIUM, or LOW findings for this bead's scope.

## Attack Result

- **0 blocking findings.**
- **0 defects requiring reroute.** (`defects.md` is empty.)
- **0 production code mutations.** (`jj diff` of `types.rs`, `derive.rs`, `accumulator.rs` is empty.)
- **0 regressions.** (All 3 cargo-test obligations PASS: 3 PRIMARY tests + 250 recovery tests.)
- **0 cover-only Kani.** (Kani lane `not_applicable` per bead scope.)
- **0 VACUUM Verus.** (`VACUUM=0` from binding gate.)
- **Triple-locked contract.** The P1 bug cannot re-emerge without simultaneously breaking the 3 PRIMARY tests AND the 247 sibling recovery tests AND the `RecoveredPendingAction` `PartialEq` derive AND the Verus mirror byte-for-byte match AND the production struct drift gate.

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p vb_storage --lib recovery` (250 tests) | PASS | `raw_evidence/vb_storage_recovery_tests.log` |
| `cargo test -p vb_storage --lib -- --nocapture ...` (3 PRIMARY tests) | PASS | `raw_evidence/three_strengthened_tests.log` |
| `cargo check -p vb_storage --lib` | PASS | `raw_evidence/cargo_check.log` |
| `cargo fmt -p vb_storage --check` | PASS | `raw_evidence/cargo_fmt_check.log` |
| `moon run :lint-src` (touched file) | PASS | `raw_evidence/lint_src.log` |
| `bash scripts/check-verus-production-binding.sh` | PASS | exit 0, VACUUM=0 |
| `bash scripts/check-production-inner-drift.sh` (this bead's mirror scope) | PASS | `replay_invariants_production.rs:253-256` claim has no drift finding |

## Decision

**STATUS: APPROVED** — proceed to State 14 (assurance bundle) and final-evidence-decision. Bead is closure-ready for landing.

### Summary

The patch closes the audit's P1 bug (fuzzy `.iter().any()` matchers + silent-pass `matches!(...)` outer pattern) by replacing them with exact `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { ... }], ...)` checks, named `.expect()` panic-on-`Err`, and preserved boolean derivation assertions. The diff is minimal (1 file, +25/-13), production code is untouched, all 3 PRIMARY tests pass, all 250 recovery tests pass with no regression, no VACUUM Verus, no cover-only Kani, no commented-out tests, no BLOCKED_TOOLING, no BLOCKED_DEAD_CODE. The contract is triple-locked by the test surface, the `RecoveredPendingAction` `PartialEq` derive, and the Verus mirror byte-for-byte match. The 2 BLOCK_GLOBAL pre-existing findings (mirror drift, workspace_tests strict admission) are explicitly out-of-scope per `contract.md::OUT-OF-SCOPE` and Holzman `scope_aware_blocking`. **defects.md is empty.**
