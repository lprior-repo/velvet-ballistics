# vb-kyyf State 2 codebase map: cross-run determinism/reproducibility BDD scope

## Isolation and bead evidence
- Isolated workspace verified by `pwd -P`: `/home/lewis/src/femdation-vb-kyyf-bdd`; this is not the forbidden source checkout `/home/lewis/src/velvet-ballistics`.
- `bd show vb-kyyf --json` fails in this isolated workspace because `.beads` points at a Dolt server missing table `issues`; State 1 captured the source-checkout bead JSON in `.beads/vb-kyyf/STATE.md` lines 23-109.
- Bead title: `bdd: Cross-run determinism and reproducibility acceptance scenarios`.
- State 2 only maps scope; no code or tests implemented.

## Master contract clauses that define acceptance scope
- `velvet-ballistics-MASTER.md` lines 23, 49, 240, 999: deterministic synchronous numeric state-machine execution until suspension.
- `velvet-ballistics-MASTER.md` lines 750, 790-797: Fjall persistence is required; recovery replays snapshots plus tail journal or full journal deterministically; replay must not re-execute non-replay-safe external side effects; corrupt records and digest mismatches return typed errors.
- `velvet-ballistics-MASTER.md` lines 1071-1107: generated Rust must preserve exact IR semantics; current gap says `compare_generated_to_ir` is source-pattern/counting only, not true terminal-result/taint/journal/error equivalence.
- `velvet-ballistics-MASTER.md` lines 1250-1314: public surfaces for storage recovery, runtime deterministic drive/action resume, and codegen parity.
- `velvet-ballistics-MASTER.md` lines 1318-1344: public CLI includes `run`, `run-compiled`, `events`, `inspect`, `replay`, and `incident`; structured output is cold-path operator contract.
- `velvet-ballistics-MASTER.md` lines 1483-1485 and 1492-1496: mandatory tests include CLI replay, generated equivalence, full recovery/replay, evidence-chain completion, and recovery evidence chain.

## Existing relevant implementation map
- Storage journal replay: `crates/vb_storage/src/journal/replay.rs` provides `FjallJournal::events_for_run` and validates contiguous per-run sequence order from durable snapshots/events.
- Storage recovery orchestration: `crates/vb_storage/src/recovery/recover.rs` exposes digest checks, `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_all_incomplete_runs`, and currently notes deferred action ABI/policy digest lookup in `verify_digests`.
- Runtime recovery boundary: `crates/vb_runtime/src/recovery.rs` converts storage hydration into runtime-facing summaries or frame hydration, rejecting unsupported live-frame state.
- Codegen parity boundary: `crates/vb_codegen/src/lib.rs` and `crates/vb_codegen/src/codegen/mod.rs` expose `emit_rust_workflow`, `validate_generated_subset`, and `compare_generated_to_ir`; the compare function currently checks forbidden patterns and counts steps/expressions/actions rather than executing IR vs generated outputs.
- Acceptance catalog: `crates/workspace_tests/src/acceptance_catalog.rs` has scenario rows for storage recovery and generated parity, but both are deferred follow-up rows (`vb-rpch`, `vb-0sps`) rather than executable BDD evidence.

## Existing relevant tests
- `crates/vb_storage/tests/replay_resume.rs`: asserts reopened journal replay is exact and deterministic when read twice, including full recovery replay equality and sequence-gap rejection.
- `crates/vb_storage/tests/recovery_bdd_tests.rs`: B-001..B-020 style BDD storage recovery tests for digest binding/mismatch, full-journal exactness, summary/frame hydration, corrupt/unsupported states, and typed errors.
- `crates/vb_storage/tests/recovery_integration.rs`: Fjall-backed crash-recovery integration coverage for strict writes, replay ordering, partial write detection, writer queue behavior, and action replay tracking.
- `crates/vb_storage/tests/vb_h6ix_integration.rs`: latest-attempt replay filtering and stale terminal behavior for mixed-attempt journals.
- `crates/vb_codegen/src/tests.rs`: contains generated-vs-interpreter checks for selected generated subset behavior and `emit_trybuild_fixture`, but not full cross-run reproducibility BDD acceptance through public release surfaces.
- `crates/vb_codegen/tests/trybuild_tests.rs` plus compile-fail fixtures: generated Rust compile-safety gates exist, but they are not semantic replay/determinism proof.
- `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs`: validates catalog metadata and confirms storage/generated parity are currently deferred gaps.
- Fuzz/formal adjacency: `fuzz/src/bin/replay_events.rs`, `fuzz/src/bin/recovery_decode.rs`, `fuzz/src/bin/recover_runtime_frame_seed_contract.rs`, `verification/verus/recovery_*.rs`, and `reference/src/replay_model.rs` are useful verifier context but not direct BDD acceptance tests.

## BDD acceptance scenarios to implement later
1. Given the same accepted artifact/input/durability profile in two isolated temp stores, when run/replay/inspect/events are driven through public surfaces, then terminal result, event sequence/kinds, slot/taint observations, digest checks, and diagnostics are byte-for-byte or structurally identical except allowed run-id/path/timestamp fields.
2. Given a persisted strict/journaled run, when the process boundary is simulated by dropping and reopening Fjall, then `events_for_run`, `recover_full_journal`, `recover_runtime_summary`, and CLI `replay` produce the same observable report on repeated reads.
3. Given a non-idempotent or at-least-once external action journal, when recovery replays after scheduling/completion boundaries, then no external side effect is re-executed and typed replay policy errors or blocked states are stable across repeated recovery.
4. Given corrupted, gapped, duplicate, or digest-mismatched records, when replay/recovery runs twice, then both runs fail with the same typed storage/replay error and do not silently continue.
5. Given a workflow supported by generated mode and IR mode, when both modes are executed on the same fixture and replayed from durable evidence, then terminal result, taint, journal events, suspensions, and typed errors match exactly.

## Gaps and risks
- Existing storage tests cover deterministic reads at API level, but there is no single BDD scenario group named for `vb-kyyf` that ties cross-run determinism/reproducibility to the release acceptance suite.
- Acceptance catalog currently records storage recovery and generated parity as deferred rows, so this bead likely needs a new executable row or a dedicated `vb_kyyf_*` workspace/storage test target.
- CLI replay is listed as mandatory public surface, but the existing mapped tests mostly use Rust APIs; public CLI acceptance may require a workspace test or xtask harness if the binary surface is stable enough.
- Generated parity risk is high: master and implementation both acknowledge `compare_generated_to_ir` is not true semantic execution equivalence.
- Full digest verification risk: `verify_digests` still defers action ABI and policy digest verification.
- Runtime recovery risk: full live-frame hydration rejects unsupported slot taint/action payload/pending action states, so BDD scope must distinguish supported deterministic replay from known release gates.
- Dependency changes are not indicated by this exploration; scope should remain tests/artifacts unless implementation later proves a missing dev dependency.

## Likely touched files/globs for implementation state
- `crates/workspace_tests/src/acceptance_catalog.rs`
- `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` or equivalent new workspace BDD test target
- `crates/vb_storage/tests/replay_resume.rs`
- `crates/vb_storage/tests/recovery_bdd_tests.rs`
- `crates/vb_storage/tests/recovery_integration.rs`
- `crates/vb_codegen/src/tests.rs`
- Optional if CLI acceptance is required: `crates/workspace_tests/tests/*cli*`, `crates/velvet_ballastics/src/**`, and existing CLI replay command tests if present.
