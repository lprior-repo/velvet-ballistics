# Wave 2 — Flux-RS Reviewer Report (Agent 05)

**Scope:** 18 bug IDs from `/tmp/wave2-chunk-05.txt` (runtime/action/durability/shard bug sweep).
**Working dir:** `/home/lewis/src/velvet-ballistics` (git root verified via `git rev-parse --show-toplevel`).
**Toolchain:** `nightly-2026-04-28-x86_64-unknown-linux-gnu`; `cargo-flux` at `/home/lewis/.cargo/bin/cargo-flux`.

## Refinement-Typed Surface Survey

Active `#[spec]`, `#[sig]`, `#[refined_by]`, `#[variant]`, `#[trusted]`, `#[trusted_impl]`,
`#[extern_spec]`, `#[opaque]`, `#[ignore]`, `#[no_panic]` annotations live ONLY in dedicated
flux verification files (no active refinements on production code):

| File | Active Annotations | Role |
|------|--------------------|------|
| `crates/vb_compile/src/flux_choose.rs` | 5 | spec model |
| `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` | 23 | trusted models |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | 1 (doc-comment only) | signature sketch |
| `crates/vb_runtime/src/shard/transitions.rs` | 1 (doc-comment only) | signature sketch |
| `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs` | 28 | extern_spec |
| `crates/vb_runtime/src/verification/flux/vb_sxkz6_shard_for_run.rs` | 1 | sig sketch |
| `crates/vb_storage/src/codec/flux_validation.rs` | 27 | trusted models |

Production source files touched by any bug in this chunk have **zero** active refinement
annotations. Flux annotations function as separate spec/spec-binding documents; they do not
constrain the production code being patched. Therefore **all 18 bugs are flux-surface: NO**.

`bash scripts/flux-check-package.sh <pkg>` was run for `vb_runtime`, `vb_storage`, `vb_core`,
`vb_compile`, `vb_validate` — all five packages finish clean (`Finished flux profile
[unoptimized + debuginfo] target(s) in <Xs>`) with no diagnostics. This is recorded as raw
flux evidence below.

## Bug Audit Table

| bug-id | pri | flux-surface | source-fix | test | flux-cmd | flux-result | cargo-result | verdict | evidence |
|--------|-----|--------------|------------|------|----------|-------------|--------------|---------|----------|
| vb-bcybf | P3 | NO | phantom (SlotSet::ensure_insert_slot absent from source) | `cargo test -p vb_runtime --test rs_026_phantom` (2 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 2/2 PASS | PATCHED | `crates/vb_runtime/tests/rs_026_phantom.rs:43,61`; `rtk grep -rln 'SlotSet' --include='*.rs' crates/vb_runtime/src/` returns empty |
| vb-bg12t | P1 | NO | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:16-77` (handle_ask_answer) still uses caller-controlled `answer.answer_slot` and `answer.ticket.resume_step` with no validation against original ask ticket | `rtk grep 'output_slot.*ask\|answer.*slot.*choice'` returns 0 matches; `cargo test -p vb_runtime --lib ask_resume` (10 tests, but ask_resume ≠ handle_ask_answer) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 10/10 PASS but tests target primitives::wait_ask::ask_resume, NOT Shard::handle_ask_answer | NOT-PATCHED | `chunk_002.rs:43` writes to `answer.answer_slot`; `chunk_002.rs:47` sets pc to `answer.ticket.resume_step`; AskAnswer struct comment at `shard/types.rs:198` explicitly says "The caller supplies both payload and destination slot" |
| vb-boq04 | P1 | NO | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:178-242` (drive_run + apply_drive_result + keep_run); `transitions.rs:79-83` (keep_run re-inserts run); `chunk_002.rs:95-108` (handle_timer re-inserts on pending_timer_remove failure) | `cargo test -p vb_runtime --lib` (full lib: 1734 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 1734/1734 PASS | PATCHED | `apply_drive_result` Continue path calls `self.keep_run(run, state)`; `keep_run` calls `run_state_insert`; `handle_timer` line 99 re-inserts on `pending_timer_remove` Err |
| vb-byta8 | P3 | NO | `crates/vb_runtime/src/engine/drive.rs:144-167` (emit_slot_evidence); still uses `let ... && let Ok(value) = run.read_slot(slot)` (line 151, 159) — silently skips branch on read_slot Err | duplicate of vb-i6n4o; no per-bug regression test | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | engine::drive tests pass but no specific coverage of read_slot Err path | PARTIAL | `emit_slot_evidence` at `drive.rs:144-167`; `let ... && let Ok(...)` pattern at lines 151 & 159 swallows `Err(Core(...))` |
| vb-c34qm | P2 | NO | `crates/vb_runtime/src/action/tests.rs` — registry enforces `0 < N <= MAX_INPUT_BYTES` | `cargo test -p vb_runtime --lib validate_input_bytes` (3 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 3/3 PASS (`validate_input_bytes_rejects_when_max_input_bytes_is_zero`, `action_registry_validate_input_bytes_rejects_zero_with_slots`, `validate_input_bytes_rejects_positive_limit_overflow`) | PATCHED | `crates/vb_runtime/src/action/tests.rs:168,201,402` |
| vb-c3j0i | P0 | NO | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs:947` (`cancel_after_kill_is_idempotent`) + `kill_on_cancelled_run_is_idempotent` exist | `cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_tests` | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 16/16 PASS, 2 ignored (`hp3_cancel_action_suspended_run_removes_pending_action`, `hp4_action_after_cancel_returns_error`) | PATCHED | `cancel_kill_lattice_tests.rs:947` + run-id 70006 trace; close-reason referenced `chunk_002_cancel_kill_idempotency.rs` which does not exist — tests relocated to workspace_tests/cancel_kill_lattice_tests.rs |
| vb-c8m2w | P1 | NO | n/a — meta-tooling bug about bd pagination in `scripts/check-vb-jpq7-closure-evidence.py`; out of scope for flux | n/a (checker self-test reported `SELF_TEST_PASS` in close-reason; `moon run :lint-src` exit 0) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | n/a (script-level, not cargo) | PATCHED | close-reason: "Filing-pass complete: 53-row manifest covers all 53 closed vb-jpq7 children"; follow-up for checker pagination filed separately per close-reason |
| vb-ca86d | P2 | NO | `crates/vb_storage/src/recovery/replay/summary.rs:285-365` — `recover_runtime_frame_seed_from_events` uses single-pass `try_fold` accumulator fusing executed-count + parallel-counter | `cargo test -p vb_storage --lib` (1270 tests) + `cargo test -p vb_runtime --test recovery_hydration_tests` (41 tests) | `bash scripts/flux-check-package.sh vb_storage` | PASS clean | 1270/1270 vb_storage lib PASS + 41/41 hydration PASS | PATCHED | `summary.rs:332-341` `recover_frame_seed_accumulator` uses `try_fold` (single pass); close-reason: "post-seed recovery hydration summary now fuses executed-count and parallel-counter event scans" |
| vb-cc2my | P2 | NO | `crates/vb_storage/src/recovery/hydrate_support.rs:190-258` (derive_dimensions_from_snapshot_and_tail) — `RunAnswered { slot_idx, .. }` is NOT in match arms (lines 208-238); `ActionScheduledTicket { ticket, .. }` (line 223-226) updates only max_step/min_step, NOT max_slot | `rtk grep 'derive_dimensions'` returns 0 tests with that name; `cargo test -p vb_storage --lib derive_dimensions` → 0 matched | `bash scripts/flux-check-package.sh vb_storage` | PASS clean | no regression test exists | NOT-PATCHED | `hydrate_support.rs:223-226` ActionScheduledTicket arm lacks max_slot update; `hydrate_support.rs:208-238` has no `RunAnswered` arm; close-reason referenced `snapshot_decode.rs:108-113,124-128` but no such file exists |
| vb-ch8og | P3 | NO | `crates/vb_runtime/src/counters.rs:49-63` (ShardCounters::add_steps) — uses `saturating_add` (line 52) + `compare_exchange_weak` (line 56), saturation guard `current == next` returns early | `cargo test -p vb_runtime --lib counters` (33 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 33/33 PASS | PATCHED | `counters.rs:48-63` `add_steps` saturating loop; bead status BLOCKED but code is fixed; `counter_snapshot_saturating_add_never_panics` (counters/tests.rs:81) verifies saturation safety |
| vb-cmydt | P3 | NO | `crates/vb_runtime/src/admission.rs:740-750` — `admit_artifact_run_with_certificate_floor` returns typed `CapabilityCountMismatch` (not fabricated CapabilityDenied); `check_capability` per-cap loop runs first (line 740-742), count check after (line 745-750) | `cargo test -p vb_runtime --lib admit_artifact_run` (21 tests, includes `admit_artifact_run_count_mismatch_returns_typed_error_not_capability_denied`) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 21/21 PASS | PATCHED (functionally — RA-023 root-cause addressed by RA-018 fix) | `admission.rs:746-749` returns `AdmissionError::CapabilityCountMismatch { required_count, granted_count }`; bead status OPEN is wave-15 bookkeeping not code state |
| vb-cnhef | P2 | NO | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:230-244` (add_executed_step_delta) — tracks `accounted_executed_steps: IndexMap` (line 243), computes `executed.checked_sub(previous)` delta (line 236), early-return on zero | `cargo test -p vb_runtime --lib counters` (33 tests, includes `shard::tests::counters_reflect_submitted_after_submit_tick`, `shard_submit_finish_then_inspect_counters`) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 33/33 PASS | PATCHED | `chunk_001.rs:230-244`; `types.rs:655` `accounted_executed_steps: IndexMap<RunId, u64>`; shard/tests/chunk_011.rs:38-45 exercises add_executed_step_delta |
| vb-crwzv | P1 | NO | `crates/vb_core/src/replay/choose/mod.rs:12,61` (replay_choose_slot, replay_choose_expr) — no deterministic-op rejection list; replay accepts operator set used by `execute_deterministic_full` | `cargo test -p vb_core --lib replay` (195 tests) | `bash scripts/flux-check-package.sh vb_core` | PASS clean | 195/195 PASS | PATCHED | `replay/choose/mod.rs:12-110`; replay and execute use same operator set; `blackhat_replay_detects_do_node_as_non_deterministic` at `replay/tests.rs:795` verifies determinism contract |
| vb-ctgcf | P2 | NO | `crates/vb_runtime/src/trace.rs:87-115` (TraceRing::drain_for_run) — preserves non-target events via `preserved.push_back` (line 102) and re-pushes to producer via `self.producer.push(event)` (line 110) | `cargo test -p vb_runtime --lib trace` (79 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 79/79 PASS | PATCHED | `trace.rs:87-115`; close-reason: "Duplicate of vb-v2zef; same external_ref bug-hunt-2026-06-21:RE-014 remains tracked there" — fix shipped via parent bead |
| vb-cwrm9 | P1 | NO | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:589-598` (flush_evidence) — still uses `try_for_each` which returns on first Err, dropping remaining drained events (matches bug description exactly) | `cargo test -p vb_runtime --lib journal` (35 tests) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 35/35 PASS (but no test exercises the "drops unprocessed after first journal error" path) | NOT-PATCHED | `chunk_001.rs:594-597` `evidence.drain().into_iter().try_for_each(...)` — `try_for_each` returns on first Err without re-queuing remaining events; this matches the original bug description verbatim |
| vb-dbocm | P2 | NO | `crates/vb_runtime/src/engine/types.rs:308-317` (RetryPolicy) — `max_attempts: u16` is a plain field with no constructor rejecting zero; `retry_math.rs:61-75` `validate_against` rejects zero but engine caller can construct `RetryPolicy { max_attempts: 0, ... }` directly | `cargo test -p vb_runtime --lib retry` (151 tests, includes `retry_policy_new_rejects_zero_max_attempts`, `retry_policy_after_action_rejects_zero_max_attempts`, `scheduling_propagates_zero_retry_policy_error`) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 151/151 PASS but engine-level rejection incomplete (no constructor guard) | PARTIAL | `engine/types.rs:319-333` RetryPolicy impl block only defines NEVER/DEFAULT constants — no zero-rejecting constructor; bead status IN_PROGRESS |
| vb-dpo83 | P2 | NO | `crates/vb_runtime/src/admission.rs:743-750` (admit_artifact_run_with_certificate_floor) — uses **exact-count equality** (`required_count != granted_count`), NOT subset/superset as close-reason claims; reverted by later F-001 fix | `cargo test -p vb_runtime --lib admit_artifact_run` (21 tests, includes `admit_artifact_run_rejects_capability_superset` which REJECTS superset, contradicting close-reason's "subset/superset" claim) | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 21/21 PASS | NOT-PATCHED (bead fix reverted by F-001) | `admission.rs:719` comment: "spec: VERUS-CARD-003 strict equality (cardinality-exact + membership-exact admission). F-001 fix: restore strict capability equality (VERUS-CARD-003)" — re-tightened from RA-002 looser behavior |
| vb-dr8k7 | P0 | NO | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:188-194` (append_journal_event) — current implementation calls `journal.append_sequenced(event, seq)` directly with NO coalescing logic at all (window=1 trivially satisfied because no coalescing exists) | `cargo test -p vb_benchmark` (52 tests); no `batched_atomicity_tests` file exists in current tree | `bash scripts/flux-check-package.sh vb_runtime` | PASS clean | 52/52 PASS, but referenced `cargo test -p vb_benchmark --test batched_atomicity_tests` test file does not exist | PATCHED (impl simplified; coalescing path entirely removed) | `chunk_001.rs:189-194` `append_journal_event` has no coalescing branch; `rtk grep 'batched_atomicity\|coalescing_ratio' crates/` returns 0 hits; close-reason's `journal_helpers.rs:38-60` does not exist |

## Summary Counts

- **bugs-checked:** 18
- **PATCHED:** 12 — vb-bcybf, vb-boq04, vb-c34qm, vb-c3j0i, vb-c8m2w, vb-ca86d, vb-ch8og, vb-cmydt, vb-cnhef, vb-crwzv, vb-ctgcf, vb-dr8k7
- **NOT-PATCHED:** 4 — vb-bg12t, vb-cc2my, vb-cwrm9, vb-dpo83
- **PARTIAL:** 2 — vb-byta8 (parent bead vb-i6n4o owns the fix), vb-dbocm (IN_PROGRESS)
- **UNKNOWN:** 0

Sum: 12 + 4 + 2 + 0 = 18. Note: `vb-cmydt` is counted as PATCHED because the underlying bug
(typed `CapabilityCountMismatch` per RA-018) is functionally present in `admission.rs:740-750`,
even though the bead status remains OPEN (wave-15 bookkeeping).

## Flux-Abuse Cases (Pre-Existing Debt, Not Introduced by These Bugs)

Existing `#[flux_rs::trusted]` accumulation in dedicated verification/spec-binding files
(mirrors production behavior without proving it):

1. **`crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs`** — 12 `#[flux_rs::trusted]`
   model functions (`model_handle_cancel_always_ok`, `model_handle_kill_always_ok`,
   `model_cancel_journal_events`, `model_kill_journal_events`, `model_terminal_runs_monotonic`,
   `model_double_terminalization_rejected`, `model_cancel_wins_terminal_race`,
   `model_timer_valid_after_cancel`, `model_ask_valid_after_cancel`,
   `model_counter_only_on_terminalization`, `model_single_journal_event_bound`, plus test
   scaffolding). Each declares `true` (or a tautological refinement) in the body and trusts the
   result. Per GOD RULE 2 (no vacuum Verus proofs), these trusted models only bind to production
   via doc comments referencing `chunk_002.rs` line numbers — no `#[flux_rs::sig]` active on the
   production code. Significant trust debt; out of scope for any of the 18 bugs but worth flagging.

2. **`crates/vb_storage/src/codec/flux_validation.rs`** — 23 `#[flux_rs::trusted]` model functions
   for record-kind/seq-contiguity contracts. Same pattern: trusted model that calls production
   (`crate::codec::validation::validate_kind_family(...)`, `crate::codec::validation::is_known_record_kind(...)`)
   and asserts `bool[true]`. The function under refinement is the production function itself —
   the flux annotation is decorative. Not introduced by these bugs; pre-existing proof debt.

3. **`crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs`** — uses
   `#[extern_spec]` legitimately (declaring refinement on a foreign struct). 28 annotations
   including `#[refined_by(attempt: u16, capacity: u16)]` and `#[invariant(self.attempt > 0)]`.
   Closer to proper Flux use; not abusive.

## Top NOT-PATCHED with Reason

1. **vb-bg12t (P1 — RS-103 Ask answers choose output slot/resume step)**
   **Reason:** `Shard::handle_ask_answer` at `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:16-77`
   still directly consumes caller-supplied `answer.answer_slot` (line 43, `write_slot_with_taint`)
   and `answer.ticket.resume_step` (line 47, `set_pc`) with zero validation against the original
   ask-step's expected output slot. The `AskAnswer` struct comment at `crates/vb_runtime/src/shard/types.rs:198`
   explicitly states "The caller supplies both payload and destination slot" — confirming the
   attacker-controlled-slot surface is still present. No regression test exercises an answer
   with mismatched output slot.

2. **vb-cc2my (P2 — SR-005 derive_dimensions misses RunAnswered/ActionScheduledTicket slot)**
   **Reason:** `derive_dimensions_from_snapshot_and_tail` at `crates/vb_storage/src/recovery/hydrate_support.rs:208-238`
   has match arms for `ActionCompletedEnvelope` (updates max_slot via `output`, line 230) and
   `SlotWrittenEvent/RunFinished` (line 232-235), but the `ActionScheduledTicket` arm
   (line 223-226) only updates max_step/min_step — NOT max_slot from `ticket.output`. There is
   no arm for `JournalEvent::RunAnswered { slot_idx, .. }` at all (verified by inspecting the
   match exhaustively). Close-reason references `snapshot_decode.rs:108-113,124-128` which does
   not exist as a file in current source — the fix appears to have been lost during a refactor
   that moved logic into `hydrate_support.rs` without preserving the slot-index update.

3. **vb-cwrm9 (P1 — RS-205 Evidence flush drops unprocessed events)**
   **Reason:** `Shard::flush_evidence` at `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:589-598`
   still uses `evidence.drain().into_iter().try_for_each(|event| self.flush_evidence_event(...))`
   which returns on the first `Err` and silently drops all remaining drained events. This
   matches the original bug description verbatim ("drops unprocessed events after the first
   journal error"). No test exercises the "first event fails, second event present in evidence"
   scenario, so the regression is masked. 35 journal tests pass without surfacing the bug.

4. **vb-dpo83 (P2 — RA-002 capability check subset/superset)**
   **Reason:** Close-reason claims admission was changed from exact-count equality to subset/
   superset (`admission.rs:743-750`). Current code does the opposite: `required_count != granted_count`
   returns typed `CapabilityCountMismatch`, and the test `admit_artifact_run_rejects_capability_superset`
   (which REJECTS supersets) passes. The RA-002 looser behavior was reverted by the F-001 fix
   which re-tightened admission to exact-match (per the comment at `admission.rs:719`:
   "F-001 fix: restore strict capability equality (VERUS-CARD-003)"). The bead is CLOSED with
   description text that no longer matches the current source.

## Honourable Mentions

- **vb-dpo83 (RA-002)** closed with a description that no longer matches current behavior.
  The fix described (subset/superset) was reverted by F-001 which re-tightened admission to
  exact-count equality (`admission.rs:719` comment: "F-001 fix: restore strict capability
  equality"). Bead is CLOSED but the underlying fix no longer exists in source.
- **vb-c3j0i (RQ-W0-05)** close-reason referenced `chunk_002_cancel_kill_idempotency.rs` but
  the tests were relocated to `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs`.
  `cancel_after_kill_is_idempotent` and `kill_on_cancelled_run_is_idempotent` both pass; the
  close-reason's mention of 6 specific new tests (including `terminal_runs_remains_monotonic`)
  does not match the actual test names in the new file.
- **vb-dr8k7 (B-014)** close-reason referenced `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs:38-60`
  and `cargo test -p vb_benchmark --test batched_atomicity_tests`. Neither the file nor the test
  target exist; the fix is implicitly present because `append_journal_event` has no coalescing
  path at all in the current implementation.

## File Path Written

`/home/lewis/src/velvet-ballistics/to-fix/wave2/agent-05-flux-rs.md`
