bead_id: vb-qi37.3
bead_title: runtime: Prove collect pagination durability and hydration
phase: State 15 - Landing gate rerun after rebase
updated_at: 2026-05-11T12:51:12Z

# Moon / Machine Gate Report

## Scope

- Workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go`
- JJ workspace: `vb-qi37-3-go`
- Current change after black-hat repair: `xqywtqkz 6771d70e`
- Parent/main: `qwxtlxqq 5fb2d246 main | fix: add missing ObligationStatus and ProofEvidence structs`
- Delivery scope: `.beads/vb-qi37.3/delivery-scope.jsonl` scope_version 2, touched crates `vb_runtime`, `vb_storage`, `vb_core`.
- Actual changed source files remain scoped to:
  - `crates/vb_core/src/engine/error_routing.rs`
  - `crates/vb_core/src/errors.rs`
  - `crates/vb_core/src/ids/mod.rs`
  - `crates/vb_core/src/lib.rs`
  - `crates/vb_runtime/src/collect_tests.rs`
  - `crates/vb_runtime/src/engine/drive.rs`
  - `crates/vb_runtime/src/engine/types.rs`
  - `crates/vb_runtime/src/primitives/collect.rs`
  - `crates/vb_storage/src/types.rs`

## Black-hat repair gate evidence

- State 6 repair reran focused black-hat tests: Nextest run ID `c9950934-6e87-44e3-80ec-418bb4618529`, 3 tests run, 3 passed, 1356 skipped.
- State 6 repair reran broad collect suite: Nextest run ID `1783661d-9078-49dc-b390-81f1e48e8d56`, 102 tests run, 102 passed, 1257 skipped.
- State 7 smoke rerun passed with product CLI help exit 0, focused black-hat repair filter 3/3, `collect_next_` filter 19/19, hydration/capacity filter 7/7, and broad collect suite 102/102.

## Passing State 8 gates after black-hat repair

- `moon run :quick`: PASS. `velvet-ballistics:quick` completed and printed `Hello, world!` four times.
- `moon run :test`: PASS. `agent-cli-contract` cached PASS, `nightly-feature-gate` PASS, `velvet-ballistics:check` PASS, `velvet-ballistics:test` PASS. Nextest run ID `c5c2f6dd-5ea3-46d0-840d-8e2fffd3a48b`; `9864 tests run: 9864 passed, 0 skipped`.

## Canonical CI invocation note

- Plain `moon ci` in the isolated JJ workspace previously failed before task execution because the workspace has no raw Git `main` ref visible to Moon: `fatal: ambiguous argument 'main': unknown revision or path not in the working tree`.
- State 8 therefore uses Moon's supported changed-file stdin mode: `jj diff --name-only | moon ci --stdin`.

## `moon ci --stdin` result after black-hat repair

- Output artifact: `/home/lewis/.local/share/opencode/tool-output/tool_e15e2afb6001ZnfdEBf0ifxqwI`.
- Summary line: `Tasks: 12 completed (2 cached), 3 failed, 3 skipped`.
- Positive evidence in the run: `test` reported Nextest run ID `f55f4f70-c825-44e4-9cd4-80fc1af7f99f` with `9864 tests run: 9864 passed, 0 skipped`; `coverage`, `miri`, `bench-build`, `doc`, and `doc-test` completed; skipped tasks were hardened/maxperf variants.
- The failing tasks are global regression sensors, not bead-local collect tests.

## Failed global sensors

### FORMAT

- `moon run :fmt`: FAIL.
- Evidence: rustfmt check emitted diffs in pre-existing unmodified/global files. Explicit diff files include `crates/vb_proof_kernels/src/step_state.rs`, `crates/vb_proof_kernels/src/taint.rs`, `crates/vb_storage/src/codec_miri_tests.rs`, `crates/vb_storage/src/kani_codec.rs`, `crates/vb_storage/src/lib.rs`, fuzz targets, `xtask/src/main.rs`, `xtask/src/proof.rs`, and other global files.
- These FORMAT failure files are not part of `vb-qi37.3` actual changed source files.

### CLIPPY / lint-src

- `moon ci --stdin` line 118: `error: you should consider adding a Default implementation for EnvelopeHeader` at `crates/vb_proof_kernels/src/envelope_header.rs:26:5`.
- Line 141: `error: could not compile vb_proof_kernels (lib) due to 1 previous error`.
- Clean canonical main reproduction also showed pre-existing lint failures in fuzz targets for `let_underscore_must_use` and `xtask/src/proof.rs` for `panic_in_result_fn` / `panic`.
- These CLIPPY failure files are not part of `vb-qi37.3` actual changed source files.

### FEATURE-POWERSET / no-default-features compile error

- `moon ci --stdin` line 1210 started `cargo check --quiet --no-default-features` on `vb_ui_model`.
- Lines 1214+ failed with missing `Vec` in `crates/vb_ui_model/src/envelope.rs` at lines 200, 201, 283, 335, and 346.
- The same run also failed on `#![no_std]` attributes outside crate root in `crates/vb_ui_model/src/emitter.rs:2` and `crates/vb_ui_model/src/envelope.rs:2` under `-D warnings`.
- Clean canonical main reproduced the same `vb_ui_model` no-default-features failure with `rustup run nightly-2026-04-28 cargo check --quiet --manifest-path crates/vb_ui_model/Cargo.toml --no-default-features`.
- `vb_ui_model` is not part of this bead's touched crates or actual changed files.

## State 8 result

- Primary CI failure category: `FORMAT`.
- Secondary categories: `CLIPPY`, `COMPILE_ERROR`.
- Blocking classification: `DEFERRED_GLOBAL`, not `BLOCK_LOCAL` or `BLOCK_REGRESSION`, because failures reproduce on clean main and the explicit failing files/crates are outside this bead's actual changed source files and delivery scope.
- Follow-up bead: `vb-bkgo` (`ci: Restore global fmt and vb_proof_kernels lint gates`), already updated to include FORMAT, CLIPPY, and `vb_ui_model` no-default-features failure evidence.

## State 15 landing gate rerun after rebase

- Rebased bead change onto remote `main@origin` for landing currency.
- Current change: `xqywtqkz 2a734ab1`.
- Parent: `stvmrlkk 1be80acf main@origin | feat(vb-l2d7): reconcile taint propagation docs`.
- Rebase command: `jj rebase -r @ -d main@origin` completed cleanly with no conflicts.

### Passing landing gates

- `moon run :quick`: PASS after rebase. `velvet-ballistics:quick` completed and printed `Hello, world!` four times.
- `moon run :test`: PASS after rebase. `agent-cli-contract`, `nightly-feature-gate`, `check`, and `test` passed. Nextest run ID `59a379be-fa8c-49ac-a096-c465f8d065fc`; `9958 tests run: 9958 passed, 0 skipped`.

### Landing `moon ci --stdin`

- Command: `jj diff --name-only | moon ci --stdin`.
- Output artifact: `/home/lewis/.local/share/opencode/tool-output/tool_e171328570011GHvsN9kKw1FAw`.
- Summary: `Tasks: 12 completed (1 cached), 3 failed, 3 skipped`.
- Positive evidence: `check` completed; `test` completed with Nextest run ID `4ab00d93-6058-4ec2-a6f4-0c3e63f6d651` and `9958 tests run: 9958 passed, 0 skipped`; `coverage` completed and wrote `target/llvm-cov/lcov.info`; `miri`, `fuzz-smoke`, `bench-build`, `doc`, and `doc-test` completed; `hardened-build`, `maxperf`, and `maxperf-native` were skipped.

### Landing failure classification

- Remaining red sensors are unchanged known global debts tracked by follow-up bead `vb-bkgo`.
- FORMAT: rustfmt diffs in unmodified/global files such as `crates/vb_proof_kernels/src/step_state.rs`, `crates/vb_proof_kernels/src/taint.rs`, `crates/vb_storage/src/codec_miri_tests.rs`, `crates/vb_storage/src/kani_codec.rs`, `crates/vb_storage/src/lib.rs`, fuzz targets, `xtask/src/main.rs`, and `xtask/src/proof.rs`.
- COMPILE_ERROR: `vb_ui_model` no-default-features feature-powerset failure with missing `Vec` in `crates/vb_ui_model/src/envelope.rs` and `#![no_std]` attribute placement errors in `emitter.rs` and `envelope.rs`.
- CLIPPY: prior `EnvelopeHeader`/fuzz/xtask lint-src failures remain included in `vb-bkgo`; no bead-local lint failure was observed in the scoped changed files.
- Classification remains `DEFERRED_GLOBAL`, not `BLOCK_LOCAL` or `BLOCK_REGRESSION`, because these files/crates are outside the bead's actual changed source files and the same debt was reproduced on clean canonical main earlier in State 8.
