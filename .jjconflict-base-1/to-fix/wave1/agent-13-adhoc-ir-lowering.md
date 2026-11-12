# Wave 1 / Agent 13 — ad-hoc IR-lowering deep-dive

**Scope:** canonical IR lowering for v1 primitives (`set, do, choose, for_each, together, reduce, repeat, wait, ask, finish, collect`) in `crates/vb_compile/src/mod_compile_lowering/`.
**Method:** for each bug ID, read the bead, locate the touched source, map it to the IR-lowering concern, run a targeted test.

## Cross-cutting findings (apply to the whole chunk)

1. **All 11 canonical primitives are fully lowered.** Verified in
   `crates/vb_compile/src/compile/mod.rs:7-19` (re-export surface) and
   `crates/vb_compile/src/lower/mod.rs:7-11`:
   `lower_set, lower_do, lower_choose, lower_for_each, lower_together, lower_collect, lower_reduce, lower_repeat, lower_wait, lower_ask, lower_finish` —
   11 of 11 present.
2. **Compiler routes through `CompiledWorkflow::try_from_parts`** at three call sites in the production compiler:
   - `crates/vb_compile/src/mod_compile_lowering/part_01.rs:59` (top-level `compile_source`)
   - `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (digest-driven)
   - `crates/vb_compile/src/mod_compile_lowering/part_07.rs` (test harness)
   `from_parts_unchecked` is **not** reachable from any public vb_compile API; the
   proptest `vb_xi2f_compile_source_proptest` actively asserts the source does not
   contain `from_parts_unchecked`. The remaining `from_parts_unchecked` callsites
   live in `vb_runtime/src/engine/drive.rs`, `vb_runtime/src/engine/drive_tests.rs`,
   `vb_core/src/workflow/mod.rs` (definition), `vb_runtime/src/verification/kani/kani_resume_state_machine.rs`,
   and one workspace_tests integration test — none of which are the compiler.
3. **Nested body lowering supports more than `set`.**
   - `canonical_body_step_width` (`part_01.rs:142-152`) accepts `Set` and `Do`.
   - `emit_single_body_set` (`part_04.rs:213-297`) lowers `Set` and `Do`.
   - `emit_choose_body_step` (`part_14.rs:179-200`) lowers `Set` and `Do` for `choose` branches.
   - `emit_together_branches` (`part_03.rs:92-159`) uses `emit_single_body_set` for each branch.
   So nested bodies emit both `Set` and `Do` nodes — not just `set`.
4. **Health check:** `cargo test -p vb_compile --lib` → **454 passed, 0 failed, 4 ignored** (2.48s). Canonical suite: 31 passed, 0 failed.
5. **`from_parts_unchecked` total in tree:** 5 callsites — 0 in production compiler lowering.

## Per-bug findings

| bug-id | pri | primitive | lowered? | try_from_parts | nested-bodies | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|
| vb-vndpk | P3 | n/a (panic-equivalent sweep) | n/a | n/a | n/a | `cargo test -p vb_yaml --lib` (cited in bead close) | 306 passed per bead | PATCHED | `crates/vb_yaml/src`, `vb_boundary_inventory/src`, `vb_ipc/src` — no IR-lowering touch. Bead closed 2026-06-22; 203 assert!(false,...) → 0. |
| vb-vuebt | P0 | n/a (test-signature duplication) | n/a | n/a | n/a | `cargo test -p vb_runtime --lib timer_methods` | referenced in close reason | PATCHED | 3 fixed signatures in `vb_runtime/src/shard/impl_parts/timer_methods.rs:141`, `shard/tests/chunk_003.rs:88`, `shard/tests/chunk_013.rs:89`. Pure test-file repair, not lowering. |
| vb-w2wde | P0 | n/a (storage capacity panic) | n/a | n/a | n/a | `cargo test -p vb_workspace_tests --test doctor_storage_scan_decode_tests bounded_scan` | 8 passed per bead | PATCHED | `crates/vb_storage/src/journal/replay.rs:14-23,152-158` — `MAX_INITIAL_REPLAY_CAPACITY=4096` constant. `vb_storage`, not compiler. |
| vb-w3li7 | P3 | n/a (forward-edges validation) | n/a | n/a | n/a | `cargo test -p vb_core --lib workflow` | IN_PROGRESS | PARTIAL | `crates/vb_core/src/workflow/mod.rs` (`validate_loop_done_only`, `validate_forward_edges`). Validation layer, not IR lowering. Status IN_PROGRESS as of 2026-06-23. |
| vb-w9zav | P3 | n/a (test rename) | n/a | n/a | n/a | `cargo test -p vb_storage --lib batch::byte_accounting_tests` | 42 passed per bead | PATCHED | `byte_accounting_tests.rs:786` — naming only. Storage batch layer. |
| vb-wb05o | P3 | n/a (runtime admission dup) | n/a | n/a | n/a | none (dup of vb-12yr3) | dup-closed | PATCHED | `crates/vb_runtime/src/admission.rs` (`admit_artifact_run_with_certificate_floor`). Marked duplicate of vb-12yr3. |
| vb-wcbde | P3 | n/a (core/frame MaxAttempts) | n/a | n/a | n/a | `cargo test -p vb_core --lib ids` | IN_PROGRESS | PARTIAL | `crates/vb_core/src/ids/mod.rs` — `MaxAttempts::try_new`. Core ids layer, not compiler. |
| vb-wi486 | P2 | n/a (replay handlers) | n/a | n/a | n/a | `cargo test -p vb_core --lib replay` | CLOSED 2026-06-24 | PATCHED | `crates/vb_core/src/replay/basic/handlers/mod.rs:49` — invalid-jump-target error mapping. Replay engine, not lowering. |
| vb-wjtve | P0 | n/a (lru_ring) | n/a | n/a | n/a | `cargo check -p vb_runtime --lib` | clean per bead | PATCHED | `crates/vb_runtime/src/shard/lru_ring.rs:178-183,275-311`. Runtime shard, not compiler. |
| vb-wl1ut | P2 | n/a (runtime primitives together_start) | n/a | n/a | n/a | `cargo test -p vb_runtime --lib` | CLOSED | PATCHED | `crates/vb_runtime` (RP-003 saturating-add). Runtime primitive, not lowering. |
| vb-wplfj | P0 | n/a (clippy build-integrity dup) | n/a | n/a | n/a | `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings` | dup-closed of vb-oe7i1 | PATCHED | Build-integrity dup; clippy, not lowering. |

## Verdict distribution

- **PATCHED:** 9 (vb-vndpk, vb-vuebt, vb-w2wde, vb-w9zav, vb-wb05o, vb-wi486, vb-wjtve, vb-wl1ut, vb-wplfj)
- **PARTIAL:** 2 (vb-w3li7, vb-wcbde) — both IN_PROGRESS
- **NOT-PATCHED:** 0
- **UNKNOWN:** 0

## Primitives still missing canonical lowering

**None.** All 11 canonical primitives (`set, do, choose, for_each, together, reduce, repeat, wait, ask, finish, collect`) are fully lowered. `reduce` is exposed as `StepPrimitive::Aggregate` and lowered via `lower_canonical_aggregate` (`part_04.rs:15-83`).

## `from_parts_unchecked` usages still present (count)

**5 total, 0 in production compiler lowering.**
- `crates/vb_core/src/workflow/mod.rs` (definition + 1 use site)
- `crates/vb_runtime/src/engine/drive.rs` (production runtime path, with explicit comment that this is intentional for engine-internal construction)
- `crates/vb_runtime/src/engine/drive_tests.rs` (test)
- `crates/vb_runtime/src/verification/kani/kani_resume_state_machine.rs` (`kani_from_parts_unchecked` for Kani harnesses)
- `crates/workspace_tests/tests/integration_storage_runtime_validate_pipeline.rs` (test)
- The proptest in `crates/vb_compile/tests/vb_xi2f_compile_source_proptest.rs` actively asserts the vb_compile source does **not** contain `from_parts_unchecked`.

## Top-3 NOT-PATCHED

None of the 11 bugs in this chunk are NOT-PATCHED. The two PARTIAL (IN_PROGRESS) items are out of scope for IR lowering:
- **vb-w3li7 (P3, IN_PROGRESS)** — `validate_loop_done_only` in `vb_core/src/workflow/mod.rs`; core-workflow validation, not compiler.
- **vb-wcbde (P3, IN_PROGRESS)** — `MaxAttempts::try_new` in `vb_core/src/ids/mod.rs`; core ids, not compiler.

Both belong to core/workflow/core-frame surfaces; the IR-lowering layer has nothing to do with either.

## Bottom line

**No lowering defects found in this chunk.** The IR-lowering layer is healthy: 11/11 primitives lowered, `try_from_parts` enforced at the compiler boundary, nested bodies emit `Set` and `Do` (not just `set`), and `cargo test -p vb_compile --lib` reports 454 passed / 0 failed. The chunk's bugs all live in other subsystems (storage, runtime, validation, replay, ids) and have no overlap with the canonical IR-lowering concern.

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-13-adhoc-ir-lowering.md`
