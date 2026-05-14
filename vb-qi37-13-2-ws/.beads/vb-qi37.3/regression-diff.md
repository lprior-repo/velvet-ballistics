bead_id: vb-qi37.3
bead_title: runtime: Prove collect pagination durability and hydration
phase: State 15 - Landing gate rerun after rebase
updated_at: 2026-05-11T12:51:12Z

# Regression Diff and Failure Classification

## Inputs compared

- Baseline artifact: `.beads/vb-qi37.3/baseline-report.md`.
  - Baseline was captured before source/test edits.
  - Baseline global machine gates were not run because the initial femdation attempt stopped in State 1.
- Delivery scope artifact: `.beads/vb-qi37.3/delivery-scope.jsonl`, scope_version 2.
- Current JJ diff evidence: `jj diff --name-only` in `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go` after black-hat repair.
- Clean-main reproduction evidence: canonical workspace `/home/lewis/src/Velvet-ballistics` at parent `qwxtlxqq 5fb2d246 main` with `jj status` clean.

## Current changed files

The bead's actual changed source files are limited to:

- `crates/vb_core/src/engine/error_routing.rs`
- `crates/vb_core/src/errors.rs`
- `crates/vb_core/src/ids/mod.rs`
- `crates/vb_core/src/lib.rs`
- `crates/vb_runtime/src/collect_tests.rs`
- `crates/vb_runtime/src/engine/drive.rs`
- `crates/vb_runtime/src/engine/types.rs`
- `crates/vb_runtime/src/primitives/collect.rs`
- `crates/vb_storage/src/types.rs`

No `vb_proof_kernels`, fuzz target, `xtask`, `vb_ui_model`, or global rustfmt-diff file is modified by this bead.

## Passing bead-scoped evidence after black-hat repair

- Focused black-hat repair tests passed: 3/3.
- Broad collect suite passed: 102/102.
- State 7 manual smoke rerun passed with `STATUS: PASS`.
- `moon run :quick`: PASS after repair.
- `moon run :test`: PASS after repair; Nextest run ID `c5c2f6dd-5ea3-46d0-840d-8e2fffd3a48b`, `9864 tests run: 9864 passed, 0 skipped`.
- `jj diff --name-only | moon ci --stdin` also reported `test` PASS with Nextest run ID `f55f4f70-c825-44e4-9cd4-80fc1af7f99f`, `9864 tests run: 9864 passed, 0 skipped`, plus completed coverage, miri, bench-build, doc, and doc-test.

## Failed global evidence after black-hat repair

`jj diff --name-only | moon ci --stdin` produced:

- Output artifact: `/home/lewis/.local/share/opencode/tool-output/tool_e15e2afb6001ZnfdEBf0ifxqwI`.
- Summary: `Tasks: 12 completed (2 cached), 3 failed, 3 skipped`.
- Primary explicit failure: `velvet-ballastics:fmt` with rustfmt diffs in pre-existing global files.
- Secondary explicit failure: `velvet-ballastics:lint-src` with Clippy failure outside the bead's changed files.
- Additional explicit global failure: `feature-powerset` / no-default-features compile error in `vb_ui_model`.

Clean-main reproduction already recorded:

- Canonical workspace `/home/lewis/src/Velvet-ballistics` was clean before and after reproduction: `jj status` reported no changes at `omurumwy c40d0ad6`, parent `qwxtlxqq 5fb2d246 main`.
- Clean main reproduced FORMAT/CLIPPY global debt through `moon run :fmt` / `moon run :lint-src` diagnostics.
- Clean main reproduced `vb_ui_model` no-default-features compile error with `rustup run nightly-2026-04-28 cargo check --quiet --manifest-path crates/vb_ui_model/Cargo.toml --no-default-features`.

## Classification matrix

| Failure | Category | Explicit failing files/crates | In actual changed files? | Reproduced on clean main? | Classification |
|---|---|---|---:|---:|---|
| rustfmt diffs | FORMAT | `crates/vb_proof_kernels/src/*`, `crates/vb_storage/src/lib.rs`, fuzz targets, `xtask/src/*`, others | No | Yes | DEFERRED_GLOBAL |
| `EnvelopeHeader::new` lacks `Default` | CLIPPY | `crates/vb_proof_kernels/src/envelope_header.rs` | No | Yes | DEFERRED_GLOBAL |
| ignored must-use results in fuzz targets | CLIPPY | `fuzz/fuzz_targets/decode_record.rs`, `fuzz/fuzz_targets/lex_expr.rs` | No | Yes | DEFERRED_GLOBAL |
| panic in result-returning proof evidence writer | CLIPPY | `xtask/src/proof.rs` | No | Yes | DEFERRED_GLOBAL |
| no-default-features compile failure | COMPILE_ERROR | `crates/vb_ui_model/src/envelope.rs`, `crates/vb_ui_model/src/emitter.rs` | No | Yes | DEFERRED_GLOBAL |

## Decision

State 8 global red gates are classified as `DEFERRED_GLOBAL`.

Rationale:

- The bead-local runtime/storage/core collect pagination implementation and tests pass the scoped gates run after black-hat repair.
- The explicit failing FORMAT/CLIPPY/COMPILE_ERROR files and crates are not among the bead's actual changed source files.
- The same global debts reproduce on clean canonical main, proving they are pre-existing repo-wide debt rather than a `vb-qi37.3` regression.
- Go-skill high-assurance bead scope requires exact evidence and follow-up work for old unrelated debt, not restarting unrelated implementation.

Follow-up bead remains open and updated:

- `vb-bkgo` - `ci: Restore global fmt and vb_proof_kernels lint gates`
- Status: open
- Priority: 1
- Labels: `deferred-global`, `ci`, `format`, `clippy`, `vb_proof_kernels`
- Description includes FORMAT, CLIPPY, and `vb_ui_model` no-default-features COMPILE_ERROR evidence and acceptance.

State 8 may continue with this deferred-global debt recorded. Downstream release/black-hat/formal gates may still choose stricter release policy if they require a fully green global `moon ci` before landing.

## State 15 landing rebase delta

Before landing, the bead change was rebased from the stale local parent onto remote `main@origin`:

- Current change after rebase: `xqywtqkz 2a734ab1`.
- Parent after rebase: `stvmrlkk 1be80acf main@origin | feat(vb-l2d7): reconcile taint propagation docs`.
- Local `main` bookmark is conflicted in another workspace and was not moved.

Post-rebase bead-local gates:

- `moon run :quick`: PASS.
- `moon run :test`: PASS; Nextest run ID `59a379be-fa8c-49ac-a096-c465f8d065fc`, `9958 tests run: 9958 passed, 0 skipped`.
- `jj diff --name-only | moon ci --stdin`: output `/home/lewis/.local/share/opencode/tool-output/tool_e171328570011GHvsN9kKw1FAw`, summary `Tasks: 12 completed (1 cached), 3 failed, 3 skipped`.
- Positive CI evidence: `test` Nextest run ID `4ab00d93-6058-4ec2-a6f4-0c3e63f6d651`, `9958 tests run: 9958 passed, 0 skipped`; `coverage`, `miri`, `fuzz-smoke`, `bench-build`, `doc`, and `doc-test` completed.

Post-rebase failures match the already recorded deferred-global debts:

| Failure | Category | Explicit failing files/crates | In actual changed files? | Existing follow-up | Classification |
|---|---|---|---:|---|---|
| rustfmt diffs | FORMAT | `crates/vb_proof_kernels/src/*`, `crates/vb_storage/src/*`, fuzz targets, `xtask/src/*`, others | No | `vb-bkgo` | DEFERRED_GLOBAL |
| lint-src debt | CLIPPY | `vb_proof_kernels`, fuzz targets, `xtask` | No | `vb-bkgo` | DEFERRED_GLOBAL |
| no-default-features compile failure | COMPILE_ERROR | `crates/vb_ui_model/src/envelope.rs`, `crates/vb_ui_model/src/emitter.rs` | No | `vb-bkgo` | DEFERRED_GLOBAL |

Landing decision: the rebase did not introduce any bead-local regression. State 15 may continue by pushing the scoped JJ change to a safe remote bookmark/branch because the local `main` bookmark is conflicted in another workspace.
