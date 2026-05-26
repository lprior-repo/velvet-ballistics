# Fuzz Future Phases — Deferred from RED_QUEEN_MASTER_PLAN

> **This document contains Phases 2-6, deferred until Phase 0 + Phase 1 are COMPLETE.**
> See `fuzz/EXECUTE.md` for what to do RIGHT NOW.
> See `fuzz/RED_QUEEN_MASTER_PLAN.md` for the full strategic context and harness inventory.

---

## PREREQUISITE

Phase 1 exit gate in `fuzz/EXECUTE.md` must be fully green:
- All targets compile with libfuzzer instrumentation
- All targets pass 10s ASAN smoke
- 21 weak functions hardened with behavioral assertions
- C.21-C.25 verified with 1-hour ASAN
- Seed corpora exist for all targets

**Do not read past this line until those gates pass.**

---

## PHASE 2: Fill Coverage Gaps — P0 Targets

### New Harnesses (13)

| # | Name | Crate | Attack Surface | Harness Type |
|---|------|-------|---------------|-------------|
| 1 | `fuzz_boundary_inventory_parse` | vb_boundary_inventory | `parse_inventory(&[u8])` | Parser |
| 2 | `fuzz_boundary_inventory_evidence_ref` | vb_boundary_inventory | `validate_evidence_reference_bytes(&[u8])` | Parser |
| 3 | `fuzz_boundary_inventory_validate` | vb_boundary_inventory | `validate_inventory()` | Structure-Aware |
| 4 | `fuzz_codegen_emit` | vb_codegen | `emit_rust_workflow()` | Structure-Aware |
| 5 | `fuzz_codegen_format` | vb_codegen | `format_generated_rust()` → `rustfmt` | Hostile |
| 6 | `fuzz_codegen_compare` | vb_codegen | `compare_generated_to_ir()` | Differential |
| 7 | `fuzz_proof_kernels_header_crc` | vb_proof_kernels | `validate_header_crc(&[u8])` | Parser |
| 8 | `fuzz_proof_kernels_header_bounds` | vb_proof_kernels | `validate_header_before_alloc(&[u8])` | Parser |
| 9 | `fuzz_storage_slot_extra` | vb_storage | `decode_slot_written_extra(&[u8])` | Parser |
| 10 | `fuzz_runtime_collect_page` | vb_runtime | `collect_page()` with page_size | Property |
| 11 | `fuzz_runtime_action_queue` | vb_runtime | `ActionQueue::enqueue/dequeue` | Property |
| 12 | `fuzz_expr_parse` | vb_expr | `parse()` on token streams | Parser |
| 13 | `fuzz_expr_differential` | vb_expr | Direct eval vs compile+evaluate | Differential |

### Steps

1. Create each harness in `fuzz/fuzz_targets/` using the EXECUTE.md harness template
2. Wire into `fuzz/Cargo.toml` with `[[bin]]` entries using `_fuzz` suffix
3. Build: `cargo fuzz build --target x86_64-unknown-linux-gnu`
4. Verify instrumentation: `nm | grep LLVMFuzzer`
5. Create seed corpora from existing test fixtures
6. Run 1-hour ASAN smoke per target: `cargo fuzz run TARGET -- -max_total_time=3600 -print_final_stats=1`
7. Triage crashes: minimize, deduplicate, fix, regression-test

---

## PHASE 3: Fill Coverage Gaps — P1 Targets

### New Harnesses (21)

| # | Name | Crate | Attack Surface | Harness Type |
|---|------|-------|---------------|-------------|
| 14 | `fuzz_validate_gate_07_stack` | vb_validate | Stack depth validation | Property |
| 15 | `fuzz_validate_gate_08_accessor` | vb_validate | Accessor path validation | Property |
| 16 | `fuzz_validate_gate_09_slots` | vb_validate | Slot reference validation | Property |
| 17 | `fuzz_validate_gate_10_node` | vb_validate | Node kind validation | Property |
| 18 | `fuzz_validate_gate_11_loop` | vb_validate | Loop graph validation | Property |
| 19 | `fuzz_validate_gate_12_action` | vb_validate | Action contract validation | Property |
| 20 | `fuzz_validate_gate_14_cycle` | vb_validate | Slot cycle detection | Property |
| 21 | `fuzz_validate_gate_15_type` | vb_validate | Type consistency validation | Property |
| 22 | `fuzz_ipc_payload_decode` | vb_ipc | `decode_payload()` per-variant | Parser |
| 23 | `fuzz_ipc_client_connect` | vb_ipc | Unix socket connection | Hostile |
| 24 | `fuzz_storage_admission_submit` | vb_storage | `submit_artifact()` | Structure-Aware |
| 25 | `fuzz_storage_journal_batch` | vb_storage | `JournalBatch` operations | Property |
| 26 | `fuzz_runtime_timer_wheel` | vb_runtime | Timer wheel tick/insert | Property |
| 27 | `fuzz_runtime_for_each` | vb_runtime | `evaluate_for_each()` | Property |
| 28 | `fuzz_runtime_together` | vb_runtime | `evaluate_together()` | Property |
| 29 | `fuzz_runtime_retry` | vb_runtime | `evaluate_retry()` | Property |
| 30 | `fuzz_compile_lowering` | vb_compile | Each `lower_*()` function | Structure-Aware |
| 31 | `fuzz_ui_model_envelope_roundtrip` | vb_ui_model | ALL envelope types roundtrip | Roundtrip |
| 32 | `fuzz_doc_scan_stale` | vb_doc | `scan_for_stale_clean_only_text()` | Parser |
| 33 | `fuzz_ui_snapshot_toml` | vb_ui_snapshot | `parse_tokens_from_toml()` | Parser |
| 34 | `fuzz_ui_snapshot_png` | vb_ui_snapshot | `validate_png_dimensions()` | Hostile |

### Additional

- Implement `Arbitrary` for `WorkflowParts`, `JournalEvent`, `IpcPayload`, `ExprOp`
- Create structure-aware harnesses using `Arbitrary` + libfuzzer custom mutators
- Seed corpora from Arbitrary: generate 100-1000 valid instances per type

---

## PHASE 4: Fill Coverage Gaps — P2 Targets

### New Harnesses (16)

| # | Name | Crate | Attack Surface | Harness Type |
|---|------|-------|---------------|-------------|
| 35 | `fuzz_benchmark_metadata` | vb_benchmark | Duration edge cases | Property |
| 36 | `fuzz_benchmark_threshold` | vb_benchmark | Budget comparison | Property |
| 37 | `fuzz_yaml_source_map` | vb_yaml | Source map construction | Property |
| 38 | `fuzz_yaml_ast_validate` | vb_yaml | AST structure validation | Structure-Aware |
| 39 | `fuzz_core_value_store` | vb_core | put/get blob/list operations | Property |
| 40 | `fuzz_core_run_loop` | vb_core | `run_loop()` with fuzzed RunFrame | Property |
| 41 | `fuzz_storage_snapshot` | vb_storage | Snapshot save/restore | Roundtrip |
| 42 | `fuzz_storage_trimming` | vb_storage | Trim logic | Property |
| 43 | `fuzz_cli_arg_parse` | vb_cli | Command-line arg parsing | Hostile |
| 44 | `fuzz_cli_command_dispatch` | vb_cli | Command dispatch | Structure-Aware |
| 45 | `fuzz_ui_ipc_bridge` | vb_ui | IPC bridge state | Property |
| 46 | `fuzz_ui_makepad_canvas` | vb_ui_makepad | Canvas rendering input | Hostile |
| 47 | `fuzz_ipc_frame_roundtrip_full` | vb_ipc | Full frame encode/decode cycle | Roundtrip |
| 48 | `fuzz_ipc_server_dispatch` | vb_ipc | Server command dispatch | Structure-Aware |
| 49 | `fuzz_compile_expression` | vb_compile | Expression compilation | Structure-Aware |
| 50 | `fuzz_runtime_shard_lifecycle` | vb_runtime | Shard start/tick/dispatch cycle | Property |

### Additional

- Differential fuzzing pairs: Direct eval vs compile+evaluate, YAML events vs AST
- Concurrency fuzz targets: ActionQueue SPSC, timer wheel, shard tick

**Note:** Targets #45 and #46 (vb_ui, vb_ui_makepad) currently lack specific function signatures. A dedicated attack-surface analysis of these crates must be completed before harness implementation. Do not create hand-wavy "IPC bridge state" targets — identify exact `pub fn` signatures and their input types first.

---

## PHASE 5: CI Integration

### GitHub Actions

**Tier 1 — PR Smoke (top-20 targets, 60s each, sequential!)**

Do NOT use a parallel matrix — GHA free tier has a 20-concurrent-job limit. Use a single job:

```yaml
fuzz-smoke:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@nightly
    - run: cargo install cargo-fuzz
    - run: |
        for target in $(cargo fuzz list | head -20); do
          cargo fuzz run "$target" -- -max_total_time=60 -rss_limit_mb=2048 -print_final_stats=1
        done
```

**Tier 2 — Nightly (all targets, 1hr each, sequential)**

Same sequential pattern. 50 targets × 1hr = 50 hours. Run on self-hosted runner or split across multiple nightly windows. Can also parallelize with a matrix of 5-10 targets per job (not 50).

**Tier 3 — Weekly Deep (top-25 targets, 12hr, self-hosted)**

Requires dedicated hardware. libfuzzer-only. No AFL++ until Phase 0+1 are proven stable.

### Moon CI

Update `.moon/tasks/all.yml` `fuzz-smoke` task to run top-20 targets for 60 seconds (not 4 targets for 1 second).

---

## PHASE 6: Advanced (Perpetual)

### AFL++ Secondary Engine
- Install `cargo-afl` and system `afl++` package
- Build AFL-instrumented targets from stdin-based binaries
- Create AFL dictionaries (fuzz/dicts/vb_storage.dict, vb_ipc.dict, vb_expr.dict)
- Run AFL++ on top-10 targets alongside libfuzzer nightly

### Mutation Testing
- `cargo mutants -p velvet-ballastics-fuzz`
- Target: ≥90% mutation kill rate on fuzz harnesses
- Fix any harness that doesn't catch a deletion of its own assertion

### Corpus Management
- Auto-commit interesting inputs on nightly
- Minimize weekly: `cargo fuzz cmin`
- Cross-pollinate corpora between related targets

### Regression Library
- Every crash → minimized reproducer → regression test in `workspace_tests`
- CI gate: all regression tests must pass

### Coverage Dashboard
- `cargo fuzz coverage` + `llvm-cov` report
- Track per-crate edge coverage over time
- Alert on coverage regression >2%

### ClusterFuzzLite (Pre-OSS-Fuzz)
- Integrate ClusterFuzzLite GitHub Action for continuous fuzzing
- Requires: `.clusterfuzzlite/project.yaml`, Dockerfile, build.sh
- This is the lightweight OSS-Fuzz path — same engine, no application required
- OSS-Fuzz submission evaluated when: project has public stars, dedicated maintainer, and >6 months stable CI

---

## Bead Mapping

| Phase | Harnesses | Beads |
|-------|-----------|-------|
| Phase 2 | 13 P0 | vb-fuzz-p0-* |
| Phase 3 | 21 P1 | vb-fuzz-p1-* |
| Phase 4 | 16 P2 | vb-fuzz-p2-* |
| Phase 5 | CI tasks | vb-fuzz-ci-* |
| Phase 6 | Perpetual | vb-fuzz-gauntlet-* |

---
**STATUS: DEFERRED. Execute fuzz/EXECUTE.md first.**
