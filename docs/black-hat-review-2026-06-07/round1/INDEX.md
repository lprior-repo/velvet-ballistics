# Round 1: Codebase Inventory — Transcript Index (12 agents)

**Run date:** 2026-06-07 · **Workspace:** /home/lewis/src/velvet-ballistics
**Subagent type:** explore (12 agents in parallel)

The full R1 outputs are too large (60+ KB per report) for this index. Key findings per agent:

## R1-A1: vb_core (175 files, 88K LoC, 7,331 LoC largest)
- Canonical `CompiledNode`/`ExprOp`/`AccessorProgram`/`ResourceContract` in `workflow/types.rs:520-710, 442-504, 277-294, 169-206`
- 2,578-line `integration_taint_propagation.rs` has 0 proptest macros
- 7 Kani harnesses in `kani/kani_taint_propagation.rs` (mathematically over-evidenced)
- 5 active flux files + 17 verification submodules
- `compiled_slug.rs` (583 LoC) is the canonical production seam cited by 6 Flux + 3 fuzz + 2 Kani + 8 proptests
- **Verdict: 99/100** (one missing proptest for `tautology in taint lattice`)

## R1-A2: vb_yaml (34 files, 7K LoC)
- Strict profile correctly rejects: anchors, aliases, merge keys, custom tags, multi-doc, YAML 1.1 booleans
- 4/5 Kani files are ORPHANED (not in module tree): `kani_is_primitive_legacy.rs`, `kani_all_variants_registered.rs`, `kani_checked_add.rs`, `kani_panic_freedom.rs`
- Only `kani_yaml_error_code.rs` is compiled — it's a DEPRECATED vacuum model (GOD RULE 1+2 violation)
- `is_primitive` matches legacy names "parallel"/"aggregate" but `parse_step_primitive` rejects them (brittle defense-in-depth)
- **Verdict: 70/100** — production correct, formal-verification story broken

## R1-A3: vb_validate (79 files, 23K LoC, 5,500+ LoC)
- All 36 Section 16 codes ✓
- 5,500+ LoC across 17 gates with multiple parallel implementations (DRIFT-5)
- `validate_taint` correctly accepts Secret/DerivedFromSecret Finish (Section 47 ✓)
- 5,500+ test LoC, 911+ #[test] functions
- **Verdict: 71/100** — production complete, modular duplication is drift

## R1-A4: vb_expr (56 files, 21K LoC, 1,016 LoC main eval)
- 3 evaluator copies: `eval.rs` (1016), `eval/evaluate.rs` (774), `eval/core.rs` (158)
- `builtin_eval.rs:107-130` has documented `i64::MIN/-1` bug (BH-BE-001)
- `LoadAccessor` opcode missing from dispatch (falls through to `UnknownOperator`)
- ExprOp count: 29 (master says 30 — section 46 mentions unary `-`)
- AND/OR no-short-circuit ✓ (3-layer enforcement)
- 10/10 helpers ✓; F64 finiteness ✓; stack bound ✓
- **Verdict: 78/100** — solid, 5x3 evaluator copies is the issue

## R1-A5: vb_compile (113 files, 37K LoC)
- 22/34 IR variants emitted (12 reserved)
- All 11 primitives lowered ✓ (with `parse_step_primitive` redirect)
- §65 SideEffect/RetrySafety: 5+3 in code, 7+4 in master
- `mod restrictions;` NOT declared; 19 dead tests in `restrictions/tests/attempt_number_tests.rs`
- 48 files over 300 lines
- 2 dead duplicate IR files (nodes.rs, expressions.rs, accessors.rs, validation.rs, compiled_workflow.rs, compiled_workflow.rs.removed)
- **Verdict: 52/100** — compiles, all primitives lower, but the §65 taxonomy drift is the most dangerous defect

## R1-A6: vb_storage (239 files, 65K LoC)
- 9/9 keyspaces, 7/7 magics, 20/20 record kinds, 11/11 typed errors, 60-byte envelope ✓
- Postcard + BLAKE3 + CRC32C envelope structure ✓
- 3 durability profiles: volatile, journaled, strict ✓
- 8,091-line monolithic `tests.rs`
- SlotWritten-before-PC-advance NOT tested in storage (deferred to runtime)
- **Verdict: 78/100** — spec-conformant

## R1-A7: vb_runtime (321 files, 98K LoC, 3,500 functions)
- 7/7 ShardCommand + 4/4 ShardDirective + 6/6 EngineSignal ✓
- `tick_shard` present at `runtime_control.rs:24`; Cancel/Barrier stubbed
- **`BoundedActionCompletionQueue` uses `Mutex<VecDeque>`** (Section 50 violation)
- `crates/vb_runtime/src/runtime/` directory does NOT exist — runtime module is `runtime.rs` single file + 4 `#[path]` includes
- IPC ingress uses `crossbeam_channel::bounded` (LETHAL Section 50)
- **Verdict: 72/100** — 7/7 P0 patches restored, 2 LETHAL backends, runtime/ dir path drift

## R1-A8: vb_ipc (60 files, 17K LoC)
- 24-byte IPC frame ✓ (byte-exact to master §21)
- 11/11 commands ✓ (IDs 1..=11, contiguous)
- In-order pipelining ✓
- 4/4 Kani harnesses ✓
- **`MemoryIngress` uses `crossbeam_channel::bounded`** (LETHAL Section 50)
- `banned-token-gates` task is a no-op (no command/script)
- **Verdict: 72/100** — wire format perfect, backend wrong

## R1-A9: vb_cli (126 files, 30K LoC)
- 30/30 master §33 subcommands ✓
- 8/30 typed Postcard envelopes; 22 use generic fallback (still typed, no JSON)
- Binary name `velvet-ballistics` (master §33.6 compliant)
- 33 src/ files over 300 lines
- 1176 `#[test]` attributes + 431 integration tests = 1607 total
- **Verdict: 88/100** — comprehensive CLI

## R1-A10: vb_benchmark + workspace_tests
- BenchmarkMetadata has 7/22 fields (only git commit on struct; rest is const-string or sidecar JSONL)
- 12 of 22 bench groups registered
- 5 of 11 Section 38 properties missing
- 2 of 22 bench groups missing entirely (`warm_throughput`, `digest_computation`)
- 11 `*_root_migrated.rs` dead duplicates (0 of 12 byte-identical to orig)
- `cargo check --benches` is GREEN even with fatal syntax error in `action_dispatch_root_migrated.rs:10`
- **Verdict: 62/100** — bench count inflated, real measurement is 3/22

## R1-A11: fuzz + xtask + scripts
- 12/76 fuzz targets weak/swallow-all (e.g., `vb_f04l_yaml_compiler_compile.rs` has `_ => {}` fallthrough)
- 4 dead `vb_5xs4_*` targets (reference non-existent `vb_boundary_inventory::quality::test_loop_inventory`)
- 27 corpus dirs with 1 file (placeholder)
- 2 corpus dirs empty
- xtask excluded from workspace
- 16/31 scripts orphaned (not referenced by moon tasks)
- **Verdict: fuzz/xtask 62/100** — inventory exists, half is dead

## R1-A12: .moon + .cargo + supply-chain
- 50 tasks defined (43 in `all.yml`, 2 in `kani.yml`, 2 in `verus.yml`, 3 in `tlc.yml`)
- 21 in pipeline + 15 `runInCI: false` (test-determinism, benchmark-regression-policy, maxperf, verify-fast/standard/deep/proof/all, contracts, etc.)
- 5 of 21 pipeline tasks are smoke-only
- 2 phantom tasks: `nightly-feature-cargo-probe` (script body is `true`), `banned-token-gates` (no command)
- 1,088 test-determinism findings hidden (256 UncontrolledClock, 784 SharedTempState, 31 UncontrolledRandom, 15 GlobalMutableState, 2 SleepAsSync)
- 12 dep exemptions in `cargo-vet.toml`
- Toolchain pinned: `nightly-2026-04-28` ✓
- **Verdict: 72/100** — broad coverage, smoke lanes are false confidence
