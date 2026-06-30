# Wave 4 — Agent 06 Architectural Drift Review

Beads audited: 6 (vb-dyulo, vb-dzibx, vb-e4uxt, vb-ebpmk, vb-eggv8, vb-esbvj)
Date: 2026-06-24 · Working dir: /home/lewis/src/velvet-ballistics

## Per-Bead Audit Table

| bug-id | pri | source-fix | test | fix-file | fix-fn-lines | file-len | drift? | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|
| vb-dyulo | P2 | none (slug/codec.rs + compiled_query/mod.rs DELETED in refactor `34ac72d3c` and `3a35fb77b`; `compiled_workflow.rs:27-42` only has `validate_parts`+`validate_budget` calls but no count-limit guard ordering) | none present in repo | `crates/vb_core/src/compiled_workflow.rs` (renamed) | `try_from_parts` ~16 lines (in scope) | 228 | n (no current drift in existing file) | `bd show vb-dyulo` + `find compiled_slug/codec.rs` | files absent; CW-011 source fix never landed in current tree | NOT-PATCHED | bead IN_PROGRESS; slug/codec.rs missing; compiled_query/mod.rs missing |
| vb-dzibx | P0 | none production-bound across audited crates (no `extern_spec`/`assume_specification` found in 92 `verification/verus/*.rs`; 0 import production crate; 0 `BINDING` markers; per-bead-note "This does not close vb-dzibx full L4; remaining hard Verus replacement scopes stay under vb-god2f") | none (proof artifact only) | `verification/verus/budget_monotonic.rs` (representative) | spec/proof fns all `verus!{}` mirror-only | 4 KB each (sub-300) | n (size) but **logical drift**: 92 mirror files vs. 0 production bindings | `find verification/verus -name "*.rs" \| xargs grep -l "extern_spec\|assume_specification"` → 0 matches | L4 production-bound obligation remains unmet; retired/downgraded only | NOT-PATCHED | `.beads/vb-dzibx` notes 2026-06-18 + grep counts |
| vb-e4uxt | P1 | `crates/vb_runtime/src/models/loom/journal_writer_queue.rs` — `JournalWriterQueue` struct + impls use `fn new/try_append/drain/pending/check_invariants` (private, no `pub`); wired into `crates/vb_runtime/src/models/loom/mod.rs:22` as `pub mod journal_writer_queue;` | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib --all-features models::loom` (per `.moon/tasks/loom.yml`) | `crates/vb_runtime/src/models/loom/journal_writer_queue.rs` | max fn = `journal_writer_queue_concurrent_append` 30 lines / `journal_writer_queue_at_capacity` 24 lines (most ≤25) | 134 | y — fn `journal_writer_queue_concurrent_append` 30 lines (>25) | `RUSTFLAGS="-Dwarnings --cfg loom" TMPDIR=$PWD/target/loom-tmp cargo test -p vb_runtime --lib --all-features models::loom -- journal_writer --nocapture` | `13 passed, 0 failed` | PATCHED | loom compile + test green; `journal_writer_queue_*` tests pass under `--cfg loom` |
| vb-ebpmk | P1 | `crates/vb_core/src/engine/signals.rs:108` — `EngineSignal::AwaitingAction` is a **unit variant**, not a struct variant. Test file `vb_test_core_workflow_slot_behavior.rs` already uses `assert_eq!(signal, EngineSignal::AwaitingAction)` (lines 351, 464, 1019) with no struct fields | `cargo check --workspace --lib --all-targets` (verification lane) | `crates/vb_core/src/engine/signals.rs` | enum def + 12 test fns ≤25 lines | 605 | n | `grep AwaitingAction crates/workspace_tests/tests/vb_test_core_workflow_slot_behavior.rs` → 3 unit-variant uses; `cargo check --workspace --lib --all-targets` runs clean for vb_core | engine_signal equality test `engine_signal_all_suspension_variants_are_distinct` passes | PATCHED | unit-variant confirmed; no struct fields; `assert_eq!(EngineSignal::AwaitingAction)` compiles |
| vb-eggv8 | P1 | `crates/vb_storage/src/preview.rs` fmt drift fixed at commit `4a0b93d1a`; `vb_runtime` test `handle_resume_propagates_flush_evidence_failure` passes (Wave-1 RA-018 typed-error refactor already resolved upstream contract) | `cargo fmt --check` and `cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation handle_resume_propagates_flush_evidence_failure` | `.evidence/vb-eggv8/evidence.yaml` | n/a (no single fn) | 4.1 KB evidence | n | `cargo fmt --check` exit 0; `cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation handle_resume_propagates_flush_evidence_failure` → 1 passed | fmt + targeted runtime test green; **moon ci deferred to Wave 5**; pre-existing lint failures outside scope (`vb_storage/src/preview.rs:359` >300, `vb_core/src/span.rs:366` >300, `vb_runtime/src/trace.rs:327` >300) | PARTIAL | `.evidence/vb-eggv8/evidence.yaml` status: DONE; bead still IN_PROGRESS in `bd`; pre-existing issues documented but not closed |
| vb-esbvj | P1 | **NOT APPLIED** — `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs` exists but `crates/vb_runtime/src/verification/kani/mod.rs` declares only 4 modules (`kani_retry_math`, `kani_for_each_ordering`, `kani_together_ordering`, `kani_engine_signals`). `kani_shard_lifecycle_harnesses` is NOT wired (orphaned). | `cargo check -p vb_runtime --tests --features kani-shard-command-queue` or similar — not run because file not wired | `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs` | multiple kani fns >25 lines: `make_minimal_run_state` 68, `kani_next_generation_monotonicity` 46, `kani_terminal_run_rejects_completion` 50, `kani_retry_exhaustion` 54, `kani_retry_terminal_typing` 59, `kani_retry_convergence` 66, `kani_validate_action_completion_step_state` 65 | **788** (>300) | **y** — file 788 lines (limit 300) AND 7 fns >25 lines AND NOT wired into `mod.rs` | `grep kani_shard_lifecycle_harnesses crates/vb_runtime/src/verification/kani/mod.rs` → 0 matches | orphaned, drift introduced | NOT-PATCHED | `mod.rs` line count 6; kani_shard_lifecycle_harnesses.rs 788 lines; `bd show` says CLOSED but wiring missing |

## Summary

- **bugs-checked**: 6
- **PATCHED**: 2 (vb-e4uxt, vb-ebpmk)
- **PARTIAL**: 1 (vb-eggv8)
- **NOT-PATCHED**: 3 (vb-dyulo, vb-dzibx, vb-esbvj)
- **UNKNOWN**: 0
- **drift-introduced cases**: 1 (vb-esbvj: 788-line orphan file with 7 oversized fns)

## Top-3 NOT-PATCHED Reasons

1. **vb-esbvj** — `kani_shard_lifecycle_harnesses.rs` (788 lines, 7 fns >25 lines) remains orphaned: `verification/kani/mod.rs` does NOT declare it. Bead is CLOSED but the wiring half of ARCH-W0-01 was never applied.
2. **vb-dzibx** — L4 production-bound Verus obligation remains unmet: 92 mirror files in `verification/verus/` carry 0 `extern_spec`/`assume_specification`/`BINDING` markers; bead notes explicitly defer to `vb-god2f`.
3. **vb-dyulo** — CW-011 source files (`compiled_slug/codec.rs`, `compiled_query/mod.rs`) were deleted in prior refactors; the count-limit-before-allocation fix never landed in the renamed `compiled_workflow.rs`.

## Architectural Drift Findings

- **vb-esbvj** is the only case where the bead is CLOSED but drift was **introduced or preserved**: file is 788 lines (>300), contains 7 functions >25 lines, and is not wired into `mod.rs`. This violates both Scott Wlaschin DDD boundary (orphan file with no module connection) and Holzman Rust function/file size discipline.
- **vb-e4uxt** has minor drift: `journal_writer_queue_concurrent_append` is 30 lines (>25), but file total (134) is within limits and loom gate is green.
- **vb-dyulo, vb-dzibx** show **logical drift** at the spec/contract level: deleted files or unbound proof artifacts. Not file-size drift but contract drift.
- **vb-eggv8** has pre-existing file-size drift in `vb_storage/src/preview.rs:359`, `vb_core/src/span.rs:366`, `vb_runtime/src/trace.rs:327` — all >300 lines. Out of bead scope per `.evidence/vb-eggv8/evidence.yaml`.

## Targeted Commands Run

```bash
# vb-e4uxt
RUSTFLAGS="-Dwarnings --cfg loom" TMPDIR=$PWD/target/loom-tmp \
  cargo test -p vb_runtime --lib --all-features models::loom -- journal_writer --nocapture
# → 13 passed, 0 failed

# vb-ebpmk
grep -n AwaitingAction crates/workspace_tests/tests/vb_test_core_workflow_slot_behavior.rs
# → 3 unit-variant uses; struct-form does not exist

# vb-eggv8
cargo fmt --check                                                    # exit 0
cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation \
  handle_resume_propagates_flush_evidence_failure                    # 1 passed

# vb-esbvj (drift detection)
grep kani_shard_lifecycle_harnesses crates/vb_runtime/src/verification/kani/mod.rs
# → 0 matches (orphaned)
wc -l crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs
# → 788

# vb-dzibx (verus production binding)
find verification/verus -name "*.rs" | xargs grep -l "extern_spec\|assume_specification"
# → 0 matches (no production binding across 92 files)

# vb-dyulo (file absence)
find crates/vb_core/src -name "compiled_slug*" -o -name "compiled_query*"
# → only compiled_workflow.rs present (refactor merged files)
```

File written: `/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-06-arch-drift.md`
