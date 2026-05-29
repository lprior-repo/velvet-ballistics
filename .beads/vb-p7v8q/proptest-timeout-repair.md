# vb-p7v8q proptest timeout repair evidence

## Scope

Bounded storage-backed proptests that caused the Moon `test` lane to hit the
10 minute wrapper timeout. The affected tests were:

- `velvet-ballistics-workspace-tests::journal_side_index_contracts`
- `vb_storage proptests::ppi_001_deterministic_replay_invariant`
- `vb_storage proptest_integration::proptests::ppi_001_deterministic_replay_invariant`

## Changes

- `crates/workspace_tests/tests/journal_side_index_contracts.rs`
  - Added fixed CI case budgets for Fjall-backed property tests:
    - `JOURNAL_IO_PROPTEST_CASES = 64`
    - `JOURNAL_KEY_PROPTEST_CASES = 128`
  - Disabled file failure persistence for integration-test proptests to remove
    `FileFailurePersistence::SourceParallel` source-root lookup failures.
- `crates/vb_storage/src/proptests.rs`
  - Added `RECOVERY_IO_PROPTEST_CASES = 64` for the recovery PPI proptest block.
  - Disabled file failure persistence.
- `crates/vb_storage/src/po010_proptests.rs`
  - Added `DETERMINISTIC_REPLAY_CASES = 64` for the duplicate deterministic
    replay property.
  - Disabled file failure persistence.

## Command evidence

- `rustup run nightly-2026-04-28 rustfmt --edition 2024 --check crates/workspace_tests/tests/journal_side_index_contracts.rs crates/vb_storage/src/proptests.rs crates/vb_storage/src/po010_proptests.rs`
  - PASS
- `rustup run nightly-2026-04-28 cargo nextest run -p velvet-ballistics-workspace-tests --test journal_side_index_contracts`
  - PASS: 11 tests passed, 0 skipped; nextest summary 1.122s after compile.
- `rustup run nightly-2026-04-28 cargo nextest run -p vb_storage proptests::ppi_001_deterministic_replay_invariant`
  - PASS: 2 tests passed, 1151 skipped; nextest summary 2.953s after compile.
- `moon run velvet-ballistics:test`
  - PASS: 12759 tests passed, 27 skipped; task completed in 4m 4s.
- `moon ci`
  - BLOCK_GLOBAL: test lane passed with 12759 tests passed, 27 skipped; CI still failed due unrelated global gates.
  - Remaining observed blockers:
    - `fuzz-smoke`: `fuzz_targets/journal_cancel_kill_record_kind.rs` uses stale `encode_record` call shape.
    - `fmt`: pre-existing formatting drift in unrelated files.
    - `ignored-fallible-results`: unreadable input `crates/vb_runtime/src/verification/kani/cancel_kill_lattice.rs`.
    - `source-length`/`lint-src`/`banned-token-gates`/`hardened-build`: skipped after dependency failures.
  - Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e7390f557001QpPTGy1gIKO4PR`

## Performance layer

No runtime performance claim. This is a CI boundedness repair for test resource
usage only; no benchmark/profiler evidence is claimed.

## Residual risk

The storage-backed proptest case budgets are now deterministic CI budgets rather
than exhaustive stress budgets. Larger stress campaigns should run as separate,
explicit long-running jobs rather than inside the 10 minute Moon `test` lane.
