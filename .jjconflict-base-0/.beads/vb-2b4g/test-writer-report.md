# Test Writer Report — vb-2b4g State 8 Repair

## Files changed

- `crates/vb_codegen/src/tests.rs`
- `.beads/vb-2b4g/test-writer-report.md`

## Tests added/replaced

- Replaced shallow Repeat parity with generated-vs-`drive_deterministic_full` full observation parity for:
  - success through `RepeatStart -> RepeatAttempt -> SetConst -> RepeatCheck -> RepeatFinish -> Finish`
  - non-i64 attempt typed-error path
- Replaced shallow Reduce parity with full observation parity for:
  - empty input
  - single input
  - multi input over distinct unrolled `ReduceNext` nodes
  - non-list typed-error path
- Replaced Together parity with full observation parity for:
  - two successful branches with ordered `[10,20]` expectation surface
  - missing-output typed-error path
- Replaced Collect parity with full observation parity for:
  - empty, single-page, and multi-page collection
  - item-limit and page-limit capacity errors
  - duplicate, stale, and out-of-order page typed-error paths using prepopulated runtime/generated collect state
- Replaced journal parity test so it now exercises Repeat, Reduce, Together, and Collect observation signatures rather than action-only boundary journal events.
- Added reusable generated/runtime normalized observation helpers comparing:
  - terminal result/error text
  - final pc
  - slot values with list contents expanded
  - taints
  - step states
  - collect state essentials
  - runtime evidence vs generated journal signatures
  - explicit `not_yet_implemented` fail-fast checks

## Command results

- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` — FAILING_TESTS_WRITTEN
  - Tests compile and execute.
  - Exposes generated journal missing `StepStarted`/`StepSucceeded` evidence parity and runtime error wrapper mismatch.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` — FAILING_TESTS_WRITTEN
  - Tests compile and execute.
  - Exposes generated journal/evidence mismatch and typed error wrapper mismatch.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` — FAILING_TESTS_WRITTEN
  - Tests compile and execute.
  - Exposes generated `TogetherStart` emitted source compile failure: step parameter named `_slot_taints` while body uses `slot_taints`.
  - Missing-output error parity also exposes evidence/wrapper mismatch.
- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` — FAILING_TESTS_WRITTEN
  - Tests compile and execute.
  - Exposes missing generated step evidence, collect typed-error field/wrapper mismatch, and journal signature mismatch.
- `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture` — FAILING_TESTS_WRITTEN
  - Tests compile and execute.
  - Exposes target-family generated journal signatures do not include runtime `StepStarted`/`StepSucceeded` evidence order.

## Implementation failures exposed

- Generated target-family journal does not match runtime evidence signature: runtime emits `StepStarted`, `SlotWritten`, `StepSucceeded`; generated journal mostly emits only slot writes and `RunFinished`.
- Generated typed errors are not normalized to runtime `RuntimeEngineError::Core(...)` shape/fields.
- Generated `TogetherStart` source can fail to compile when output is present because `slot_taints` is referenced while the emitted step argument is `_slot_taints`.
- Collect duplicate/stale/out-of-order errors differ in exact fields: runtime includes `run_id`, typed `SlotIdx/ListId`; generated omits `run_id` and uses raw integers.

## State 8 test-reviewer rejection repair

### Files changed in this repair

- `crates/vb_codegen/src/tests.rs`
- `.beads/vb-2b4g/test-writer-report.md`

### Exact repairs

- Removed `normalize_observation_text` filtering that dropped `journal:RunFinished` evidence.
- Removed `status=Err(Core(...)) -> status=Err(...)` normalization; generated harness status now prints `status=Err(Core(...))` so wrapper shape is compared instead of erased.
- Added runtime terminal `journal:<order>:RunFinished:<step>:<value>:<taint>` evidence derived from `drive_deterministic_full` finished status and the runtime finish node/result-slot taint.
- Updated generated observation printer to include journal event order for generated events and to print `RunFinished` step, value, and taint.
- Replaced Collect duplicate/stale/out-of-order generated setup strings that used `unwrap_or(0)` and ignored `collect_states.upsert` results with fail-fast generated-harness setup helpers. Setup failures print `setup:<case>:<operation>:<error>` and exit non-zero.
- No generated production code was needed.

### Command outcomes

- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` — PASS: `cargo test: 3 passed, 364 filtered out (3 suites, 0.39s)`
- `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture` — PASS: `cargo test: 1 passed, 366 filtered out (3 suites, 0.58s)`
- `rtk cargo test -p vb_codegen -- --nocapture` — PASS: `cargo test: 367 passed (4 suites, 4.02s)`
- `rtk cargo fmt --check` — PASS: no output
- `rtk cargo check -p vb_codegen --all-targets --all-features` — PASS: `cargo build (1 crates compiled)` and `Finished dev profile [unoptimized + debuginfo] target(s) in 0.59s`

### Residual risks

- No mutation run was requested or executed for this repair loop.
- This repair intentionally changed only the parity/test harness and report; pre-existing working-copy changes in other files were not touched by this pass.
