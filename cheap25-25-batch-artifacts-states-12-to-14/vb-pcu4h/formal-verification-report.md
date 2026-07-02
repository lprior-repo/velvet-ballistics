# Formal Verification Report — vb-pcu4h

STATUS: APPROVED

## Scope

- Bead: `vb-pcu4h` — Tests: assert pending-action recovery fields exactly (P1 bug)
- Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`
- JJ workspace root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`
- JJ change under verification: `tlmuzmvk 85e69302 vb-pcu4h: p11-holzman-rust — assert pending-action recovery fields exactly`
- Parent commit: `lzmznkmm 97102739 (empty) (no description set)` — empty parent on top of `rsvywymk 1d6c017f (AGENTS.md round10 forward-port)`.
- Production files (forbidden to mutate per bead scope):
  - `crates/vb_storage/src/recovery/types.rs` (defines `RecoveredPendingAction` at lines 644-650)
  - `crates/vb_storage/src/recovery/replay/summary/derive.rs` (lines 69-73, 287-296)
  - `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` (lines 35, 68)
- Production files touched by this change: **none** (`jj diff -r @ --summary` shows only `crates/vb_storage/src/recovery/replay/summary/tests.rs`).
- Modified file: `crates/vb_storage/src/recovery/replay/summary/tests.rs` (only).
- Bead scope: TEST-ONLY — three assert-region rewrites (`unresolved_action_marks_pending_action_recovery_unsupported`, `action_scheduled_ticket_advances_max_slot_and_step_dimensions`, `crash_after_schedule_then_recover_hydrates_resume_queue`) plus one import line addition (`RecoveredPendingAction` re-exported via existing glob at line 2).

## Verifier Lanes

Per `proof-strategy.md` and `verifier-lane-decisions.jsonl`, the required lanes are **cargo-test** and **source-lint**. The remaining lanes (verus, kani, flux, proptest, loom, miri, fuzz) are explicitly `not_applicable` per bead scope and recorded in `verifier-lane-decisions.jsonl` — these are *lane-not-applicable decisions* (the proof-plan-reviewer disposition is `accepted`), **not** behavior-affecting waivers; therefore `formal-waivers.jsonl` is empty per the user's deliverable instruction.

| Lane | Decision | Source |
|---|---|---|
| cargo-test | required | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003/005/006-cargo-test` |
| source-lint | required | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003/005/006/007/008-source-lint`, `VLD-vb-pcu4h-seed-007-drift-gate` |
| verus | not_applicable | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003-verus` |
| kani | not_applicable | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003-kani` |
| flux | not_applicable | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003-flux` |
| proptest | not_applicable | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003-proptest` |
| loom | not_applicable | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003-loom` |
| miri | not_applicable | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003-miri` |
| cargo-fuzz | not_applicable | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-001/002/003/008-fuzz` |
| SECONDARY uplift (seed-004) | required_if_applied | `verifier-lane-decisions.jsonl::VLD-vb-pcu4h-seed-004-cargo-test` — **not applied** for this bead; SEC-04 deferred per `delivery-scope.jsonl::optional-modify` and prior contract decision |

## Pre-Flight Gates (MANDATORY)

### Verus production-binding gate

```bash
bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h
```

Output:
```
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 71
  VACUUM (no production binding):  0
```

Exit code: `0` — **PASS**. `VACUUM=0` confirms no Verus spec is detached from production code. This bead's relevant mirror `verification/verus/production_inner/replay_invariants_production.rs:253-256` continues to mirror production `crates/vb_storage/src/recovery/types.rs:644-650` byte-for-byte (no edit to either side). **No VACUUM blocker.**

### Mirror drift gate

```bash
bash scripts/check-production-inner-drift.sh
```

Output:
```
Mirror files checked:  60
Extern files scanned:  73
Drift findings:        12
Log:                   target/verus-drift/drift.log
PRODUCTION-INNER DRIFT DETECTED. See target/verus-drift/drift.log
```

Exit code: `1` — **drift detected**, but **NOT in this bead's scope**.

Investigation of `target/verus-drift/drift.log`:

- 12 findings total.
- Findings reference `verification/verus/extern_run_frame_invariant.rs`, `extern_storage_kind_family.rs`, `extern_vb_jnz9_journal_event_seq_valid.rs`, `extern_vb_jpq724_events_for_run_production.rs`, `extern_vb_rpch_seed_dimensions.rs`, plus mirror gaps for `unsupported_recovery_state_production.rs` and `action_replay_tracker_production.rs`.
- **None** of the 12 findings reference the `RecoveredPendingAction` struct at `replay_invariants_production.rs:253-256` (the mirror tied to this bead's `RecoveredPendingAction` claim).
- The drift findings concern other unrelated types: `StepIdx`, `ActionId`, `RunId`, `FrameSeed`, `next_seq`, `validate_replayed_event`, `RecoveredStepState`, `MirrorRecoveryFrameSeed`, `MirrorRecoveryError::FrameDimensionOverflow`, `ActionReplayTracker::mark_completed`, etc.
- `jj log -r '@-' --no-graph -T 'commit_id.shortest(8) ++ " | " ++ description'` returns `97102739 |` (empty parent description), confirming the drift findings pre-exist on the parent commit, NOT introduced by this bead.

Classification: **`BLOCK_GLOBAL`** prerequisite repair per the Holzman `scope_aware_blocking` rule. The 12 drift findings exist on the parent commit and are not in this bead's `jj diff` (which touches only `crates/vb_storage/src/recovery/replay/summary/tests.rs`). This bead is test-only and does not own the production mirror drift repair. Per `contract.md::OUT-OF-SCOPE`: "All mirrors under `verification/verus/production_inner/**` (drift gate runs as gate only)."

### Tooling health

- `cargo` (nightly-2026-04-28, active per `rust-toolchain.toml`): available
- `rustup run nightly-2026-04-28 cargo`: available
- `moon` v2.2.4: available
- `bash scripts/check-verus-production-binding.sh`: exit 0
- `bash scripts/check-production-inner-drift.sh`: exit 1 (BLOCK_GLOBAL, see above)
- `cargo check -p vb_storage --lib`: exit 0
- `cargo test -p vb_storage --lib -- --nocapture ...`: exit 0
- `cargo test -p vb_storage --lib recovery`: exit 0
- `cargo test -p velvet-ballistics-workspace-tests`: exit 101 (1 pre-existing BLOCK_GLOBAL failure, see below)

## Executed Obligations

| ID | Command | Status | Evidence artifact |
|---|---|---|---|
| PO-VBPCU4H-001 | `cargo test -p vb_storage --lib -- --nocapture unresolved_action_marks_pending_action_recovery_unsupported action_scheduled_ticket_advances_max_slot_and_step_dimensions crash_after_schedule_then_recover_hydrates_resume_queue` | **PASS** | `raw_evidence/three_strengthened_tests.log` (3 passed; 0 failed; 0 ignored; 1527 filtered out) |
| PO-VBPCU4H-002 | `cargo test -p vb_storage --lib recovery` | **PASS** | `raw_evidence/vb_storage_recovery_tests.log` (250 passed; 0 failed; 0 ignored; 1280 filtered out) |
| PO-VBPCU4H-003 | `cargo fmt -p vb_storage --check` | **PASS** | `raw_evidence/cargo_fmt_check.log` (no diff for `vb_storage`) |

Per `proof-obligations.planned.jsonl`, three PRIMARY obligations (PO-VBPCU4H-001, -002, -003) were planned with `verifier: cargo-test`. The seed-001 source-lint obligation and seed-007 drift-gate obligation fold under `moon run :lint-src` (which executed) and the manual binding/drift gate runs above; both are tracked in `verification-ledger.jsonl` per their `verifier-lane-decisions.jsonl` rows.

Closure summary:

- 3 of 3 planned cargo-test obligations PASS.
- 0 obligations FAIL_LOCAL.
- 0 obligations FAIL_REGRESSION.
- 0 obligations FAIL_GLOBAL *for this bead's scope*.
- 0 obligations WAIVED.
- **Behavior-affecting waivers**: 0 (the 6 non-applicable verifier lanes — verus, kani, flux, proptest, loom, miri, fuzz — are recorded as `not_applicable` decisions in `verifier-lane-decisions.jsonl`, accepted by `proof-plan-reviewer`; no `formal-waivers.jsonl` row is needed because the lanes never advanced to `required` status).
- 2 BLOCK_GLOBAL pre-existing findings (mirror drift, workspace_tests strict admission) are recorded as **NOT-IN-SCOPE** for this bead — see "Pre-Existing Global Findings" below.

## Source-Lint Sub-Gates (Implementation-Backed)

These sub-gates were executed by the prior `holzman-rust` State 11 and re-verified here:

| Sub-gate | Command | Status | Notes |
|---|---|---|---|
| `cargo check` (vb_storage) | `cargo check -p vb_storage --lib` | PASS | `raw_evidence/cargo_check.log` — `Finished dev profile`, exit 0 |
| `cargo fmt -p vb_storage --check` | `cargo fmt -p vb_storage --check` | PASS | `raw_evidence/cargo_fmt_check.log` — no diff for `vb_storage` |
| Workspace-wide `cargo fmt --all --check` | `cargo fmt --all -- --check` | DEFERRED_GLOBAL | 4 pre-existing failures in `crates/vb_core/src/lib.rs:26`, `crates/vb_core/src/time.rs:71`, `crates/vb_runtime/src/frame_pool/tests.rs:114, 139` — pre-exist in parent commit, unrelated to vb-pcu4h |
| Workspace source lint | `moon run :lint-src` | PASS (this bead's touched file) | `raw_evidence/lint_src.log` — moon lint completed in 33s, no warnings/errors in `tests.rs`; full workspace test-clippy noise (`restate_timer_deadline_primitive_tests.rs` etc.) is pre-existing lint debt |
| `check-source-length.sh` | `bash scripts/check-source-length.sh` | PASS (this bead's touched file) | `raw_evidence/source_length_check.log` — the touched `crates/vb_storage/src/recovery/replay/summary/tests.rs` is NOT in the FAIL list (821 lines, well under 1500 line limit for `test_in_src`) |

The touched test file (`tests.rs`) is fmt-clean (per `cargo fmt -p vb_storage --check` exit 0), cargo-check-clean, and free of new clippy violations introduced by this bead.

## Test Outcome Analysis

### PO-VBPCU4H-001 — Three PRIMARY strengthened tests

All 3 tests in `tests.rs` pass with 1527 filtered out. The previously-fuzzy assertions (`Vec::iter().any(|entry| entry.step == X && entry.action == Y)` and `matches!(seed, Ok(recovered) if <bool>)` outer patterns) have been replaced with:

1. `unresolved_action_marks_pending_action_recovery_unsupported` (lines 436-462):
   - First, the recovery call uses `.expect("schedule-only event must produce a recoverable seed")` (replacing the silent-pass `matches!` outer pattern) — semantic improvement: an `Err(_)` return now panics with a named message rather than silently passing.
   - Then `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }], ...)` — exact Vec equality.
   - Then `assert!(recovered.unsupported.pending_actions, ...)` — preserved from prior (boolean derivation path remains exercised).

2. `action_scheduled_ticket_advances_max_slot_and_step_dimensions` (lines 628-682):
   - Existing `.expect("schedule-only event must produce a seed")` retained.
   - Existing `slot_count == 10`, `step_count == 6`, step 5 Running, `summary.actions_scheduled == 1` assertions preserved.
   - New `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(5), action: ActionId::new(11) }], ...)` — exact Vec equality.

3. `crash_after_schedule_then_recover_hydrates_resume_queue` (lines 752-821):
   - Existing `.expect("post-schedule crash must produce a recoverable seed")` retained.
   - Existing `slot_count == 9`, `step_count == 7` assertions preserved.
   - Existing redundant `let _ = frame_recovery;` second recovery call preserved.
   - Existing live-frame hydration comment (lines 813-816) preserved.
   - New `assert_eq!(seed.pending_actions, vec![RecoveredPendingAction { step: StepIdx::new(6), action: ActionId::new(17) }], ...)` — exact Vec equality.

Per `contract.md::POST-001/002/003`, all three tests now satisfy the exact-Vec-equality + boolean-flag-preserved shape that closes the audit's three failure modes (drop-all, phantom-duplicate, field-drift).

### PO-VBPCU4H-002 — All vb_storage recovery tests (250 passed)

All **250** tests in `vb_storage --lib recovery` pass with 0 failures and 0 ignored. The 250 count proves zero collateral impact on the broader `vb_storage` recovery surface (recover_runtime_summary_reads_summary_from_journal, recover_all_incomplete_runs_*, verify_digests_*, snapshot_tail_*, replay_journal_wrapper_uses_recovery_replay, ppi_003_no_recovery_data_for_nonexistent_run, etc.). The strengthened tests in PO-VBPCU4H-001 are a subset of these 250. **No regression** at the production crate level.

### PO-VBPCU4H-003 — Format gate

`cargo fmt -p vb_storage --check` exits 0 — `vb_storage` is fmt-clean. The 4 workspace-wide fmt failures (`crates/vb_core/src/lib.rs:26`, `crates/vb_core/src/time.rs:71`, `crates/vb_runtime/src/frame_pool/tests.rs:114, 139`) are pre-existing on the parent commit and unrelated to vb-pcu4h's edits.

## Pre-Existing Global Findings (NOT-IN-SCOPE)

These findings pre-exist on the parent commit and are explicitly classified as `BLOCK_GLOBAL` prerequisite repair, NOT introduced by this bead:

### 1. Mirror drift gate (12 findings)

Findings: see `target/verus-drift/drift.log`. None reference `RecoveredPendingAction` or `crates/vb_storage/src/recovery/replay/summary/tests.rs`. Drift findings are in:
- `verification/verus/extern_run_frame_invariant.rs` — 7 findings, all in `crates/vb_core/src/frame.rs`
- `verification/verus/extern_storage_kind_family.rs` — 2 findings in `crates/vb_storage/src/codec/mod.rs`
- `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` — 1 finding in `crates/vb_core/src/ids/mod.rs`
- `verification/verus/extern_vb_jpq724_events_for_run_production.rs` — 1 finding in `crates/vb_storage/src/codec/mod.rs`
- `verification/verus/extern_vb_rpch_seed_dimensions.rs` — 2 findings in `crates/vb_core/src/ids/mod.rs` and `crates/vb_storage/src/recovery/types.rs` (neither in the `RecoveredPendingAction` definition range 644-650)

Verification command: `bash scripts/check-production-inner-drift.sh` (exit 1).

Classification: `BLOCK_GLOBAL` prerequisite repair. Out of scope for this test-only bead.

### 2. workspace_tests pre-existing failure

Test: `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`
Location: `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs:1466`
Panic: `assertion 'left == right' failed; left: false, right: true`

This test asserts that `crates/vb_runtime/src/admission.rs` contains the string `"impl AcceptedArtifactStore for AlwaysPresentArtifactStore"`. Direct grep confirms: `grep "impl AcceptedArtifactStore for AlwaysPresentArtifactStore" crates/vb_runtime/src/admission.rs` returns no match — the string is not present in admission.rs (only a doc-comment reference in line 17). This is a pre-existing repo-wide regression test failure that does not involve recovery pending actions, `RecoveredPendingAction`, or any file in this bead's `jj diff`. Pre-exists on parent commit `lzmznkmm 97102739`.

Verification command: `cargo test -p velvet-ballistics-workspace-tests` (exit 101, 1 failed of 21 in `vb_qi37_4_2_strict_runtime_admission.rs`).

Classification: `BLOCK_GLOBAL` prerequisite repair. Out of scope for this test-only bead.

## Trusted-Base Verification

| Trusted surface | Status | Evidence |
|---|---|---|
| `cargo test` runner | healthy | `cargo test` exits 0 across PO-VBPCU4H-001, -002 |
| `cargo +nightly check` for vb_storage | exit 0 | `raw_evidence/cargo_check.log` |
| `cargo fmt -p vb_storage --check` | exit 0 | `raw_evidence/cargo_fmt_check.log` |
| `assert_eq!` macro (std) | healthy | standard library; no exotic macro path |
| `RecoveredPendingAction` (production, forbidden to mutate) | unchanged | `crates/vb_storage/src/recovery/types.rs:644-650` is pre-existing; `jj diff` shows zero edits to this file |
| `recover_runtime_frame_seed_from_events` (production, forbidden to mutate) | unchanged | `crates/vb_storage/src/recovery/replay/summary/derive.rs:69-73` is pre-existing; `jj diff` shows zero edits |
| `recovered_pending_actions` sort order (production, forbidden to mutate) | unchanged | `crates/vb_storage/src/recovery/replay/summary/derive.rs:287-296` is pre-existing; `jj diff` shows zero edits |
| `accumulator.pending_actions.is_empty()` derivation (production, forbidden to mutate) | unchanged | `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:35,68` is pre-existing |
| `Vec<RecoveredPendingAction>` Vec-equality contract | satisfied | `RecoveredPendingAction` derives `Debug, Clone, Copy, PartialEq, Eq` at `crates/vb_storage/src/recovery/types.rs:644`; literal `vec![...]` equality is sound |
| Verus mirror `verification/verus/production_inner/replay_invariants_production.rs:253-256` | byte-for-byte match | `RecoveredPendingAction` mirror matches production `RecoveredPendingAction` |
| Verus STRONG binding | preserved | `verification/verus/extern_vb_rpch_replay_invariants.rs:191` retains STRONG `#[path = "..."]` |
| Binding gate | exit 0 | `bash scripts/check-verus-production-binding.sh` |
| Drift gate (this bead's mirror) | PASS | `replay_invariants_production.rs:253-256` `RecoveredPendingAction` claim has no drift finding |

## Mapping Status Verification

- All `mapping_status` rows in `proof-obligations.planned.jsonl` for this bead are `planned`; this report closes PO-VBPCU4H-001, -002, -003 as PASS.
- All source/test/harness refs cited in `proof-obligations.planned.jsonl` exist on disk and were inspected:
  - `crates/vb_storage/src/recovery/replay/summary/tests.rs:437-454, 621-672, 743-809` — exists, lines 436-462, 628-682, 752-821 contain the rewritten assert regions.
  - `crates/vb_storage/src/recovery/types.rs:644-650` — exists, `RecoveredPendingAction` struct unchanged.
  - `crates/vb_storage/src/recovery/replay/summary/derive.rs:69-73, 287-296` — exists, production reducer and sort order unchanged.
  - `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:35, 68` — exists, accumulator unchanged.
- All behavior-affecting proof obligations for this bead: **none** (all 3 obligations are `behavior_affecting: false`).
- All `trusted-base-plan.md` dispositions are PASS for this bead's scope; the 12 pre-existing drift findings are explicitly out-of-scope (BLOCK_GLOBAL).
- All `verifier-lane-decisions.jsonl` rows have a final disposition (PASS for 3 cargo-test, `not_applicable` for 21 verus/kani/flux/proptest/loom/miri/fuzz, `required_if_applied` for SECONDARY seed-004 which is **not applied**).

## Findings

- **No blocking findings for this bead.** All 3 cargo-test obligations PASS with zero failures, zero panics, zero ignored tests in the touched scope.
- **No regressions** at `vb_storage --lib recovery` (250/250 passed).
- **No production code mutated** — `crates/vb_storage/src/recovery/types.rs`, `crates/vb_storage/src/recovery/replay/summary/derive.rs`, and `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` are untouched per `jj diff --summary` (only `crates/vb_storage/src/recovery/replay/summary/tests.rs` modified).
- **0 non-behavior waivers** required (formal-waivers.jsonl is empty). The 6 non-applicable verifier lanes (verus, kani, flux, proptest, loom, miri, fuzz) were recorded as `not_applicable` decisions in `verifier-lane-decisions.jsonl` upstream of the formal-verifier stage; they did not advance to `required` status and therefore do not require waiver rows.
- **2 BLOCK_GLOBAL pre-existing findings** (12 mirror-drift findings, 1 workspace_tests strict-admission failure) — pre-exist on the parent commit and are explicitly out of this bead's scope per `contract.md::OUT-OF-SCOPE` and Holzman `scope_aware_blocking`. The touched test file is lint-clean, fmt-clean, and cargo-check-clean.

## Verdict

**APPROVED** — all 3 cargo-test obligations PASS, no production code mutated, no regressions observed, no behavior-affecting waivers, 0 non-behavior waivers, no blocking findings for this bead's scope. Bead is closure-ready for State 13 (black-hat review).
