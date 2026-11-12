# Wave 5 — Flux-RS Reviewer (chunk 05, 5 bugs)

- Reviewer: flux-rs (refinement types) reviewer
- Date: 2026-06-24
- Working dir: `/home/lewis/src/velvet-ballistics`
- Bug IDs reviewed: vb-dk9lq, vb-eawfo, vb-fkbx8, vb-fsewl, vb-gnjpp

## Flux surface inventory

Two files in the touched crates carry flux annotations:

- `crates/vb_storage/src/codec/flux_validation.rs` — 14 `#[flux_rs::trusted]` model functions (record-kind validation, replay contiguity, kind-id stability)
- `crates/vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` — 12 `#[flux_rs::trusted]` cancel/kill lattice models

No `#[flux_rs::ignore]` annotations exist anywhere in the workspace. No `#[flux_rs::opaque]` either. Every trusted model is paired with a unit test that executes the production call site, so the models remain non-vacuous.

The five bugs in this chunk all land on non-flux code paths:

- `vb-dk9lq` — xtask test gate, vb_compile validate, vb_storage preview/queue/types/recovery (no `flux_rs` in any of these files).
- `vb-eawfo` — `vb_storage/src/recovery/hydrate_support.rs:145` (`decode_snapshot_slots`). Function is `pub(super)`, plain Rust, no flux annotations. The fix replaces the O(N·M) `iter().find()` taint probe with a single `HashMap` pre-index; both shapes are flux-friendly, but no annotation exists in the file.
- `vb-fkbx8` — `vb_core/src/ids/mod.rs:9` single-canonical `numeric_id!` macro. No flux annotations.
- `vb-fsewl` — `kani_shard_lifecycle_harnesses.rs` (788 lines after de-dupe). Pure Kani harnesses, no flux surface.
- `vb-gnjpp` — `vb_runtime/src/shard/impl_parts/chunk_001.rs:215-224` (`advance_journal_sequence`). Now uses `checked_add`, returns `SequenceOverflow`. No flux annotations.

## Results table

| bug-id | pri | flux-surface | source-fix | test | flux-cmd | flux-result | cargo-result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|
| vb-dk9lq | P1 | NO | xtask `release_rendering.rs:14-28` single `.parent()`; vb_compile `validate.rs:229` `bytes.get`; vb_storage `preview.rs:129` `u64::MAX` sentinel; vb_storage `queue/writer.rs:197` `mem::take`; vb_storage `types/index.rs:49` `checked_sub`; vb_storage `recovery_ops.rs:136` collapsible-if → `&&` | `cargo test -p velvet-ballistics-workspace-tests --test vb_nf2u_ui_release_acceptance` | n/a (no flux path) | n/a | 8 passed, 0 failed | PATCHED | `all_eight_screens_pass_reachability_and_overlap_gates ok`, `secret_values_are_redacted_in_every_screen ok`, plus 6 more |
| vb-eawfo | P2 | NO (adjacent to flux_validation.rs but decode_snapshot_slots is plain Rust) | `vb_storage/src/recovery/hydrate_support.rs:145` pre-indexes taint via `HashMap<SlotIdx, Taint>` then O(1) lookup per slot — O(N+M) | `cargo test -p vb_storage --lib hydrate_run_frame` | `bash scripts/flux-check-package.sh vb_storage` | clean (2.55s) | 37 passed, 0 failed (1236 filtered) | PATCHED | `hydrate_run_frame_rejects_corrupt_snapshot_slots_bytes ok`, `hydrate_run_frame_states_merge_snapshot_and_tail ok`, `hydrate_run_frame_taint_preserved_when_tail_has_no_taint ok`, plus 34 more hydrate tests |
| vb-fkbx8 | P4 | NO | `crates/vb_core/src/ids/mod.rs:9` single canonical `numeric_id!` macro; downstream `workflow_ids.rs`/`symbol_ids.rs`/`storage_ids.rs`/`index_ids.rs` removed (consolidated); `ids/mod.rs:54-67` single macro call site per id type | `cargo test -p vb_core --lib ids::` and `numeric_id` filter | `bash scripts/flux-check-package.sh vb_core` | clean (1.06s) | ids::tests: 96 passed, 0 failed; `cf013_all_numeric_id_types_construct_via_single_macro ok`, `cf013_from_str_round_trip_for_all_numeric_id_types ok` | PATCHED | `ids::tests::cf013_all_numeric_id_types_construct_via_single_macro ok`; macro_rules! appears once at `mod.rs:9` |
| vb-fsewl | P0 | NO (Kani, not Flux) | `kani_shard_lifecycle_harnesses.rs` de-duplicated from 1140 → 788 lines; line 325 syntax error fixed; 12 `#[kani::proof]` harnesses (`po-001`..`po-008` plus extras) preserved | `cargo check -p vb_runtime --lib` (kani file not gated by `#[cfg(kani)]` in `mod.rs` — only compiled by `cargo kani`); manual structural verification (12 kani proofs, no duplicate bodies, no parse error) | n/a | n/a | `cargo check -p vb_runtime --lib` clean | PATCHED | 12 `#[kani::proof]` proofs at lines 180, 217, 277, 327, 374, 422, 459, 499 — no duplicate paste; file is 788 lines (was 1140) |
| vb-gnjpp | P1 | NO (production path uses `checked_add`; flux_validation.rs contiguity model uses `saturating_add` on test inputs but is bounded by unit tests) | `vb_runtime/src/shard/impl_parts/chunk_001.rs:215-224` — `advance_journal_sequence` now uses `seq.get().checked_add(1).map(EventSeq::new).ok_or(JournalError::SequenceOverflow)`; `vb_storage/src/codec/mod.rs:141` `next_seq` already used `checked_add` | `cargo test -p vb_runtime --lib journal` and `cargo test -p vb_storage --lib next_seq` | `bash scripts/flux-check-package.sh vb_runtime` and `vb_storage` | both clean (0.63s, 2.55s) | vb_runtime journal: 36 passed, 0 failed; vb_storage next_seq: 8 passed, 0 failed (`next_seq_max_returns_overflow ok`, `next_seq_max_minus_one_returns_max ok`, `next_seq_rejects_overflow ok`, `next_seq_returns_sequence_overflow_at_max ok`) | PATCHED | `JournalError::SequenceOverflow` returned at `chunk_001.rs:220`; `next_seq_max_minus_one_returns_max` proves `MAX-1 + 1 == MAX`; `next_seq_rejects_overflow` proves `MAX + 1` returns `Err(SequenceOverflow)` |

## Flux-abuse cases

None. Specifically:

- No `#[flux_rs::ignore]` annotations anywhere in the workspace.
- No `#[flux_rs::opaque]` annotations.
- The `#[flux_rs::trusted]` annotations (14 in `vb_storage::codec::flux_validation`, 12 in `vb_runtime::shard::lifecycle::flux_cancel_kill`) are concentrated in dedicated model files. Each trusted model delegates to a production call site and is paired with a unit test that executes the production function. None of the wave-5 bug fixes introduced new trusted/ignore annotations.
- `flux_validation.rs:142,158` uses `saturating_add(1)` inside trusted contiguity models. This is safe because `saturating_add(1)` on `u64::MAX` returns `u64::MAX`, so a `[MAX-1, MAX]` pair is correctly classified as contiguous and a `[MAX, X != MAX]` pair is correctly classified as non-contiguous. The pair is exercised by `contiguous_sequence_passes`, `gap_sequence_detected`, and `duplicate_sequence_detected`.

## Counts

- bugs-checked: 5
- PATCHED: 5
- NOT-PATCHED: 0
- PARTIAL: 0
- UNKNOWN: 0

## Top-3 NOT-PATCHED

(none — all five bugs verified patched)

## Notes / risk

- `vb-dk9lq` and `vb-fsewl` were closed before the bug-hunt doc directory (`bug-hunt-2026-06-21/findings/`) was removed from the tree; adjudication evidence relies on bead close-reasons plus regression tests rather than the original SR/CF/RE/ARCH doc itself. This is fine because the bead close reasons cite specific source-line fixes and the regression tests reproduce.
- `vb-gnjpp`'s bead description cites line ranges `chunk_001.rs:275-278` and `chunk_002.rs:340-343`, but the production arithmetic has since been moved to `vb_runtime/src/shard/impl_parts/chunk_001.rs:215-224` (`advance_journal_sequence`) and `vb_storage/src/codec/mod.rs:141` (`next_seq`). The current production path uses `checked_add` and returns `JournalError::SequenceOverflow`, which is the intended fix.
- The flux surface is dominated by `#[flux_rs::trusted]` model functions. Each trusted model is paired with a runtime test, satisfying the proof/test/source bridge requirement and avoiding the "vacuous trusted" anti-pattern.

## File path

`/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-05-flux-rs.md`
