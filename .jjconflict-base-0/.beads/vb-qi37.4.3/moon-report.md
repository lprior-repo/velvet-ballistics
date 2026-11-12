bead_id: vb-qi37.4.3
phase: State 8 rerun green after State 13 refactor and rebase repair
updated_at: 2026-05-11T00:00:00Z

# Moon Report

## State 13 focused evidence
- `rtk cargo fmt --all` -> PASS.
- touched/scoped split line-count check -> PASS; checked 69 Rust façade/chunk files, max 280 lines at `crates/vb_runtime/src/shard/tests/chunk_023.rs`, bad=0.
- `rtk cargo test -p vb_runtime runtime::tests::submit_direct_returns_durability_error_before_ack_when_header_cannot_persist` -> PASS: 1 passed, 1349 filtered out.
- `rtk cargo test -p vb_runtime submit_rejects_duplicate_run_id` -> PASS: 2 passed, 1348 filtered out.
- `rtk cargo test -p velvet_ballistics --test admission_evidence_integration storage_failure_before_header_prevents_ack` -> PASS: 1 passed, 7 filtered out.
- `rtk cargo test -p velvet_ballistics --test admission_evidence_integration restart_lookup_finds_persisted_header` -> PASS: 1 passed, 7 filtered out.

## State 8 rerun commands
- `moon run :quick` -> PASS.
- `moon run :test` -> PASS: 9857 tests run, 9857 passed, 0 skipped.
- `moon ci` -> FAIL/non-zero; captured output path `/home/lewis/.local/share/opencode/tool-output/tool_e19eec513001LhIUgUOPhQLcS1`.

## `moon ci` red items observed
- `velvet-ballistics:lint-src` failed in pre-existing/global files outside this State 13 split:
  - `crates/vb_proof_kernels/src/envelope_header.rs`: clippy `new_without_default` for `EnvelopeHeader`.
  - `xtask/src/proof.rs`: clippy `panic_in_result_fn` / `panic` at `unwrap_or_else(|| panic!(...))`.
- `velvet-ballistics:feature-powerset` failed in pre-existing/global UI model code:
  - `crates/vb_ui_model/src/envelope.rs`: `Vec` not in scope under `--no-default-features`.
  - `crates/vb_ui_model/src/emitter.rs` and `crates/vb_ui_model/src/envelope.rs`: `#![no_std]` attribute only valid at crate root.

## Classification
- State 13 blocker is unblocked by refactor: source-length task completed during `moon ci`.
- Downstream State 8 rerun is now green after rebasing onto `main` and repairing local split/schema drift.

## 2026-05-11 rebase repair rerun
- `moon run :quick` -> PASS.
- `rtk cargo test -p vb_runtime --test vb_jggy_lifecycle_tests --all-features` -> PASS: 15 passed.
- `rtk cargo test -p vb_runtime retry_exhaustion_emits_single_action_failed --all-features` -> PASS: 1 passed, 1441 filtered out.
- `rtk cargo test -p vb_runtime future_attempt_completion_rejected_when_current_attempt_exists --all-features` -> PASS: 1 passed, 1441 filtered out.
- `moon ci` -> PASS. Output path: `/home/lewis/.local/share/opencode/tool-output/tool_e1a0aaf70001OZ4gLQnSoCc4xB`.
- `moon ci` summary: 19 completed, 2 cached, 0 failed; `velvet-ballistics:test` 8015/8015 passed; `feature-powerset`, `miri`, `coverage`, `mutants-smoke`, `bench-build`, `doc-test`, `doc`, `maxperf`, `hardened-build`, and `maxperf-native` passed.
