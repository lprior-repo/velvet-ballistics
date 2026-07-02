# Wave 5 — Agent 12: Ad-hoc File-Size Deep-Dive

**Sweep date:** 2026-06-24
**Working dir:** `/home/lewis/src/velvet-ballistics`
**Role:** file-size-expert (file length, hot function length, source-length gate, deferred codegen residue, duplicate module trees)
**Method:** Read-only. No source modified. No beads created.

## Source-length gate baseline

`bash scripts/check-source-length.sh` returns **3 active violations** + **1 hot-function violation** at sweep time:

| path | lines | ledger row |
|---|---|---|
| `crates/vb_core/src/span.rs` | 366 | none |
| `crates/vb_runtime/src/trace.rs` | 327 | none |
| `crates/vb_storage/src/preview.rs` | 359 | none |
| `crates/vb_runtime/src/error/equality.rs:176` | 26 logical | n/a (hot-fn) |

Ledger baseline `.config/source-length-exceptions.txt` carries **484 rows** (101 KB). 30 rows mention vb_yaml / vb_boundary_inventory / vb_ipc. None of the four bug IDs in this chunk introduce new violations.

## Deferred codegen residue

`vb_codegen/` does **not exist** in production crates. `crates/vb_compile/src/` uses the required `mod_compile_core.rs / mod_compile_errors.rs / mod_compile_lowering.rs / mod_compile_validation.rs` split — `compile_core_impl.rs` is absent. No `generated/` or `perf/` directories under `crates/*/src/`.

Pattern flagged but not gate-breaking: `crates/vb_runtime/src/shard/{impl_,lifecycle}.rs` are 13-line shells that `include!()` chunk files from sibling directories (`impl_parts/`, `impl_tests/`, `lifecycle/`, `lifecycle_tests/`). This is a "deferred include" idiom rather than true codegen; chunks are git-tracked and ledger-exceptioned.

## Duplicate module trees

- `crates/vb_runtime/src/shard/helpers_main.rs.bak` — **2456 lines**, git-tracked, not declared in `shard/mod.rs`. Was deleted in commit `95b945f3e` and re-introduced later. This is the only true duplicate-tree residue in the wave scope. No active reference.
- `shard/types.rs` = 1994 lines (ledger baseline 634) — debt grew 1360 lines but exception still covers it.
- No `helpers_main.rs.bak` analogue exists in `vb_yaml`, `vb_boundary_inventory`, `vb_ipc`, or `vb_storage` source trees.

## Per-bug verdicts

| bug-id | pri | file-len-current | fn-len-current | source-length-gate | duplicate-tree | deferred-residue | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| vb-vndpk | P3 | vb_yaml max 413 (ast/types.rs), vb_binv max 233, vb_ipc max 222 (action_output.rs, non-test) | longest hot fn in scope ≤25 logical | active for vb_yaml/binv/ipc files (all in ledger) | none in scope | none in scope | `cargo test -p vb_yaml --lib` / `-p vb_boundary_inventory --lib` / `-p vb_ipc --lib` | 228 / 202 / 540 pass | PATCHED | assert!(false,...) eliminated from vb_yaml (0 sites) and vb_boundary_inventory (0 sites); 28 remaining in vb_ipc, all inside `#[test]` blocks (action_output.rs:124-170, ids/tests.rs:133-327, server/impl_tests.rs:266-1418, server/trace.rs:396-504). Close-reason note "Used match-based panics and panic!() in test contexts" allows test-only sites; no production-code panic-equivalent survives. |
| vb-vuebt | P0 | chunk_003.rs 294, chunk_013.rs 369 — both test paths, ledger-excluded | n/a (test files) | chunk_003.rs not in ledger (293 baseline), chunk_013.rs not in ledger (369 baseline < 300 gate applies but file matches `*/tests/**` so excluded); no `timer_methods.rs` exists | none | none | `cargo test -p vb_runtime --lib finished_run_releases_frame_to_dimension_pool` and `... shard_submit_with_inputs_seeds_slots_and_drives` | both pass | PATCHED | `chunk_003.rs:94 fn finished_run_releases_frame_to_dimension_pool() -> Result<(), String>` has only one return-type clause. `chunk_013.rs:89 fn shard_submit_with_inputs_seeds_slots_and_drives() -> Result<(), &'static str>` has only one. The originally cited `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs:141 fn timer_fired_command_returns_none_when_no_pending_timer` no longer exists in the source tree (`impl_parts/` has chunk_001-004 only, no `timer_methods.rs`); `rg "fn .*\\(\\) -> .* -> .*\\{" crates/` returns zero matches. |
| vb-widdi | P2 | `crates/vb_storage/src/trimming/logic.rs` = 388 (ledger row 233 baseline 307, ledger exception valid) | `pub fn latest_durable_snapshot_seq` lines 17-57 = **33 logical lines** (exceeds 25-line hot-fn limit; file is NOT in hot_files() so gate does not flag); 3 other fns in same file >25 logical (trim_events_for_run 43, trim_eligibility_diagnostic 51, compute_retained_terminal_runs 62) | ledger row 233 covers file; no hot-fn violation emitted (vb_storage not in hot_files globs) | none in scope | none | `cargo test -p vb_storage --lib trimming::tests::latest_durable_snapshot_seq_reads_max_key_without_decoding_value` | PASS (37/37 trimming tests pass) | **NOT-PATCHED** | Bug-hunt fix (reverse prefix scan with key-only lookup) introduced in commit `7586b096f` "wave-8 — 17 storage P0 ..." then **reverted** in commit `944b95d5c` "femdation: round 8 production fixes". Current source lines 17-56 still calls `decode_record` for every snapshot and validates `snapshot.run == run` / `snapshot.seq == key_seq`, which is the exact SC-004 behavior the close-reason claims was removed. Round-8 commit diff shows: removed `self.run_snapshot.prefix(prefix_key).next_back()` pattern, restored the `for item in self.run_snapshot.prefix(prefix_key)` loop with `decode_record`. Test passes because behavior is correct; perf bug (O(N · payload_size) per run) is intact. |
| vb-wplfj | P0 | n/a (clippy not file-size) | n/a | n/a | n/a | n/a | n/a | n/a | PATCHED (via duplicate) | Closed as duplicate of `vb-oe7i1` which reports full workspace `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings` passes. Note: subsequent per-crate clippy runs in this sweep emit errors (`vb_runtime` 122+4 errors, `vb_storage` 28+7+49 errors), so the original "workspace green" state has regressed but the duplicate bead itself remains closed. Out of scope for file-size-expert. |

## Summary

- **bugs-checked:** 4
- **pass:** 3 (vb-vndpk, vb-vuebt, vb-wplfj)
- **not-patch:** 1 (vb-widdi)
- **partial / unknown:** 0
- **files still >300 lines (this wave's scope):** `crates/vb_storage/src/trimming/logic.rs` (388, ledger exception valid) — note the ledger baseline is 307 so file grew 81 lines past baseline without re-baselining.
- **hot functions still >25 logical lines (this wave's scope):** `trimming/logic.rs::{latest_durable_snapshot_seq (33), trim_events_for_run (43), trim_eligibility_diagnostic (51), compute_retained_terminal_runs (62)}`. File is excluded from `hot_files()` globs (which only watch `vb_runtime`, `vb_cli`, `engine`, `runtime`, `generated`, `perf`), so the gate does not emit these violations.

## Top NOT-PATCHED

1. **vb-widdi** — `latest_durable_snapshot_seq` still decodes every snapshot value. Wave-8 fix (commit `7586b096f`) was reverted by round-8 commit `944b95d5c`. Source at `crates/vb_storage/src/trimming/logic.rs:21-53` still iterates the full snapshot prefix and calls `decode_record` per item, restoring the O(N · payload_size) cost and the `decode_record`-per-snapshot BLAKE3 + postcard work that SC-004 was opened to remove. The regression test (`trimming/tests.rs:403 latest_durable_snapshot_seq_reads_max_key_without_decoding_value`) only asserts the returned sequence number, not that the value path is skipped, so the bug is invisible to CI. Re-applying the wave-8 patch (replace forward scan with `self.run_snapshot.prefix(prefix_key).next_back()` and drop `decode_record`/`MAGIC_SNAPSHOT`/`MAX_SNAPSHOT_BYTES` imports) will close vb-widdi and shrink the file to ~250 lines.
2. **Deferred** — `crates/vb_runtime/src/shard/helpers_main.rs.bak` (2456 lines, git-tracked) is duplicate residue, but is unrelated to any bug in this chunk.
3. **Debt growth** — `crates/vb_runtime/src/shard/types.rs` is now 1994 lines against a 634-line ledger baseline (3.1× growth). Not a bug ID but a file-size gate risk for future waves.

## Output file

Written: `/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-12-adhoc-file-size.md`
