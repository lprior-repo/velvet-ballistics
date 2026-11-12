# Architectural Drift / DDD Polish Review: vb-qi37.3

STATUS: APPROVED

## Startup citations / precedence

- Read `/home/lewis/.claude/skills/architectural-drift/SKILL.md`: lines 8-10 require `.rs` line-count checks and splitting files over the generic threshold; lines 12-15 require Scott Wlaschin DDD checks for primitive obsession, explicit workflows, and parse-don't-validate; lines 17-22 define refactor/status outputs.
- Read `/home/lewis/.agents/skills/architectural-drift/SKILL.md`: same content observed; agents copy wins on conflict, no conflict found.
- Read `/home/lewis/.claude/skills/scott-ddd-refactor/SKILL.md`: lines 10-19 require illegal states unrepresentable, parse at boundaries, typed specs, explicit workflows, functional core/imperative shell, no primitive obsession, no bool control flags, and explicit domain error taxonomy.
- Read `/home/lewis/.agents/skills/scott-ddd-refactor/SKILL.md`: same content observed; agents copy wins on conflict, no conflict found.
- Read required bead artifacts: `STATE.md`, `delivery-scope.jsonl`, `implementation.md`, `test-suite-review.md`, `black-hat-review.md`, `formal-verification-report.md`, and `regression-diff.md`.

## Scope basis

`jj diff --name-only` shows the bead's changed source/test files are exactly the expected collect/error API surface:

- `crates/vb_core/src/engine/error_routing.rs`
- `crates/vb_core/src/errors.rs`
- `crates/vb_core/src/ids/mod.rs`
- `crates/vb_core/src/lib.rs`
- `crates/vb_runtime/src/collect_tests.rs`
- `crates/vb_runtime/src/engine/drive.rs`
- `crates/vb_runtime/src/engine/types.rs`
- `crates/vb_runtime/src/primitives/collect.rs`
- `crates/vb_storage/src/types.rs`

Known global FORMAT/CLIPPY/`vb_ui_model` debt remains classified `DEFERRED_GLOBAL` under `vb-bkgo` per `regression-diff.md` and `formal-verification-report.md`; no bead-local causality found.

## Commands / scans run

1. `jj diff --name-only` — observed only bead artifacts plus the nine expected source/test files above.
2. `jj diff --stat && jj diff --summary` — observed scoped source deltas: collect/error/runtime/storage changes plus bead artifacts; no unrelated production crate modified.
3. Focus file line-count scan:
   - `crates/vb_runtime/src/primitives/collect.rs`: 733 lines.
   - `crates/vb_runtime/src/engine/types.rs`: 1177 lines.
   - `crates/vb_runtime/src/engine/drive.rs`: 1362 lines.
   - `crates/vb_runtime/src/collect_tests.rs`: 3596 lines.
   - `crates/vb_core/src/errors.rs`: 1477 lines.
   - `crates/vb_core/src/engine/error_routing.rs`: 477 lines.
   - `crates/vb_core/src/ids/mod.rs`: 1083 lines.
   - `crates/vb_core/src/lib.rs`: 73 lines.
   - `crates/vb_storage/src/types.rs`: 167 lines.
4. Function-span scan for changed production surface:
   - `collect.rs`: all changed production functions/helpers measured <= 30 lines except no changed function exceeded 30; key bodies: `collect_start` 28, `collect_next` 27, `hydrate_extra_with_context` 29, `record_lineage` 24, `hydrate_journal_event` 25.
   - `drive.rs`: `drive_deterministic_full` 34, `emit_slot_evidence` 22, extracted helpers 3-17 lines.
   - `engine/types.rs`: `push_slot_written_with_extra` 30.
   - `errors.rs`: collect kind enums 8 and 29 lines. The only >60 function found was an inline test helper/function in `errors.rs`, not production runtime code.
5. Production forbidden-construct scan before inline test modules for `unsafe`, `.unwrap(`, `.expect(`, `panic!`, `todo!`, `unimplemented!`, and `dbg!` — observed 0 production matches in all changed production files. Crate-level `#![forbid(unsafe_code)]` remains present.
6. Bool-control/signature scan — observed 0 changed production function signatures with `bool` parameters.
7. DDD primitive-pattern scan — observed 0 public stringly-ID parameters and 0 bool control flags. One public numeric capacity constructor (`EvidenceCollector::with_capacity(capacity: usize)`) is an infrastructure capacity boundary, not a collect domain identifier/state primitive.
8. Focused black-hat repair regression:
   ```bash
   rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'
   ```
   Observed: exit 0, Nextest run ID `3250a85f-e642-4d73-a06a-5da20278ddbe`, `3 tests run: 3 passed, 1356 skipped`.

## DDD / architectural findings

- Collect page identity and error concepts are typed: `RunId`, `SlotIdx`, `ListId`, `EventSeq`, `CollectPageOrderViolationKind`, and `CollectExtraHydrationFailureKind` carry the domain surface instead of free-form strings.
- Semantic collect lineage is explicit in `CollectStates`/`CollectPageLineage`; duplicate/stale/out-of-order is no longer inferred from allocator adjacency.
- Expected domain failures use explicit `EngineError` variants: `CollectPageOrderViolation`, `CollectExtraHydrationFailed`, and `CollectEvidenceCapacityExceeded`.
- Workflow transitions remain explicit and fail closed: invalid current-page/hydration/capacity paths return typed errors before mutating state or dropping required evidence.
- Functional core / imperative shell split is coherent after black-hat repair: `drive_deterministic_full`, `collect_start`, and `collect_next` are orchestration shells over short validation/planning/write helpers.
- No new production `unsafe`, unwrap/expect, panic/todo/unimplemented/dbg, bool control flags, or stringly domain API was found in the changed production surface.

## Line-count / cohesion decision

Several touched files exceed the generic 300-line file threshold. Under the bead's explicit scope policy, I did not refactor broad pre-existing aggregate modules/tests solely for file length. For the bead-local changed production behavior, the black-hat helper split is real and the changed production functions are small/cohesive (all measured <= 34 lines in the main repair surface; collect-specific functions <= 30 lines). The remaining whole-file length is recorded as structural debt/context, not a State 13 blocker for `vb-qi37.3` because no changed production function/cohesion violation remains and global structural cleanup would exceed this bead's safe scope.

## Edits made

- No source or test edits made.
- This review artifact was added only.

## Decision

State 13 can advance directly to State 14. No rerun from State 8 is required because no source/test files were refactored and no bead-local architectural/DDD blocker remains.
