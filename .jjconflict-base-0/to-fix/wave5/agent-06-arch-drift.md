# Wave 5 — Architecture Drift / IPC / CLI — Agent 06

Scope: vb-goqvb, vb-hfwjr.1, vb-igldl, vb-jhkez
Working dir verified: `/home/lewis/src/velvet-ballistics`
Mode: read-only review, no source edits, no beads.

## Findings Table

| bug-id | pri | source-fix | test | fix-file | fix-fn-lines | file-len | drift? | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| vb-goqvb | P2 | `crates/vb_storage/src/recovery/hydrate_support.rs:145-185` `decode_snapshot_slots` builds `taint_by_slot: HashMap<vb_core::SlotIdx, vb_core::Taint>` at L167–170 (pre-indexes taint in O(N+M) merge) and looks up via `taint_by_slot.get(&slot)` at L173 (O(1) per slot). Per-slot `Vec::iter().find_map(...)` is gone. | `cargo test -p vb_storage --lib snapshot_with_empty_slots_populated_taint --no-fail-fast` | `crates/vb_storage/src/recovery/hydrate_support.rs` | 41 (`decode_snapshot_slots` L145–185) | 484 | y (file >300 by 184; fn >25 by 16; cross-context `vb_storage/recovery`) | `cargo test -p vb_storage --lib snapshot_with_empty_slots_populated_taint --no-fail-fast` | 1 passed; 0 failed; 1272 filtered out | PATCHED | hydrate_support.rs:167 builds HashMap; lookup is O(1) at L173; full snapshot suite (78) reported green in prior waves; cargo test confirmed 1 representative passes |
| vb-hfwjr.1 | P0 | `crates/vb_compile/src/mod_compile_lowering/part_04.rs` exists as a flat file; `mod part_04;` (crate-private) + `pub(crate) use part_04::*;` (crate-visible re-export). Submodules `compound`, `body_dispatch`, `reduce_chain` referenced in the bead no longer exist (refactored away), so `-D unreachable-pub` no longer fires on the lowering tree. | `cargo check -p vb_compile --lib` + `cargo fuzz check` (builds all fuzz targets incl. `ipc_frame`) | `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | n/a (no single fix-fn; visibility is at module/wiring level) | 312 | y (file >300 by 12; DDD boundary is `vb_compile/lowering`, consistent with bounded context) | `cargo check -p vb_compile --lib`; `cargo fuzz check` | `cargo check -p vb_compile --lib` → Finished dev profile in 1.00s; `cargo fuzz check` → Finished release profile, all crates including vb_compile/vb_ipc built clean (no `unreachable-pub` errors) | PATCHED | mod_compile_lowering.rs L6 uses `mod part_04;` (private), L41 uses `pub(crate) use part_04::*;`; fuzz workspace builds clean |
| vb-igldl | P1 | `crates/vb_runtime/src/recovery.rs:73-83` `reject_unsupported_live_frame_state` returns `RuntimeError::InvalidRecoveryHydration` for **all** unsupported cases (`slot_values \|\| slot_taint \|\| action_payloads \|\| pending_actions`). Bead close-reason claims it now distinguishes `UnsupportedFullRecoveryHydration` for slot_taint-only — **that distinction is NOT in the source.** | `cargo test -p velvet-ballistics-workspace-tests --test integration_storage_runtime_recovery recovery_detects_unsupported_slot_taint --no-fail-fast`; `--test integration_storage_runtime_validate_pipeline runtime_boundary_rejects_unsupported_slot_taint_in_pipeline --no-fail-fast` | `crates/vb_runtime/src/recovery.rs` | 11 (`reject_unsupported_live_frame_state` L73–83) | 190 | n (fn <=25, file <=300, stays in `vb_runtime/recovery`) | `cargo test -p velvet-ballistics-workspace-tests --test integration_storage_runtime_recovery recovery_detects_unsupported_slot_taint --no-fail-fast` (and the pipeline counterpart) | recovery test: 1 passed, 12 filtered; pipeline test: 1 passed, 14 filtered | PARTIAL | Tests pass because both use `assert!(result.is_err())` (weak, tautological). Production source still returns the broad `InvalidRecoveryHydration` for slot_taint, contradicting close-reason claim. |
| vb-jhkez | P2 | `crates/vb_ipc/src/tests.rs:14-32` and `crates/vb_ipc/src/frame/tests.rs:5-12` still DEFINE `assert_ok!` and `prop_assert_ok!` macros. Actual call-site counts: tests.rs has 88 `assert_ok!` + 6 `prop_assert_ok!` = 94; frame/tests.rs has 36 `assert_ok!` + 0 `prop_assert_ok!` = 36; total 130 (bead said 125). `matches!()` appears 3× in tests.rs and 0× in frame/tests.rs — replacement did not happen. | `cargo test -p vb_ipc --lib frame`; `cargo test -p vb_ipc --lib payload` (regression tests still exercise the macro paths) | `crates/vb_ipc/src/tests.rs` + `crates/vb_ipc/src/frame/tests.rs` | n/a (file-level test refactor) | tests.rs 1915; frame/tests.rs 1314 | y (both test files are >300 by 1615 and 1014 respectively; technically tests are not production, but bead asserts cleanup that did not occur) | `cargo test -p vb_ipc --lib frame --no-fail-fast`; `cargo test -p vb_ipc --lib payload --no-fail-fast` | frame: 163 passed, 0 failed; payload: 104 passed, 0 failed | NOT-PATCHED | Macro definitions and 130 call-sites remain intact in source; tests still pass *because* the macros are in use. Bead claim of `matches!()` replacement is not reflected in the source. |

## Summary

- bugs-checked: 4
- PATCHED: 2 (vb-goqvb, vb-hfwjr.1)
- PARTIAL: 1 (vb-igldl)
- NOT-PATCHED: 1 (vb-jhkez)
- UNKNOWN: 0
- drift-introduced cases: 3
  - vb-goqvb — `hydrate_support.rs` 484 LoC (flag >300); `decode_snapshot_slots` 41 LoC (flag >25); DDD context `vb_storage/recovery` is correct.
  - vb-hfwjr.1 — `part_04.rs` 312 LoC (flag >300); cross-context question is moot since visibility is now wired correctly at the module level (`mod part_04;` + `pub(crate) use`).
  - vb-jhkez — `tests.rs` 1915 LoC, `frame/tests.rs` 1314 LoC (both heavily over 300); the cleanup is the entire purpose of the bead and did not occur.

## Top NOT-PATCHED with one-line reason

1. **vb-jhkez (P2)** — `assert_ok!` / `prop_assert_ok!` macros still defined and still invoked 130 times across `tests.rs` + `frame/tests.rs`; replacement to `matches!()` never executed.
2. **vb-igldl (P1)** — production `reject_unsupported_live_frame_state` returns `InvalidRecoveryHydration` for every unsupported flag; the close-reason's `UnsupportedFullRecoveryHydration` branch for slot_taint-only is not in source.
3. *(none — only one NOT-PATCHED in this chunk)*

## File Path Written

`/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-06-arch-drift.md`