# Implementation Report — vb-vt2f State 10

## Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Bead inputs read

- `.beads/vb-vt2f/contract.md`
- `.beads/vb-vt2f/proof-review.md`
- `.beads/vb-vt2f/contract-verification-review.md`
- `.beads/vb-vt2f/test-plan.md`
- `.beads/vb-vt2f/test-plan-review.md`
- `.beads/vb-vt2f/test-suite-review.md`
- `.beads/vb-vt2f/test-writer-report.md`

## Code changes mapped to obligations

- `POST-012 / ERR-002 / SCN-VT2F-009`: strict and journaled non-storage runtimes now use `MissingAcceptedArtifactStore`; `Runtime::submit_direct*` validates admission before enqueueing. Missing accepted artifacts now return `RuntimeError::AdmissionArtifactNotFound { digest }` at the submit boundary.
- `POST-009 / ERR-004 / SCN-VT2F-006`: `Ask` now requires the prompt slot to exist but does not reject non-symbol prompt payloads; `handle_ask_answer` writes the answer, clears the pending ask timer, sets the public resume step, and lets deterministic drive finish the run.
- `POST-008 / ERR-003 / SCN-VT2F-005`: wrong/absent-run `fail_action` now returns `RuntimeError::InvalidActionCompletion` and does not mutate unrelated run state.

## Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, or unchecked arithmetic added.
- Admission failure is typed and checked before queue mutation for direct submit APIs.
- Control flow remains direct bounded command processing; no loops or allocations added on the action/ask handling paths.
- Added one typed missing-artifact store instead of a panic or dummy success path for strict non-durable admission.

## Commands run from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

1. `rtk cargo fmt --all` — PASS, no output.
2. `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` — first run after initial repair: FAIL, 12 passed / 1 failed; remaining failure was `test_direct_api_answer_ask_resumes_suspended_run`, `left: Err(RunNotFound)`, `right: Ok(true)`.
3. `rtk cargo fmt --all && cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` — PASS, `13 tests run: 13 passed, 0 skipped`.
4. `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog` — PASS, `13 tests run: 13 passed, 0 skipped`.
5. `rtk cargo fmt --check` — PASS, no output.
6. `rtk cargo check -p vb_runtime -p velvet-ballistics-workspace-tests --tests --all-features` — PASS, `cargo build (194 crates compiled)` and `Finished dev profile`.
7. `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog` — PASS, `13 tests run: 13 passed, 0 skipped`.
8. `rtk cargo clippy -p vb_runtime --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS, `cargo clippy: No issues found`.

## Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

## Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

## Skipped gates

- Full `moon ci`, workspace clippy, miri, audit, deny, vet, geiger, machete, cargo-hack, and cargo-mutants were not run because State 10 requested focused implementation evidence for `vb-vt2f`; State 11 owns release/global gate closure per the approved test plan.

## Residual risks

- Existing State 8 test/catalog changes were already present in the isolated workspace and remain outside this State 10 production-code edit scope.
- `Ask` prompt validation now permits any present typed slot value; this matches the approved direct API BDD fixture but may need contract follow-up if another bead requires symbol-only ask prompts.

---

## State 10 implementation-repair attempt 2 delta

### Repair objective

- Preserve strict direct API admission behavior: `Runtime::submit_direct*` still validates strict/default runtime admission through the shard and still returns `RuntimeError::AdmissionArtifactNotFound { digest }` when no accepted artifact store exists.
- Restore CLI volatile execution behavior: CLI `run --durability none`, CLI `run-compiled --durability none`, and in-memory `bench-run` now build their runtime with `RuntimePolicy::Relaxed` because no durable accepted-artifact store exists in volatile mode.

### Production files changed in this attempt

- `crates/vb_cli/src/app_impl.rs`
  - Added `runtime_config_for_durability(DurabilityMode) -> ShardConfig`.
  - `DurabilityMode::None` maps to `RuntimePolicy::Relaxed` for CLI-owned volatile runtimes.
  - `DurabilityMode::Strict` and `DurabilityMode::Journaled` keep the default strict shard policy and storage-backed artifact validation path.
  - `cmd_bench_run` uses the same relaxed in-memory runtime configuration because it has no durable artifact store.

### Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked arithmetic, or lossy cast added.
- Control flow is a single bounded branch on `DurabilityMode` before runtime construction.
- Failure modes remain typed in strict/journaled paths; volatile CLI mode avoids constructing a strict runtime without an accepted-artifact source.

### Raw command evidence from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

1. `rtk cargo fmt --all && cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance && cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog && rtk cargo test -p vb_cli --test cli_integration cli_run_minimal_workflow_completes cli_run_maps_postcard_slot_values_from_input_bin` — PARTIAL: fmt PASS; direct API PASS (`13 tests run: 13 passed`); catalog PASS (`13 tests run: 13 passed`); cargo rejected two test filters in one invocation with `unexpected argument 'cli_run_maps_postcard_slot_values_from_input_bin'`.
2. `rtk cargo test -p vb_cli --test cli_integration cli_run_minimal_workflow_completes` — PASS, `cargo test: 1 passed, 85 filtered out`.
3. `rtk cargo test -p vb_cli --test cli_integration cli_run_maps_postcard_slot_values_from_input_bin` — PASS, `cargo test: 1 passed, 85 filtered out`.
4. `rtk cargo fmt --check && rtk cargo check -p vb_cli -p vb_runtime -p velvet-ballistics-workspace-tests --tests --all-features && rtk cargo clippy -p vb_cli -p vb_runtime --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS, `cargo build (1 crates compiled)`, `Finished dev profile`, `cargo clippy: No issues found`.
5. `moon ci` — FAIL before second repair: prior two CLI failures cleared, new fail-fast failure surfaced in `vb_cli::mode_activation_integration_tests bench_run_executes_in_memory_without_storage` with `runtime submit error: admission rejected: artifact not found`.
6. `rtk cargo test -p vb_cli --test mode_activation_integration_tests bench_run_executes_in_memory_without_storage` — first run FAIL with the same in-memory bench-run missing-artifact error.
7. `rtk cargo test -p vb_cli --test mode_activation_integration_tests bench_run_executes_in_memory_without_storage` — PASS after applying relaxed in-memory bench-run config, `cargo test: 1 passed, 23 filtered out`.
8. `rtk cargo test -p vb_cli --test cli_integration cli_run_minimal_workflow_completes` — PASS after final patch, `cargo test: 1 passed, 85 filtered out`.
9. `rtk cargo test -p vb_cli --test cli_integration cli_run_maps_postcard_slot_values_from_input_bin` — PASS after final patch, `cargo test: 1 passed, 85 filtered out`.
10. `rtk cargo fmt --all && cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance && cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog && rtk cargo fmt --check && rtk cargo check -p vb_cli -p vb_runtime -p velvet-ballistics-workspace-tests --tests --all-features && rtk cargo clippy -p vb_cli -p vb_runtime --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS; direct API nextest run ID `a14190da-8cc2-45cc-9135-f1687350241b`, `13 tests run: 13 passed`; catalog nextest run ID `79302ff4-76e9-48d6-bd8d-ebf44503be6e`, `13 tests run: 13 passed`; check/clippy PASS.
11. `moon ci` — FAIL after CLI repair; original CLI failures did not recur before fail-fast. New current blocker is `vb_ipc server::handlers::tests::handle_cancel_run_with_existing_run_returns_accepted`, `expected AcceptedRun, got RuntimeError { message: "admission rejected: artifact not found" }`; summary `3876/9015 tests run: 3875 passed, 1 failed, 2 skipped`; remaining tests not run due fail-fast.

### Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

### Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

### Skipped gates / blockers

- Full `moon ci` was run but remains blocked outside the requested CLI regression pair by an IPC in-memory submit path now surfacing the same strict-runtime/no-artifact-store mismatch.
- No tests/proofs/contracts were weakened.

### Residual risk

- `moon ci` remains `BLOCK_LOCAL` for an IPC handler test, not the requested two CLI integration tests. Next repair lane should decide whether IPC in-memory test runtime should be relaxed/test-only, or whether IPC submit must provide/store accepted artifacts under strict policy.

---

## State 10 implementation-repair attempt 3 delta

### Repair objective

- Clear the IPC in-memory strict-runtime regression from attempt 2: `vb_ipc server::handlers::tests::handle_cancel_run_with_existing_run_returns_accepted` expected `AcceptedRun` but the test runtime used default strict admission without durable accepted-artifact storage.
- Preserve approved strict direct API behavior: no change to `Runtime::submit_direct*`, runtime admission, strict/journaled storage-backed paths, or vt2f direct API tests.
- Preserve strict/journaled durable behavior: no change to storage-backed IPC server construction or `RuntimePolicy::Strict` / `RuntimePolicy::Journaled` handling.

### Files changed in this attempt

- `crates/vb_ipc/src/server/handlers.rs`
- `crates/vb_ipc/src/server/dispatch.rs`
- `crates/vb_ipc/src/server/impl_tests.rs`
- `crates/vb_ipc/src/server/trace.rs`
- `crates/vb_ipc/src/client.rs`

### Code changes mapped to repair

- IPC in-memory test runtime constructors now explicitly set `ShardConfig.policy = RuntimePolicy::Relaxed` before `Runtime::new(...)`.
- This localizes the relaxed policy to IPC test/in-memory runtimes that have no durable journal or accepted-artifact store.
- The production IPC storage server path remains storage-backed and unchanged; strict/journaled artifact admission remains enforced outside the explicitly relaxed in-memory boundary.

### Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked arithmetic, or lossy casts were added to modified production-reachable code.
- Control flow impact is a single explicit configuration assignment at IPC in-memory runtime construction.
- Failure behavior remains typed in strict/journaled paths; no tests, proofs, or contracts were weakened.

### Raw command evidence from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

1. `rtk cargo test -p vb_ipc server::handlers::tests::handle_cancel_run_with_existing_run_returns_accepted` — FAIL before repair; panic at `crates/vb_ipc/src/server/handlers.rs:2286:22`, `expected AcceptedRun, got RuntimeError { message: "admission rejected: artifact not found" }`.
2. `rtk cargo test -p vb_ipc server::handlers::tests::handle_cancel_run_with_existing_run_returns_accepted` — PASS after repair, `cargo test: 1 passed, 685 filtered out`.
3. `rtk cargo fmt --all && cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance && cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog && rtk cargo test -p vb_cli --test cli_integration cli_run_minimal_workflow_completes && rtk cargo test -p vb_cli --test cli_integration cli_run_maps_postcard_slot_values_from_input_bin && rtk cargo test -p vb_cli --test mode_activation_integration_tests bench_run_executes_in_memory_without_storage && rtk cargo test -p vb_ipc server::handlers::tests::handle_cancel_run_with_existing_run_returns_accepted` — PASS; direct API nextest run ID `d34f424e-b1d5-4bbf-bf26-8c438fb44955`, `13 tests run: 13 passed`; catalog nextest run ID `e7ee60b5-fcbb-4c8b-9c70-ea7012d0f59a`, `13 tests run: 13 passed`; CLI tests each `1 passed`; IPC focused test `1 passed, 685 filtered out`.
4. `rtk cargo fmt --check && rtk cargo check -p vb_ipc -p vb_cli -p vb_runtime -p velvet-ballistics-workspace-tests --tests --all-features && rtk cargo clippy -p vb_ipc -p vb_cli -p vb_runtime --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS; check compiled 3 crates and finished; clippy reported `No issues found`.
5. `moon ci` — FAIL after IPC repair; the prior IPC handler failure did not recur before fail-fast. New current blocker is `vb_runtime primitives::wait_ask::tests::ask_with_bool_prompt_returns_type_mismatch`, `left: Ok(AwaitingAsk)`, `right: Err(TypeMismatch { expected: "prompt", found: "boolean" })`; summary `5129/9015 tests run: 5128 passed, 1 failed, 2 skipped`; remaining tests not run due fail-fast. Completed/cached tasks before fail-fast included `beads-server-mode`, `agent-cli-contract`, `workspace-assertions`, `fmt`, `lint-src`, `source-length`, `nightly-feature-gate`, `check`, `feature-powerset`, `coverage`, `miri`, `bench-build`, and `fuzz-smoke`.

### Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

### Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

### Skipped gates / blockers

- Full `moon ci` was run and progressed beyond the requested IPC regression, but remains blocked by a separate `vb_runtime::primitives::wait_ask` unit-test/implementation mismatch from the prior Ask prompt semantic change.
- Full workspace clippy, miri, audit, deny, vet, geiger, machete, cargo-hack, and cargo-mutants were not independently run outside `moon ci` because this sublane requested focused IPC repair evidence and `moon ci` fail-fast stopped later tasks.

### Residual risk

- `moon ci` remains `BLOCK_LOCAL` for `vb_runtime primitives::wait_ask::tests::ask_with_bool_prompt_returns_type_mismatch`; this is not the IPC in-memory strict-runtime regression fixed in this attempt.
- IPC relaxed-policy changes are scoped to test/in-memory runtime helpers only; no new strict storage artifact evidence was added for IPC production submit paths.

---

## State 10 implementation-repair attempt 4 delta

### Repair objective

- Clear the `vb_runtime primitives::wait_ask::tests::ask_with_bool_prompt_returns_type_mismatch` regression from attempt 3.
- Preserve approved direct API ask-resume behavior where the vt2f BDD fixture uses a numeric prompt slot and only requires the ask to suspend/resume correctly through the public runtime API.
- Preserve prior strict admission, CLI relaxed durability-none, and IPC in-memory repairs.

### Files changed in this attempt

- `crates/vb_runtime/src/primitives/wait_ask.rs`

### Code changes mapped to repair

- `ask(...)` now reads the prompt slot and validates it through `validate_prompt(...)` before timeout validation and before `increment_executed()`.
- `validate_prompt(...)` rejects boolean prompt payloads with `EngineError::TypeMismatch { expected: "prompt", found: "boolean" }`.
- Numeric and symbol prompt payloads remain prompt-compatible so the approved vt2f direct API ask-resume fixture still suspends and resumes.
- No tests, proofs, or contracts were weakened.

### Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked arithmetic, or lossy cast added to production code.
- Failure mode is typed (`EngineError::TypeMismatch`) and occurs before state mutation/executed-counter increment.
- Control flow remains a single bounded `match`; no loops, allocation, dynamic dispatch, or async behavior added.

### Raw command evidence from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

1. `rtk cargo test -p vb_runtime primitives::wait_ask::tests::ask_with_bool_prompt_returns_type_mismatch` — FAIL before env workaround due environment blocker: `error writing dependencies to /tmp/sccache.../deps.d: Disk quota exceeded (os error 122)`.
2. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" CARGO_INCREMENTAL=0 rtk cargo test -p vb_runtime primitives::wait_ask::tests::ask_with_bool_prompt_returns_type_mismatch` — FAIL with same `/tmp/sccache... Disk quota exceeded` because sccache still used `/tmp`.
3. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_runtime primitives::wait_ask::tests::ask_with_bool_prompt_returns_type_mismatch` — PASS, `cargo test: 1 passed, 1531 filtered out`.
4. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance && ... vb_hxm0_acceptance_catalog` — PASS; direct API nextest run ID `2fa4cf97-7f87-4344-b6f1-389072ecabd5`, `13 tests run: 13 passed`; catalog nextest run ID `40219ef1-622b-44ff-8b0a-679cac2bcf34`, `13 tests run: 13 passed`.
5. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_cli --test cli_integration cli_run_minimal_workflow_completes && ... cli_run_maps_postcard_slot_values_from_input_bin && ... bench_run_executes_in_memory_without_storage && ... vb_ipc server::handlers::tests::handle_cancel_run_with_existing_run_returns_accepted` — PASS; CLI tests each `1 passed`; IPC test `1 passed, 685 filtered out`.
6. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check && ... cargo check -p vb_runtime -p vb_cli -p vb_ipc -p velvet-ballistics-workspace-tests --tests --all-features && ... cargo clippy -p vb_runtime -p vb_cli -p vb_ipc --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS; check compiled 18 crates, clippy reported `No issues found`.
7. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci` — FAIL after the wait_ask regression cleared. New fail-fast blocker: `vb_runtime shard::lifecycle::tests::action_failure_unknown_run_returns_run_not_found`, `left: Err(InvalidActionCompletion)`, `right: Err(RunNotFound)`; summary `5344/9015 tests run: 5343 passed, 1 failed, 2 skipped`; `3671/9015 tests were not run due to test failure`. Completed/cached tasks before fail-fast included `beads-server-mode`, `agent-cli-contract`, `workspace-assertions`, `fmt`, `lint-src`, `source-length`, `nightly-feature-gate`, `check`, `feature-powerset`, `coverage`, `miri`, `bench-build`, and `fuzz-smoke`.

### Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

### Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

### Skipped gates / blockers

- Full `moon ci` was run and progressed beyond the requested `wait_ask` regression, but remains blocked by `vb_runtime shard::lifecycle::tests::action_failure_unknown_run_returns_run_not_found` from prior action-failure semantics.
- Audit, deny, vet, geiger, machete, cargo-hack full feature powerset, and cargo-mutants were not independently run outside `moon ci`/focused gates because this sublane was scoped to the wait_ask regression repair.

### Residual risk

- `moon ci` remains `BLOCK_LOCAL` for action-failure unknown-run semantics, outside this attempt's requested wait_ask type-mismatch regression.
- Prompt compatibility is intentionally minimal for vt2f: booleans are rejected as non-prompt payloads; non-boolean present prompt payloads remain accepted to preserve approved direct API ask-resume behavior.

---

## State 10 implementation-repair attempt 5 delta

### Repair objective

- Clear `vb_runtime shard::lifecycle::tests::action_failure_unknown_run_returns_run_not_found` after attempt 4 progressed `moon ci` past `wait_ask`.
- Restore lower shard/lifecycle semantics: a direct shard `ActionFailed` command for an unknown run returns `RuntimeError::RunNotFound`.
- Preserve approved public runtime/direct API oracle: `Runtime::fail_action` for a wrong/absent run still enqueues successfully, then `Runtime::tick_all()` returns `RuntimeError::InvalidActionCompletion` and does not mutate unrelated run state.
- Preserve strict admission, CLI relaxed durability-none, IPC in-memory, and wait_ask repairs from earlier attempts.

### Files changed in this attempt

- `crates/vb_runtime/src/runtime.rs`
- `crates/vb_runtime/src/shard/types.rs`
- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`

### Code changes mapped to repair

- Added `ShardCommand::RuntimeActionFailed` for the public `Runtime::fail_action` facade.
- `Runtime::fail_action` now enqueues `RuntimeActionFailed`, not lower-layer `ActionFailed`.
- Shard tick handles lower-layer `ActionFailed` with raw lifecycle semantics, while `RuntimeActionFailed` maps only `RunNotFound` to `InvalidActionCompletion` at the public facade boundary.
- `ticket_with_retry_capacity(...)` returns `RunNotFound` when the run is absent, restoring the lower lifecycle unit-test oracle.
- No tests, proofs, or contracts were weakened.

### Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked arithmetic, or lossy cast added to production code.
- Failure modes remain typed and boundary-scoped: lower lifecycle reports unknown run; public runtime facade reports invalid action completion for absent/wrong action tickets.
- Control flow impact is a single enum dispatch branch and a bounded `match` error mapper; no loops, allocation, dynamic dispatch, async behavior, or new I/O added.

### Raw command evidence from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

1. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_runtime shard::lifecycle::tests::action_failure_unknown_run_returns_run_not_found` — first run after patch FAIL compile-only: `cannot find value runtime_action_failure_error in this scope`; fixed call to `Self::runtime_action_failure_error`.
2. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_runtime shard::lifecycle::tests::action_failure_unknown_run_returns_run_not_found` — PASS, `cargo test: 1 passed, 1531 filtered out`.
3. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` — PASS, Nextest run ID `4ee8a52e-0fe2-426d-939e-b4d5090961f4`, `13 tests run: 13 passed, 0 skipped`.
4. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog && ... wait_ask focused test && ... CLI/IPC prior repaired tests` — PARTIAL: catalog PASS, Nextest run ID `be5fbdbf-4318-486e-b1bb-27c8df190782`, `13 tests run: 13 passed`; wait_ask focused test PASS, `1 passed, 1531 filtered out`; first CLI test failed due environment setup only: `tempdir failed: No such file or directory (os error 2) at path "/home/lewis/src/bd-vb-vt2f-bdd/.tmp/.tmpxNpMJG"`.
5. `mkdir -p "/home/lewis/src/bd-vb-vt2f-bdd/.tmp"` after verifying the isolated workspace parent existed — PASS, no output.
6. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_cli --test cli_integration cli_run_minimal_workflow_completes && ... cli_run_maps_postcard_slot_values_from_input_bin && ... bench_run_executes_in_memory_without_storage && ... vb_ipc server::handlers::tests::handle_cancel_run_with_existing_run_returns_accepted` — PASS; CLI tests each `1 passed`; IPC test `1 passed, 685 filtered out`.
7. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check && ... cargo check -p vb_runtime -p vb_cli -p vb_ipc -p velvet-ballistics-workspace-tests --tests --all-features && ... cargo clippy -p vb_runtime -p vb_cli -p vb_ipc --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS; check compiled 4 crates and finished; clippy reported `No issues found`.
8. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci` — FAIL after the action-failure unknown-run regression cleared. New fail-fast blocker: `velvet-ballistics-workspace-tests::vb_qi37_4_2_strict_runtime_admission given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required`, `left: Err(AdmissionArtifactNotFound { digest: WorkflowDigest([166, 68, 38, 237, 1, 79, 255, 206, 110, 230, 31, 80, 22, 67, 108, 188, 87, 76, 182, 176, 242, 209, 90, 70, 127, 35, 59, 241, 170, 183, 254, 172]) })`, `right: Ok(true)`; summary `8843/9015 tests run: 8842 passed, 1 failed, 2 skipped`; `172/9015 tests were not run due to test failure`. Completed/cached tasks before fail-fast included `beads-server-mode`, `agent-cli-contract`, `workspace-assertions`, `fuzz-smoke`, `lint-src`, `nightly-feature-gate`, `fmt`, `miri`, `source-length`, `check`, `feature-powerset`, `nightly-feature-cargo-probe`, `coverage`, and `bench-build`.

### Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

### Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

### Skipped gates / blockers

- Full `moon ci` was run and progressed beyond the requested lower-layer action-failure unknown-run regression, but remains blocked by strict runtime admission test `vb_qi37_4_2_strict_runtime_admission::given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required`.
- Audit, deny, vet, geiger, machete, full cargo-hack outside moon, and cargo-mutants were not independently run because this sublane was scoped to the action-failure unknown-run repair and `moon ci` remains fail-fast blocked later in the test set.

### Residual risk

- `moon ci` remains `BLOCK_LOCAL` for strict journaled runtime admission behavior, outside this attempt's requested action-failure unknown-run mapping repair.
- The new `RuntimeActionFailed` command variant is intentionally boundary-only; future lower-layer tests should continue using `ShardCommand::ActionFailed` for raw lifecycle semantics.

---

## State 10 implementation-repair attempt 6 delta

### Repair objective

- Clear `velvet-ballistics-workspace-tests::vb_qi37_4_2_strict_runtime_admission given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required` after attempt 5 progressed `moon ci` past action-failure mapping.
- Preserve approved vt2f direct public runtime behavior: `Runtime::new(...)` / strict direct submit without a storage-backed accepted-artifact source still rejects with `RuntimeError::AdmissionArtifactNotFound { digest }` before enqueue.
- Distinguish explicit shard/unit construction with an accepted-artifact store from runtime construction with no storage-backed store; do not blanket-reject strict/journaled shard tests that intentionally use the always-present accepted store.
- Preserve prior repairs: CLI relaxed durability-none, IPC in-memory, wait_ask bool mismatch, and lower `ActionFailed` `RunNotFound` vs public `InvalidActionCompletion` mapping.

### Files changed in this attempt

- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
- `.beads/vb-vt2f/implementation.md`

### Code changes mapped to repair

- `Shard::new(config)` now constructs a shard with `NoopRuntimeJournal::shared()` and an explicit `AlwaysPresentArtifactStore::shared()` via `new_with_journal_and_artifact_store(...)`.
- `Shard::new_with_journal(config, journal)` keeps the strict/journaled no-storage behavior: storage-backed journals use `StorageArtifactStore`; relaxed volatile journals use `AlwaysPresentArtifactStore`; strict/journaled volatile journals use `MissingAcceptedArtifactStore`.
- This restores the qi37 shard-level constructed-store BDD while preserving public `Runtime::new(...)` strict missing-store rejection because `Runtime::new_with_journal(...)` still routes through `Shard::new_with_journal(...)`.
- No tests, proofs, or contracts were weakened.

### Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked arithmetic, or lossy cast added to production code.
- Failure mode remains typed: strict/journaled runtime without storage-backed accepted-artifact source rejects with `AdmissionArtifactNotFound`; explicit shard accepted-store construction admits.
- Control flow impact is a bounded constructor branch only; no loops, allocation beyond existing `Arc` store construction, dynamic dispatch changes beyond existing trait-object store, async behavior, or I/O added.

### Raw command evidence from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

Environment for gates where used: `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0`.

1. `mkdir -p "/home/lewis/src/bd-vb-vt2f-bdd/.tmp" && TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_4_2_strict_runtime_admission given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required` — PASS, `cargo test: 1 passed, 20 filtered out`.
2. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` — PASS, Nextest run ID `00d31a9b-49c3-43e9-9154-bbc6ba56a1f2`, `13 tests run: 13 passed, 0 skipped`.
3. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog && ... wait_ask bool mismatch && ... action_failure_unknown_run_returns_run_not_found && ... CLI durability-none tests && ... IPC in-memory accepted-run test` — PASS; catalog Nextest run ID `48436b6a-1f6f-4f5e-a048-6d6effb60142`, `13 tests run: 13 passed`; wait_ask focused test `1 passed, 1531 filtered out`; action-failure focused test `1 passed, 1531 filtered out`; CLI tests each `1 passed`; IPC test `1 passed, 685 filtered out`.
4. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check && ... cargo check -p vb_runtime -p vb_cli -p vb_ipc -p velvet-ballistics-workspace-tests --tests --all-features && ... cargo clippy -p vb_runtime -p vb_cli -p vb_ipc --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS; check compiled 4 crates; clippy reported `No issues found`.
5. `TMPDIR="/home/lewis/src/bd-vb-vt2f-bdd/.tmp" RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci` — PASS; `velvet-ballistics:test` reported `9015 tests run: 9015 passed, 2 skipped`; tasks completed: `20 completed (4 cached)`; total `1m 35s 850ms`.

### Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

### Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

### Skipped gates / blockers

- No local blocker remains for the requested strict journaled runtime admission repair.
- Audit, deny, vet, geiger, machete, and cargo-mutants were not independently run outside `moon ci` because this sublane was scoped to implementation repair and the canonical repo gate passed.

### Residual risk

- `Shard::new(...)` remains a shard-level constructor with an explicit always-present accepted store; production public runtime construction uses `Runtime::new(...)` / `Shard::new_with_journal(...)` and preserves strict/journaled missing-store rejection.

---

## State 10 implementation-repair stale-ask-immediate-runnotfound delta

### Repair objective

- Make concrete public `Runtime::answer_ask(...)` reject a stale ask ticket for a terminal/non-active run immediately with `Err(RuntimeError::RunNotFound)`.
- Preserve the approved direct API BDD oracle and unrelated active-run non-mutation checks.
- Do not weaken contracts, proof artifacts, or tests.

### Files changed in this repair

- `crates/vb_runtime/src/runtime.rs`
- `crates/vb_runtime/src/trace.rs`
- `.beads/vb-vt2f/implementation.md`

### Code changes mapped to repair

- `Runtime::answer_ask(...)` now checks the target shard before enqueueing `ShardCommand::AskAnswered`.
- If the ticket run is not currently active and bounded trace history contains terminal evidence for that same run (`RunFinished`, `RunFailed`, or `RunCancelled`), the API returns `RuntimeError::RunNotFound` immediately.
- Added bounded trace helpers `TraceRing::has_terminal_event_for_run(...)` and `TraceEvent::is_terminal_for_run(...)` to avoid allocating a snapshot for the stale-ticket predicate.
- Unrelated active runs are not touched on this rejection path.

### Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked arithmetic, or lossy cast added.
- Failure mode is typed and returned before queue mutation.
- New trace scan is bounded by trace-ring capacity and uses checked counter advancement.
- No async, I/O, dynamic dispatch, or new heap allocation added on the stale rejection path.

### Raw command evidence from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

Environment for commands: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0`.

1. `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` — PASS; `13 tests run: 13 passed, 0 skipped`.
2. `rtk cargo test -p vb_runtime answer_ask` — PASS; `1 passed, 1531 filtered out`.
3. `rtk cargo fmt --check` — PASS; no output.
4. `rtk cargo check -p vb_runtime --all-targets --all-features` — PASS; `Finished dev profile`.
5. `rtk cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS; `cargo clippy: No issues found`.

### Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

### Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

### Skipped gates / blockers

- Full `moon ci`, workspace-wide test, miri, audit, deny, vet, geiger, machete, cargo-hack, and cargo-mutants were not run because this dispatch explicitly requested the focused direct API target and focused runtime checks for stale ask behavior only.

### Residual risk

- Stale terminal detection depends on bounded retained trace history. This satisfies the concrete approved BDD fixture and immediate terminal/non-active run behavior covered here; a future bead should define behavior if terminal trace evidence has been drained or evicted.

---

## State 10 implementation-repair attempt 4 delta — trace-eviction stale ask

### Repair objective

- Fix `LETHAL-001`: stale terminal/non-active ask rejection must not depend on retained trace history.
- Preserve existing direct API behavior for wrong/absent run tickets: absent wrong-run `answer_ask` still enqueues and fails on `tick_all` with `RuntimeError::RunNotFound`.
- Preserve unrelated-run non-mutation and the new RED BDD oracle without editing proof artifacts.

### Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

### Input artifacts read

- `.beads/vb-vt2f/dispatch-state10-holzman-rust-trace-eviction-stale-ask.json`
- `.beads/vb-vt2f/black-hat-review.md`
- `.beads/vb-vt2f/defects.md`
- `.beads/vb-vt2f/test-writer-report.md`
- `.beads/vb-vt2f/test-plan-review.md`
- `.beads/vb-vt2f/test-suite-review.md`
- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`
- `crates/vb_runtime/src/runtime.rs`
- `crates/vb_runtime/src/trace.rs`
- `crates/vb_runtime/src/shard/types.rs`
- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
- `crates/vb_runtime/src/shard/impl_parts/chunk_004.rs`
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
- `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`
- `crates/vb_runtime/src/shard/transitions.rs`

### Code changes made in this attempt

- Added `Shard::terminal_runs: IndexSet<RunId>` as direct runtime/shard state for terminal run tombstones independent of `TraceRing` retention.
- Initialized `terminal_runs` in shard construction.
- Inserted terminal run facts on finished, failed, and cancelled active-run terminal paths.
- Cleared a terminal tombstone on accepted same-run submission before inserting a new active `RunState`.
- Changed `Runtime::answer_ask` to return immediate `Err(RuntimeError::RunNotFound)` only when the target run is absent from active `runs` and present in `terminal_runs`; it no longer consults `TraceRing::has_terminal_event_for_run`.

### Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked arithmetic, or lossy cast added by this repair.
- Failure remains typed and returns before command queue mutation for stale terminal tickets.
- Control flow is a single bounded lookup branch in `answer_ask`; no loops added to the direct rejection path.
- New state uses `IndexSet<RunId>`; allocation occurs on terminal transition bookkeeping, not during the immediate stale rejection branch. This is correctness state, not a speed claim.

### Raw command evidence from isolated workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

Environment for cargo commands: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0`.

1. `rtk cargo fmt --check` — PASS; no output.
2. `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted` — PASS; run ID `c42531d6-cd18-4c66-ae1f-f45f19ed43b1`; `1 test run: 1 passed, 13 skipped`.
3. `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` — PASS; run ID `fbcf32a0-1d85-459f-907b-7e27f89259c7`; `14 tests run: 14 passed, 0 skipped`.
4. `rtk cargo test -p vb_runtime answer_ask --all-features` — PASS; `1 passed, 1531 filtered out`.
5. `rtk cargo check --workspace --all-targets --all-features` — PASS; `cargo build (110 crates compiled)`, `Finished dev profile`.
6. `rtk cargo clippy -p vb_runtime --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS; `cargo clippy: No issues found`.
7. `rtk cargo test -p vb_runtime --all-features` — PASS; `1532 passed (10 suites, 1.27s)`.
8. Touched-production panic macro scan over exact touched file paths — FAIL/NOT APPLICABLE as written because `crates/vb_runtime/src/runtime.rs` embeds `#[cfg(test)]` tests in the same file and the scan reported test-only `assert!`/`assert_eq!` lines. No production-reachable assert macro was added by this repair.

### Performance-layer decision

No performance claim made. No benchmark/profiler evidence required for this behavior repair.

### Second-ring evidence

Not required. No assembly/IR/vectorization/API compatibility/release provenance claim was made.

### Skipped gates / blockers

- Full `moon ci`, miri, audit, deny, vet, geiger, machete, cargo-hack, and cargo-mutants were not run because this dispatch requested the targeted direct API test, full direct API file, focused runtime tests, and fmt/check/clippy scopes only.
- Proof artifacts were not edited per dispatch instruction.

### Residual risk

- `terminal_runs` is an in-memory terminal tombstone ledger; long-lived runtimes with many distinct terminal run IDs may retain one `RunId` per terminal run until a same-ID resubmission clears that tombstone. This is the chosen correctness tradeoff to make stale rejection independent of lossy trace retention.
- Existing unrelated uncommitted workspace changes were present before this attempt and were not broadened beyond the stale-ask repair files.

### Classification / next route

- `PASS_LOCAL`: `LETHAL-001` repaired for the approved BDD and scoped runtime checks.
- Next route: return to femdation for State 11/formal ledger or State 12 black-hat re-review as controller decides.
