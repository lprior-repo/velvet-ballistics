bead_id: vb-qi37.4.3
phase: State 8 classification after State 13 refactor and rebase repair
updated_at: 2026-05-11T00:00:00Z

# Regression Diff

## Classification
- `BLOCK_RELEASE` / downstream State 8 not fully green for landing because this bead is release-critical and `moon ci` remains red.
- The observed red items are outside the State 13 mechanical split and outside the scoped touched files for the line-count repair; prior artifacts already classified unrelated workspace Moon debt as deferred/global.

## Passing bead-local evidence
- `moon run :quick` passed.
- `moon run :test` passed with 9857/9857 tests passing.
- Focused durability/admission tests passed; see `moon-report.md`.
- `moon ci` source-length task completed after the split, proving the previous State 13 line-count blocker is removed for touched/scoped split files.

## Current blocking evidence
- `moon ci` output: `/home/lewis/.local/share/opencode/tool-output/tool_e19eec513001LhIUgUOPhQLcS1`.
- `lint-src`: `crates/vb_proof_kernels/src/envelope_header.rs` clippy `new_without_default`.
- `lint-src`: `xtask/src/proof.rs` clippy `panic_in_result_fn` and `panic`.
- `feature-powerset`: `crates/vb_ui_model/src/envelope.rs` missing `Vec` in no-default-features mode.
- `feature-powerset`: `crates/vb_ui_model/src/emitter.rs` and `crates/vb_ui_model/src/envelope.rs` invalid module-level `#![no_std]` attribute.

## Follow-up text
- Continue rerun from State 8 after resolving or formally classifying the global `moon ci` red items. Do not advance to State 14/15 until the downstream gate policy is satisfied.

## 2026-05-11 rebase repair classification
- Previous `moon ci` global red items are no longer present after rebase onto `main` `c993943126cc` and local split/schema repair.
- `moon ci` passed with 19 completed tasks and 0 failed tasks; output: `/home/lewis/.local/share/opencode/tool-output/tool_e1a0aaf70001OZ4gLQnSoCc4xB`.
- Classification: no active `BLOCK_RELEASE`/`BLOCK_LOCAL` from State 8 rerun. Continue downstream States 9-14.
