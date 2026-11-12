# Black-Hat Review Rerun: vb-qi37.3

STATUS: APPROVED

## Doctrine read / precedence

- Read `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md`: contract/bead parity first, Farley hard limits, Holzman typed Rust, DDD simplicity, and explicit file/line findings.
- Read `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md`: same content; no conflict observed. Agents copy wins if a future conflict appears.

## Findings, ordered by severity

- LETHAL: none.
- MAJOR: none.
- MINOR: none.
- Rejection findings: none.

## Files and diff inspected

- Required bead artifacts read: `STATE.md`, `contract.md`, `test-plan.md`, `test-plan-review.md`, `implementation.md`, `test-suite-review.md`, `red-queen-report.md`, `qa-report.md`, `qa-review.md`, `moon-report.md`, `regression-diff.md`, previous `defects.md`, and `test-repair-blackhat.md`.
- Actual changed source files from `jj diff --name-only` are scoped to `vb_core`, `vb_runtime`, and `vb_storage` collect/error API surfaces:
  - `crates/vb_core/src/engine/error_routing.rs`
  - `crates/vb_core/src/errors.rs`
  - `crates/vb_core/src/ids/mod.rs`
  - `crates/vb_core/src/lib.rs`
  - `crates/vb_runtime/src/collect_tests.rs`
  - `crates/vb_runtime/src/engine/drive.rs`
  - `crates/vb_runtime/src/engine/types.rs`
  - `crates/vb_runtime/src/primitives/collect.rs`
  - `crates/vb_storage/src/types.rs`

## Command evidence from this rerun

1. Focused black-hat repair tests:
   ```bash
   rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'
   ```
   Observed: exit 0, Nextest run ID `0f2018ce-414e-4cde-b28a-77ddfc2b83d8`, `3 tests run: 3 passed, 1356 skipped`.

2. Broad collect regression suite:
   ```bash
   rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_
   ```
   Observed: exit 0, Nextest run ID `7cd27d24-372e-4897-afe0-dba4fbea68c0`, `102 tests run: 102 passed, 1257 skipped`.

3. Prior downstream evidence accepted after inspection:
   - State 9 QA `STATUS: PASS`, including focused black-hat repair `3/3`, `collect_next_` `19/19`, hydration/capacity `7/7`, and broad collect `102/102`.
   - State 10 suite review `STATUS: APPROVED`, with repair-test existence scan and weak-pattern scan reporting `0 matches`.
   - Red Queen rerun `STATUS: PASS`, including page lineage `4/4`, capacity `3/3`, hydration `5/5`, and broad collect `102/102`.

## Previous rejection defect verification

### DEFECT-001 — semantic collect lineage: RESOLVED

- Production evidence: `crates/vb_runtime/src/primitives/collect.rs:46-55` adds per-key `CollectPageLineage`; `record_lineage` at `collect.rs:71-94` records collect transition history independent of allocator adjacency; `classify_observed_page` at `collect.rs:140-155` classifies `previous_page` as `Duplicate`, older recorded pages as `Stale`, and unknown pages as `OutOfOrder`.
- Test evidence: `crates/vb_runtime/src/collect_tests.rs:3246-3330` proves an immediate duplicate remains `Duplicate` when unrelated list allocations occur before and after the next page write.
- Command evidence: focused repair tests passed `3/3`; Red Queen page-lineage challenger passed `4/4`.

### DEFECT-002 — capacity-one evidence loss: RESOLVED

- Production evidence: `EvidenceCollector::push_slot_written_with_extra` at `crates/vb_runtime/src/engine/types.rs:133-162` now returns exact `EngineError::CollectEvidenceCapacityExceeded` when required collect extra arrives at full capacity; it does not clear existing evidence.
- Test evidence: `crates/vb_runtime/src/engine/types.rs:1134-1176` asserts capacity-one returns the exact capacity error and drains the preserved prior `StepStarted` event.
- Command evidence: focused repair tests passed `3/3`; Red Queen capacity challenger passed.

### DEFECT-003 — corrupt collect-bearing hydration: RESOLVED

- Production evidence: `collect_page_from_event_value` at `crates/vb_runtime/src/primitives/collect.rs:317-336` maps postcard decode failure to exact `CollectExtraHydrationFailed { kind: DecodeFailed, ... }`; `hydrate_journal_event` at `collect.rs:229-253` propagates that failure before any state insert.
- Test evidence: `crates/vb_runtime/src/collect_tests.rs:3570-3596` asserts corrupt slot value bytes with collect extra return exact `DecodeFailed` with `event_seq: Some(EventSeq(6))` and no state inserted.
- Command evidence: focused repair tests passed `3/3`; Red Queen hydration challenger passed `5/5`.

### DEFECT-004 — Farley/cohesion split: RESOLVED

- Production split is real, not cosmetic:
  - `collect_start` delegates validation/planning to `build_collect_start_plan` (`collect.rs:395-423`), empty finish to `finish_empty_collect_start` (`collect.rs:425-436`), page finish to `finish_collect_start_page` (`collect.rs:438-454`), and state mutation to `upsert_started_collect` (`collect.rs:456-476`). The orchestration body is now small and readable at `collect.rs:353-367`.
  - `collect_next` delegates state validation/page planning to `build_collect_next_plan` (`collect.rs:510-528`), cursor guard to `validate_cursor_in_source` (`collect.rs:530-540`), and terminal write/removal to `write_terminal_collect_page` (`collect.rs:542-553`). The orchestration body is now `collect.rs:488-505`.
  - `drive_deterministic_full` delegates setup to `initialize_drive` (`drive.rs:87-91`), per-step budget/pc/running state to `begin_drive_step` (`drive.rs:93-109`), post-step state/evidence to `finish_drive_step` (`drive.rs:111-124`), and slot evidence to `emit_slot_evidence` (`drive.rs:130-151`). The loop body is now `drive.rs:58-79`.
- I am not issuing a style rejection: the changed production bodies are cohesive shells over typed helpers, with no hidden I/O in calculation helpers and no bead-local panic/unsafe/unwrap path found in the inspected repair surface.

## Contract parity decision

- ERR-004/ERR-005/ERR-006 parity: typed `CollectPageOrderViolation` behavior is covered by exact duplicate/stale/out-of-order tests and semantic lineage implementation.
- ERR-007 parity: corrupt, empty, identity-mismatched, and current-page-mismatched collect extras fail closed with typed `CollectExtraHydrationFailed`; non-list decodable non-collect extras remain skipped as non-collect.
- ERR-008 / INV-009 parity: required collect `SlotWritten.extra` no longer silently disappears on capacity exhaustion; full capacity returns typed `CollectEvidenceCapacityExceeded` and preserves existing evidence.
- POST-008 parity: all invalid page-order tests assert state preservation.

## Deferred global debt decision

- Accepted: State 8 `DEFERRED_GLOBAL` FORMAT/CLIPPY/`vb_ui_model` debt remains outside this bead's changed files and is tracked by `vb-bkgo`.
- Not a State 11 blocker: `moon-report.md` and `regression-diff.md` show failures reproduce on clean main and are not caused by `vb-qi37.3` source/test changes.

## Verdict

State 11 can exit: YES.

Approved. The black-hat repair closed the prior semantic-lineage, evidence-capacity, hydration fail-closed, and cohesion defects with actual production changes plus focused and broad executable evidence. No bead-local rejection ground remains.
