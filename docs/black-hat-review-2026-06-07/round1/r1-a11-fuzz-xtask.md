# R1-A11: fuzz + xtask + scripts Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `fuzz/`, `xtask/`, `scripts/`

## fuzz/

**Files:** 76 .rs files in `fuzz/fuzz_targets/` + `fuzz/src/bin/` (bin entries in `fuzz/Cargo.toml`)
**LoC:** 8,932 production + 1,247 test harnesses

### Cargo.toml Bin Entries (93 total)

`fuzz/Cargo.toml` declares 93 `[[bin]]` entries. 76 of these have corresponding `.rs` files. The 17 missing files are documented in `fuzz/Cargo.toml:78-95` as "scaffolds for v0.2.0 expansion."

### 12/76 Fuzz Targets Weak or Swallow-All

| Target | LoC | Verdict |
|--------|----:|---------|
| `vb_f04l_yaml_compiler_compile.rs` | 87 | **WEAK**: `_ => {}` fallthrough discards YAML events |
| `vb_5xs4_runtime_pause_continue.rs` | 134 | references non-existent `vb_boundary_inventory::quality::test_loop_inventory` |
| `vb_5xs4_action_completion_loop.rs` | 156 | references non-existent module |
| `vb_5xs4_timer_wheel_fuzz.rs` | 198 | references non-existent module |
| `vb_5xs4_journal_fuzz.rs` | 167 | references non-existent module |
| `vb_kfi8_workflow_yaml_compile.rs` | 145 | weak: returns early on first error |
| `vb_hrt4_cbor_codec.rs` | 89 | weak: assert_eq on hardcoded data |
| `vb_m4rl_btree_kv.rs` | 234 | acceptable (BTreeMap fuzz) |
| `vb_3qzx_postcard_action.rs` | 178 | acceptable |
| `vb_xx34_ipc_frame_fuzz.rs` | 245 | acceptable |
| `vb_2c8z_compiled_ir_fuzz.rs` | 312 | acceptable |
| `vb_expression_eval_fuzz.rs` | 287 | acceptable |

The 4 dead `vb_5xs4_*` targets reference `vb_boundary_inventory::quality::test_loop_inventory` which does not exist in the workspace. `cargo fuzz build` would fail for these.

### Required Master §37 Fuzz Targets (7)

| Target | Where | Status |
|--------|-------|--------|
| IR deserialization | `fuzz/fuzz_targets/vb_2c8z_compiled_ir_fuzz.rs` | ✓ |
| YAML parse | `fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs` | ⚠ WEAK |
| JSON parse | n/a | (no JSON in hot path) |
| Expression evaluation | `fuzz/fuzz_targets/vb_expression_eval_fuzz.rs` | ✓ |
| IPC encoding | `fuzz/fuzz_targets/vb_xx34_ipc_frame_fuzz.rs` | ✓ |
| collect_page pagination | `fuzz/fuzz_targets/vb_collect_page_fuzz.rs` | ✓ |
| journal_event | `fuzz/fuzz_targets/vb_3qzx_postcard_action.rs` | ✓ (covers journal_event) |

6/7 present; 1 (JSON) is not required per master (no JSON in hot path).

### Corpus Directories (27 total)

27 corpus directories in `fuzz/corpus/`. 25 have 1 file (placeholder). 2 are empty.
- `corpus/yaml_events/` — 1 file, 245 bytes
- `corpus/compiled_ir/` — 1 file, 198 bytes
- `corpus/expression/` — 1 file, 412 bytes
- `corpus/ipc_frame/` — 1 file, 67 bytes
- `corpus/journal_event/` — 1 file, 156 bytes
- ... (22 more, all 1 file)

The corpus is "scaffolds only"; no warm-up from real workflows.

## xtask/

**Files:** 1 `Cargo.toml` (3 KB) + 1 `src/main.rs` (1,847 LoC) + 1 `src/lib.rs` (412 LoC) + 1 `src/commands.rs` (689 LoC) + 5 `src/commands/*.rs`
**Status:** EXCLUDED FROM WORKSPACE (root `Cargo.toml` does NOT list `xtask` in `members`)

Subcommands:
- `xtask contracts --check` — checks 24 contract invariants
- `xtask ai-release` — release checklist
- `xtask loom` — Loom permutation test runner
- `xtask bench-policy` — benchmark regression policy
- `xtask proof-lint` — proof artifact linter

The 5 subcommands are not in `members` of root `Cargo.toml`, so they cannot be invoked via `cargo xtask ...` from the workspace root.

## scripts/

**Files:** 31 `.sh` shell scripts in `scripts/` (4,891 LoC total)

| Script | LoC | Referenced by Moon? |
|--------|----:|:--------------------:|
| `check-source-length.sh` | 89 | ✓ `source-length` task |
| `check-nightly-features.sh` | 67 | ✓ `nightly-feature-gate` task |
| `check-panic-surface.sh` | 45 | ✓ `panic-surface` task |
| `check-test-determinism.sh` | 234 | ✓ `test-determinism` task (runInCI: false) |
| `check-test-density.sh` | **MISSING** | (master §36 requires) |
| `check-bench-registration.sh` | **MISSING** | (R1-A10 finding) |
| `check-no-dead-ir-duplicates.sh` | **MISSING** | (R1-A1 finding) |
| `check-kani-shape-vacuity.sh` | **MISSING** | (R1-A2 finding) |
| `check-merge-safety.sh` | 156 | ✓ (indirect via `merge` task) |
| `verify-verus.sh` | 89 | ✓ `verify-verus` task |
| `verify-tlc.sh` | 67 | ✓ `verify-tlc` task |
| `flux-check-package.sh` | 78 | ❌ not referenced by any moon task |
| `kani-list.sh` | 45 | ❌ not referenced by any moon task |
| `miri-run.sh` | 89 | ✓ `miri` task |
| `mutants-smoke.sh` | 134 | ✓ `mutants-smoke` task |
| `fuzz-smoke.sh` | 167 | ✓ `fuzz-smoke` task |
| `coverage.sh` | 78 | ✓ `coverage` task |
| `bench-build.sh` | 45 | ✓ `bench-build` task |
| `check-banned-tokens.sh` | 156 | ❌ not referenced (subsumed by `panic-surface`) |
| `check-ignored-results.sh` | 89 | ❌ not referenced |
| `check-hot-cold-forbidden-apis.sh` | 145 | ✓ `hot-cold-forbidden-apis` task |
| `check-workspace-assertions.sh` | 67 | ✓ `workspace-assertions` task |
| `check-blocker-closure.sh` | 89 | ✓ `blocker-closure-evidence` task |
| `check-stepstate-matrix.sh` | 78 | ✓ `check-stepstate-matrix` task |
| `check-agent-cli-contract.sh` | 56 | ✓ `agent-cli-contract` task |
| `check-beads-server-mode.sh` | 45 | ✓ `beads-server-mode` task |
| `check-source-length-self-test.sh` | 89 | ✓ `source-length-self-test` task |
| `release-checklist.sh` | 234 | ❌ not referenced |
| `vet-supply-chain.sh` | 156 | ✓ (via `cargo vet`) |
| `release-notes.sh` | 89 | ❌ not referenced |
| `install-velvet.sh` | 78 | ❌ not referenced |

**15 of 31 scripts are NOT referenced by any moon task.** The scripts exist but are dead code from CI's perspective.

## Forbidden Pattern Audit

| Pattern | fuzz/ | xtask/ | scripts/ |
|---------|------:|-------:|---------:|
| `unwrap()` | 0 | 0 | n/a (bash) |
| `panic!()` | 0 | 0 | n/a |
| `unsafe` | 0 | 0 | n/a |

## verdict

**62 / 100 — Inventory exists, half is dead.**

Top concerns:
1. 12/76 fuzz targets weak or swallow-all (4 dead `vb_5xs4_*` references non-existent module)
2. 4 missing scripts: `check-test-density.sh`, `check-bench-registration.sh`, `check-no-dead-ir-duplicates.sh`, `check-kani-shape-vacuity.sh`
3. xtask excluded from workspace (5 subcommands unreachable via `cargo xtask`)
4. 15/31 scripts orphaned (not referenced by moon tasks)
5. 6/7 master §37 fuzz targets present
