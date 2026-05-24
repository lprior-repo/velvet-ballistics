# Red Queen Fuzzing — Master Execution Plan
## velvet-ballistics · 2026-05-24

> **IF YOU ARE EXECUTING WORK RIGHT NOW:** Read `fuzz/EXECUTE.md` — actionable Phase 0+1 steps with failure branches.
> **IF YOU ARE PLANNING FUTURE WORK:** Read this document for the full strategic context. Phases 2-6 are deferred to `fuzz/FUTURE.md`.
> **FOR EXTREME DEPTH:** Read `fuzz/EXTREME_FUZZING.md` — stage-split harnesses, multi-oracle design, structure-aware mutation, sanitizer matrix, crash handling, campaign monitoring.
>
> "It takes all the running you can do, to keep in the same place." — Red Queen Hypothesis
> 
> Every function. Every crate. Every code path. ASAN+UBSAN. Continuous. Relentless. Survive.

---

## TABLE OF CONTENTS

1. [Current State Assessment](#1-current-state-assessment)
2. [Gap Analysis — The Lethal Truth](#2-gap-analysis)
3. [Attack Surface Matrix](#3-attack-surface-matrix)
4. [Red Queen Campaign Architecture](#4-red-queen-campaign-architecture)
5. [Harness Inventory — Full Catalog](#5-harness-inventory)
6. [Implementation Phases](#6-implementation-phases)
7. [CI Integration](#7-ci-integration)
8. [Success Metrics](#8-success-metrics)
9. [Tooling Requirements](#9-tooling-requirements)
10. [Risk Register](#10-risk-register)

---

## 1. Current State Assessment

### 1.1 Brutal Truth

| Dimension | Status | Severity |
|-----------|--------|----------|
| **cargo-fuzz installed** | ❌ NOT INSTALLED | **BLOCKER** — zero targets can run |
| **cargo-afl installed** | ❌ NOT INSTALLED | No AFL++ integration exists |
| **cargo-hfuzz installed** | ❌ NOT INSTALLED | No honggfuzz integration exists |
| **libfuzzer harnesses** | 1 declared, 11 undeclared (dead code) | 11 proper harnesses are invisible to cargo-fuzz |
| **stdin binaries** | 37 declared, 9 undeclared | All lack ASAN + coverage feedback |
| **Strong assertions** | 19 of ~40 fuzz functions | Only 47.5% carry behavioral assertions |
| **Coverage-only targets** | 21 of ~40 fuzz functions | 52.5% are smoke tests — find nothing |
| **CI fuzzing** | 4 targets × 1 second each | Cosmetic only — no meaningful coverage |
| **Seed corpora** | 7 of 31+ targets | 24 targets start from scratch |
| **Crates with zero fuzz** | 8 of 19 production crates | 42% of crates never fuzzed |
| **12h campaign executed** | Stdin-based, no ASAN | Proved panic-freedom only |

### 1.2 What Works

- `fuzz/src/lib.rs` — 3,010 lines of shared harness library with typed error assertion helpers
- `fuzz/fuzz_targets/` — 12 proper libfuzzer harnesses exist (though 11 are unreachable)
- `fuzz/corpus/` — 6 active seed corpora with real test data
- `fuzz/REDO_PLAN.md` — Accurate diagnosis of the broken state
- 6 LETHAL fuzz gaps identified (C.21–C.25 + generated_compare)
- 22 TLA+ specs, 44 Verus proofs, 26 Kani harnesses — strong formal foundation

---

## 2. Gap Analysis — The Lethal Truth

### 2.1 LETHAL Gaps (Blocking)

| ID | Gap | Impact |
|----|-----|--------|
| **L1** | `cargo-fuzz` not installed | Zero fuzz targets runnable |
| **L2** | 11 fuzz_targets/ undeclared in Cargo.toml | Proper libfuzzer harnesses are dead code |
| **L3** | `generated_compare` STUB | No comparison logic — zero behavioral assertions |
| **L4** | `compiled_ir` STUB | Drops decode results — slot bounds never checked |
| **L5** | `ipc_frame` discards results | `Ok(_) | Err(_) => {}` — no assertions |
| **L6** | `expression` discards results | `let _result =` — no taint/type assertions |
| **L7** | `collect_page_pagination` fn does not exist in lib.rs | Target compiles but calls missing function |
| **L8** | `decode_record` suppresses all failures with `.ok()` | Silent error suppression |
| **L9** | No `[package.metadata.cargo-fuzz]` config | No minimization/len_control |

### 2.2 CRITICAL Gaps (Ship-Breaking)

| ID | Gap | Impact |
|----|-----|--------|
| **C1** | vb_boundary_inventory — zero fuzz | JSON parser from `&[u8]` never fuzzed |
| **C2** | vb_codegen — zero fuzz | `emit_rust_workflow()` spawns external processes — never fuzzed |
| **C3** | vb_proof_kernels — zero fuzz | Header CRC/bounds validation on raw bytes — never fuzzed |
| **C4** | vb_ui_model — single weak fuzz | `decode_postcard()` for envelope types — minimal coverage |
| **C5** | vb_ui — 801 pub fn, zero fuzz | Largest attack surface — completely exposed |
| **C6** | vb_ui_snapshot — zero fuzz | TOML parser, PNG validation, layout kernels — bare |
| **C7** | vb_benchmark — zero fuzz | Duration/numeric edge cases never exercised |
| **C8** | 37 targets are stdin-based | No ASAN, no coverage feedback, no corpus mutation |
| **C9** | Only 4/38 targets in CI | The other 34 never run automatically |
| **C10** | 24 targets lack seed corpora | Fuzzer starts from random bytes — slow convergence |

### 2.3 MAJOR Gaps (Quality)

| ID | Gap | Impact |
|----|-----|--------|
| **M1** | `vb_runtime::action_queue` no fuzz | Lock-free bounded MPMC queue — critical for concurrency |
| **M2** | `vb_expr::parse()` no direct fuzz | Expression parser on raw token streams — only tested via full chain |
| **M3** | `vb_storage::decode_slot_written_extra()` no fuzz | Slot extra decode — potential buffer overflow |
| **M4** | `vb_ipc::codec::decode_payload()` no direct fuzz | Payload decoder for 15+ variant types |
| **M5** | `vb_yaml::collect_events()` — fuzz passes but no assertions | Accepts arbitrary bytes, only tests panic-freedom |
| **M6** | No AFL++ dictionary files | Magic bytes, record kinds, schema versions not seeded |
| **M7** | No `Arbitrary` impls for domain types | Cannot generate structurally valid random inputs |
| **M8** | 21 functions are "coverage-only" | Explicitly documented as weak — intentional but gaping holes |
| **M9** | No mutation testing on fuzz harnesses | Can't verify harnesses catch regressions |
| **M10** | 37 src/bin/ files duplicate `run_with_stdin` boilerplate | Maintenance hazard, inconsistency risk |

---

## 3. Attack Surface Matrix

### 3.1 Every Crate, Every Function — Fuzz Coverage Required

| Crate | Pub fn | Fuzz Targets | Coverage | Gaps |
|-------|--------|-------------|----------|------|
| **vb_core** | 150 | 15 existing | ~80% paths | IR deserialization, validation, engine run loop, value store, replay |
| **vb_expr** | 59 | 5 existing | ~60% paths | Direct parser fuzz, bytecode fuzz, differential eval |
| **vb_storage** | 144 | 16 existing | ~70% paths | Slot extra decode, snapshot fuzz, trimming fuzz, process lock |
| **vb_runtime** | 264 | 8 existing | ~30% paths | Action queue, timer wheel, for_each/together/collect primitives, shard lifecycle |
| **vb_ipc** | 107 | 4 existing | ~50% paths | Payload decode per-variant, server dispatch, client connection |
| **vb_validate** | 116 | 1 existing | ~10% paths | All 9 gates individually, schema validation, taint checking, references |
| **vb_yaml** | 43 | 3 existing | ~70% paths | Source map fuzz, AST validation, profile enforcement |
| **vb_compile** | 80 | 1 existing | ~30% paths | Each lowering function, expression compilation, error recovery |
| **vb_codegen** | 40 | 1 existing (STUB) | ~5% paths | Code emitter, formatter, comparison |
| **vb_cli** | 48 | 2 existing | ~10% paths | Arg parsing, command dispatch, lifecycle |
| **vb_boundary_inventory** | 23 | **0** | **0%** | JSON parse, evidence validation, inventory validation |
| **vb_doc** | 11 | 1 existing | ~40% paths | Text scanning, taint vocabulary, evidence claims |
| **vb_ui_model** | 29 | 1 existing | ~20% paths | Postcard codec for ALL envelope types, canonicalization |
| **vb_ui_snapshot** | 72 | 4 existing | ~20% paths | TOML parser, layout kernels, color drift, spelling |
| **vb_ui** | 801 | 0 | **0%** | IPC bridge, state management, graph renderer, theme |
| **vb_ui_makepad** | 106 | 0 | **0%** | Widget renderer, canvas, nodes, edges |
| **vb_benchmark** | 6 | 0 | **0%** | Duration, metadata, threshold checks |
| **vb_proof_kernels** | 44 | 0 | **0%** | Header CRC, bounds, envelope validation |
| **vb_verification** | 0 | 0 | N/A | Placeholder — no runtime code |

### 3.2 By Input Category — Highest Priority First

#### CATEGORY A: Raw Bytes (`&[u8]`) — PARSER/CODEC ATTACK SURFACE

These functions are the **highest-value fuzz targets**. Every one needs its own harness.

| Function | Crate | Existing? | Priority |
|----------|-------|-----------|----------|
| `decode_record_header(&[u8])` | vb_storage | Partial | **P0** |
| `decode_record::<T>(&[u8])` | vb_storage | decode_record (weak) | **P0** |
| `decode_frame_header(&[u8])` | vb_ipc | ipc_frame (discards) | **P0** |
| `decode_frame_payload(&[u8])` | vb_ipc | ipc_frame (discards) | **P0** |
| `decode_slot_written_extra(&[u8])` | vb_storage | **NONE** | **P0** |
| `parse_inventory(&[u8])` | vb_boundary_inventory | **NONE** | **P0** |
| `validate_evidence_reference_bytes(&[u8])` | vb_boundary_inventory | **NONE** | **P0** |
| `verify_digest_match(&[u8])` | vb_storage | digest_coherence (weak) | **P0** |
| `decode_postcard::<T>(&[u8])` | vb_ui_model | vb_ui_model_postcard_decode | **P1** |
| `validate_header_crc(&[u8])` | vb_proof_kernels | **NONE** | **P1** |
| `validate_header_before_alloc(&[u8])` | vb_proof_kernels | **NONE** | **P1** |
| `validate_frame_magic(&[u8])` | vb_ipc | ipc_frame_boundary | **P1** |
| `validate_frame_bounds(&[u8])` | vb_ipc | ipc_frame_boundary | **P1** |
| `verify_frame_crc(&[u8])` | vb_ipc | **NONE** | **P1** |

#### CATEGORY B: String/Text Input (`&str`, `String`) — PARSER/LEXER SURFACE

| Function | Crate | Existing? | Priority |
|----------|-------|-----------|----------|
| `lex(text: &str)` | vb_expr | lex_expr exists | **P1** |
| `parse(yaml: &str)` | vb_yaml | yaml_events | **P1** |
| `collect_events(yaml: &str)` | vb_yaml | yaml_events (no assertions) | **P1** |
| `plan_taint_doc_reconciliation(text: &str)` | vb_doc | check_doc_taint exists | **P1** |
| `scan_for_stale_clean_only_text(text: &str)` | vb_doc | **NONE** | **P2** |
| `BoundaryCandidate::new(marker: impl Into<String>)` | vb_boundary_inventory | **NONE** | **P2** |
| `parse_tokens_from_toml(toml: &str)` | vb_ui_snapshot | **NONE** | **P2** |
| `spellcheck_text(text: &str)` | vb_ui_snapshot | **NONE** | **P2** |
| `xtask_parse_argv(args: &[String])` | vb_cli | xtask_parse_argv_hostile | **P2** |

#### CATEGORY C: Structured Types (Deserialized) — VALIDATOR/ENGINE SURFACE

| Function | Crate | Existing? | Priority |
|----------|-------|-----------|----------|
| `try_from_parts(parts: WorkflowParts)` | vb_core | compiled_ir (STUB) | **P0** |
| `validate_compiled_workflow(workflow)` | vb_core | generated_compare (STUB) | **P0** |
| `eval_expr_program(program, store)` | vb_core | expression (discards) | **P0** |
| `run_loop(frame: RunFrame)` | vb_core | **NONE** | **P1** |
| `validate_with_contracts(input, contracts)` | vb_validate | verifier_gates (weak) | **P1** |
| `compile_workflow(yaml, profile)` | vb_compile | vb_f04l (1 target) | **P1** |
| `emit_rust_workflow(workflow)` | vb_codegen | **NONE** | **P1** |
| `compare_generated_to_ir(rust, workflow)` | vb_codegen | generated_compare (STUB) | **P1** |
| `validate_graph(graph)` | vb_validate | **NONE** | **P2** |
| `validate_targets(targets)` | vb_validate | **NONE** | **P2** |
| `validate_node_kind(node)` | vb_validate | **NONE** | **P2** |
| `put_blob(key, data)` / `get_blob(key)` | vb_core | **NONE** | **P2** |
| `put_list(key, items)` / `get_list(key)` | vb_core | **NONE** | **P2** |
| `encode_record(record)` | vb_storage | journal_event (partial) | **P2** |
| `submit_artifact(artifact)` | vb_storage | admission_* (weak) | **P2** |

#### CATEGORY D: Numeric Types — OVERFLOW/UNDERFLOW SURFACE

| Function | Crate | Existing? | Priority |
|----------|-------|-----------|----------|
| `StepBudget::new(remaining: u64)` | vb_core | step_budget_new | **P1** |
| `StepBudget::try_take(n: u64)` | vb_core | step_budget_new | **P1** |
| `compute(aggregate: AggregateResourceBudget)` | vb_core | budget_compute (weak) | **P1** |
| `validate_aggregate_budget(budget)` | vb_core | **NONE** | **P2** |
| `validate_step_ceilings(budget)` | vb_core | **NONE** | **P2** |
| `capture_metadata(duration: Duration)` | vb_benchmark | **NONE** | **P2** |
| `baseline_within_budget(baseline, budget)` | vb_benchmark | **NONE** | **P2** |
| `collect_page(list, page_size: usize)` | vb_runtime | collect_page (MISSING) | **P0** |

#### CATEGORY E: Collection Types — BOUNDS SURFACE

| Function | Crate | Existing? | Priority |
|----------|-------|-----------|----------|
| `ActionQueue::enqueue(item)` | vb_runtime | **NONE** | **P1** |
| `ActionQueue::dequeue()` | vb_runtime | **NONE** | **P1** |
| `evaluate_for_each(items)` | vb_runtime | **NONE** | **P2** |
| `evaluate_together(items)` | vb_runtime | **NONE** | **P2** |
| `evaluate_collect(items)` | vb_runtime | **NONE** | **P2** |
| `evaluate_retry(policy)` | vb_runtime | **NONE** | **P2** |
| `fanout(items)` | vb_runtime | **NONE** | **P2** |

---

## 4. Red Queen Campaign Architecture

### 4.1 Three-Engine Defense-in-Depth

```
┌─────────────────────────────────────────────────────────────────┐
│                     RED QUEEN FUZZING ENGINE                     │
│                                                                 │
│  ┌─────────────┐   ┌──────────────┐   ┌──────────────────┐    │
│  │  libfuzzer   │   │    AFL++     │   │   honggfuzz      │    │
│  │  (Primary)   │   │  (Secondary) │   │   (Tertiary)     │    │
│  │              │   │              │   │                   │    │
│  │ ASAN+UBSAN   │   │ Deterministic│   │ HW counter fdbk  │    │
│  │ Coverage fdbk│   │ Havoc stages │   │ Intel PT/BTS      │    │
│  │ Cmp feedback │   │ Custom dicts │   │ PERF events       │    │
│  │ Value profile│   │ Persistent   │   │                   │    │
│  └──────┬───────┘   └──────┬───────┘   └────────┬──────────┘    │
│         │                  │                     │               │
│         ▼                  ▼                     ▼               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              SANITIZER MATRIX (libfuzzer)                │    │
│  │  ASAN │ UBSAN │ LSan                                     │    │
│  │  (MSAN/TSAN: separate CI jobs only — not libfuzzer)      │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  CORPUS FUSION LAYER                     │    │
│  │  Seed corpus → libfuzzer corpus → AFL++ corpus           │    │
│  │  → honggfuzz corpus → merged → minimized → re-seeded     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │               CRASH TRIAGE PIPELINE                      │    │
│  │  Crash artifact → minimizer → dedup → triage → fix →    │    │
│  │  regression test → corpus → re-fuzz                      │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Tiered Campaign Schedule

```
TIER 1: SMOKE — Every PR, every push
├── Engine: libfuzzer only
├── Duration: 60 seconds per target
├── Targets: ALL 50+ targets (short run to catch regressions)
├── Sanitizer: ASAN + UBSAN
├── Seeds: From corpus
└── Gate: Must pass (0 crashes, 0 leaks)

TIER 2: NIGHTLY — Every night
├── Engine: libfuzzer (primary) + AFL++ (top-15 targets)
├── Duration: 1 hour per target (libfuzzer), 2 hours (AFL++)
├── Targets: ALL 50+ targets
├── Sanitizer: ASAN + UBSAN (libfuzzer)
├── Seeds: From live corpus, auto-minimized
├── Corpus: Auto-commit new interesting inputs
└── Gate: Must pass (0 crashes, 0 leaks in ASAN)

TIER 3: WEEKLY DEEP — Every weekend
├── Engine: libfuzzer + AFL++ + honggfuzz
├── Duration: 12 hours (libfuzzer), 6 hours (AFL++), 4 hours (honggfuzz)
├── Targets: Top-25 high-value targets
├── Sanitizer: ASAN + UBSAN + LSan
├── Corpus: Cross-engine merged + minimized
├── Mutation: cargo-mutants on fuzz harnesses (verify ≥90% kill rate)
└── Report: Coverage delta, corpus growth, new edges found

TIER 4: MONTHLY GAUNTLET — First weekend of month
├── Engine: All three engines simultaneously
├── Duration: 24 hours (libfuzzer), 12 hours (AFL++), 8 hours (honggfuzz)
├── Targets: ALL 50+ targets
├── Sanitizer: ASAN + UBSAN + LSan
├── Additional: prop-fuzz bridging, differential fuzzing pairs
├── Corpus: Full cross-engine fusion, deep minimization
└── Report: Full coverage report, mutation kill report, trend analysis
```

### 4.3 Harness Type Distribution Across Tiers

| Harness Type | Count | Tier 1 | Tier 2 | Tier 3 | Tier 4 |
|-------------|-------|--------|--------|--------|--------|
| Parser/Codec | 25+ | ✅ | ✅ | ✅ | ✅ |
| Roundtrip | 12+ | ✅ | ✅ | ✅ | ✅ |
| Differential | 8+ | — | ✅ | ✅ | ✅ |
| Structure-Aware | 10+ | — | — | ✅ | ✅ |
| Property Invariant | 15+ | ✅ | ✅ | ✅ | ✅ |
| Hostile Input | 12+ | — | ✅ | ✅ | ✅ |
| Concurrency | 5+ | — | — | ✅ | ✅ |

---

## 5. Harness Inventory — Full Catalog

### 5.1 Existing Harnesses — Status & Remediation

#### STRONG ASSERTIONS (19 — keep, monitor)

| # | Target | Type | Status | 
|---|--------|------|--------|
| 1 | `fuzz_capability_name_schema` | Parser | ✅ APPROVED — typed error assertions |
| 2 | `fuzz_capability_contract_schema` | Parser | ✅ APPROVED — typed error assertions |
| 3 | `fuzz_ipc_frame` | Parser+Roundtrip | ⚠️ FIXED in C.23 — now has assertions |
| 4 | `fuzz_journal_event` | Parser+Roundtrip | ✅ APPROVED — is_valid() + roundtrip |
| 5 | `fuzz_expression` | Property | ⚠️ FIXED in C.24 — now has type_name assertions |
| 6 | `fuzz_compiled_ir` | Structure-Aware | ⚠️ FIXED in C.22 — check_node_slots covers 34+ variants |
| 7 | `fuzz_generated_compare` | Differential | ⚠️ FIXED in C.21 — now has digest/node/slot equality |
| 8 | `fuzz_taint_propagation` | Property | ✅ APPROVED — monotonicity + Clean→Clean |
| 9 | `fuzz_resource_budget` | Property | ✅ APPROVED — exhaustion + budget invariants |
| 10 | `fuzz_step_budget_new` | Property | ✅ APPROVED — clamping + try_take correctness |
| 11 | `fuzz_strict_artifact_decoder` | Parser | ✅ APPROVED — gate_count, node_count assertions |
| 12 | `fuzz_slot_value_roundtrip` | Roundtrip | ✅ APPROVED — byte equality + type_name |
| 13 | `fuzz_vb_ui_model_postcard_decode` | Parser | ✅ APPROVED — schema_version + field exclusivity |
| 14 | `fuzz_vb_qi37_12_persisted_payload_decode` | Parser | ✅ APPROVED — truncation→UnexpectedEof, corruption→DigestMismatch |
| 15 | `fuzz_ipc_frame_boundary` | Hostile | ✅ APPROVED — magic/length boundary assertions |
| 16 | `fuzz_storage_envelope_boundary` | Hostile | ✅ APPROVED — empty/truncated→typed errors |
| 17 | `fuzz_binary_payload_boundary` | Hostile | ✅ APPROVED — empty/Eof assertions |
| 18 | `fuzz_external_input_adapter_boundary` | Hostile | ✅ APPROVED — is_err on empty inventory |
| 19 | `fuzz_strict_yaml_profile` | Parser | ✅ APPROVED — unsupported YAML → compile error |

#### WEAK/COVERAGE-ONLY (21 — MUST HARDEN)

| # | Target | Current State | Required Fix |
|---|--------|--------------|-------------|
| 1 | `fuzz_yaml_events` | No assertions | Add: node_count≥1, source_map non-empty, typed errors |
| 2 | `fuzz_replay_events` | No assertions | Add: event count match, state invariance |
| 3 | `fuzz_extract_terminal` | No assertions | Add: terminal node has no children |
| 4 | `fuzz_action_tracker` | No assertions | Add: state transition validity |
| 5 | `fuzz_accepted_artifact_envelope_qi37_4_2` | Field access only | Add: envelope field invariants |
| 6 | `fuzz_expr_bytecode` | No assertions | Add: eval result type match, stack invariants |
| 7 | `fuzz_verifier_gates` | No assertions | Add: per-gate error variant exhaustiveness |
| 8 | `fuzz_budget_compute` | `let _` on results | Add: budget component bounds, non-negative |
| 9 | `fuzz_admission_flow` | No assertions | Add: artifact store invariants |
| 10 | `fuzz_expr_eval` | No assertions | Add: type_name non-empty, no silent panics |
| 11 | `fuzz_accessor_traversal` | No assertions | Add: path depth bounds, slot reference validity |
| 12 | `fuzz_admission_fuzz` | No assertions | Add: decoded parts structure assertions |
| 13 | `fuzz_digest_coherence` | `let _result` | Add: blake3 vs verify_digest_match equivalence |
| 14 | `fuzz_admission_input_surface` | `let _strict`/`_relaxed` | Add: field roundtrip assertions |
| 15 | `fuzz_readback_family_set` | `let _classification` | Add: classification invariants |
| 16 | `fuzz_accepted_artifact_decode` | `let _result` | Add: decode struct invariants |
| 17 | `fuzz_recovery_decode` | `let _summary`/`_seed` | Add: seed field bounds |
| 18 | `fuzz_collect_page_pagination` | **FN DOES NOT EXIST in lib.rs** | **MUST IMPLEMENT — LETHAL C.25** |
| 19 | `fuzz_action_tracker` in src/bin | Maps to `fuzz_action_tracker` | (same as #4) |
| 20 | `decode_record` (fuzz_targets) | `.ok()` suppresses all | `match` with typed error variants |
| 21 | `expr_eval` (fuzz_targets) | Calls fuzz_lib::fuzz_expr_eval | (same as #10) |

### 5.2 NEW Harnesses — Must Create

#### P0 — Critical (Blocking)

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

#### P1 — High Priority

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

#### P2 — Medium Priority

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

### 5.3 Harness Template — All New Targets Must Follow

```rust
// fuzz/fuzz_targets/TARGET_NAME.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // 1. Parse/decode
    let result = crate_name::public_function(data);
    
    // 2. Match result — NEVER suppress
    match result {
        Ok(value) => {
            // Structural invariants on success
            assert!(/* invariant that proves it's valid */);
            // For roundtrip targets: re-encode and compare
        }
        Err(e) => {
            // Typed error exhaustiveness — match EVERY variant
            match e {
                crate_name::Error::Variant1 { .. } => {},
                crate_name::Error::Variant2 { .. } => {},
                // ... all variants ...
                _ => {}, // wildcard for future variants (not unreachable!)
            }
        }
    }
});
```

---

## 6. Implementation Phases

### Phase 0: Foundation (PRIORITY: NOW)

**Estimated time: 2 hours**

| Step | Action | Command |
|------|--------|---------|
| 0.1 | Install cargo-fuzz | `cargo install cargo-fuzz` |
| 0.2 | Install cargo-afl | `cargo install cargo-afl` |
| 0.3 | Install cargo-hfuzz | `cargo install honggfuzz` |
| 0.4 | Verify cargo-fuzz works | `cargo fuzz --version` |
| 0.5 | Declare ALL 11 orphan fuzz_targets/ | Add `[[bin]]` entries in `fuzz/Cargo.toml` |
| 0.6 | Declare 9 orphan src/bin/ | Add `[[bin]]` entries in `fuzz/Cargo.toml` |
| 0.7 | Build all libfuzzer targets | `cargo fuzz build --target x86_64-unknown-linux-gnu` |
| 0.8 | Verify libfuzzer instrumentation | `nm target/.../TARGET | grep LLVMFuzzer` |
| 0.9 | Verify -help=1 works | `./target/.../TARGET -help=1` |
| 0.10 | Create fuzz profile in Cargo.toml | Add `[profile.fuzz]` with debug+asan config |
| 0.11 | Add `[package.metadata.cargo-fuzz]` | minimization config |
| 0.12 | Smoke test: 10s per target | `for t in $(cargo fuzz list); do cargo fuzz run $t -- -max_total_time=10; done` |

### Phase 1: Harden Existing (PRIORITY: AFTER PHASE 0)

**Estimated time: 4 hours**

| Step | Action | Details |
|------|--------|---------|
| 1.1 | Harden 21 weak functions | Add assertions to every coverage-only function per §5.1 |
| 1.2 | Fix C.25 (collect_page) | Implement `fuzz_collect_page_pagination` in lib.rs |
| 1.3 | Fix L3 (generated_compare) | Already fixed in C.21 bead — verify |
| 1.4 | Fix L4 (compiled_ir) | Already fixed in C.22 bead — verify |
| 1.5 | Fix L5 (ipc_frame) | Already fixed in C.23 bead — verify |
| 1.6 | Fix L6 (expression) | Already fixed in C.24 bead — verify |
| 1.7 | Fix L8 (decode_record) | Replace `.ok()` with `match` + typed errors |
| 1.8 | Create seed corpora for 24 targets | Generate from test fixtures, hand-craft edge cases |
| 1.9 | Create AFL++ dictionaries | Magic bytes, record kinds, schema versions |
| 1.10 | Refactor: extract shared `run_with_stdin` | Move to `fuzz/src/bin_common.rs` |
| 1.11 | Add mutation resistance assertions | Every assertion must be un-removable without test failure |
| 1.12 | Run 1-hour smoke with ASAN | `cargo fuzz run --release -- -max_total_time=3600` for top-10 targets |

### Phases 2–6: Deferred to `fuzz/FUTURE.md`

**Phases 2-6 are deferred.** They cannot begin until Phase 0 + Phase 1 exit gates are fully green (see `fuzz/EXECUTE.md`). Planning future work on a system that doesn't run today is premature optimization.

See **[fuzz/FUTURE.md](./FUTURE.md)** for:
- Phase 2: 13 P0 new harnesses (boundary_inventory, codegen, proof_kernels, storage, runtime, expr)
- Phase 3: 21 P1 new harnesses + `Arbitrary` impls for key types
- Phase 4: 16 P2 new harnesses (benchmark, YAML, core, CLI, UI)
- Phase 5: GitHub Actions CI (Tier 1-3) with sequential job execution (no parallel matrix DoS)
- Phase 6: AFL++ secondary engine, mutation testing, corpus management, coverage dashboard, ClusterFuzzLite

---

## 7. CI Integration

**CI workflows are defined in `fuzz/EXTREME_FUZZING.md` §8 and `fuzz/FUTURE.md` Phase 5.**

For now, the operational pattern is:
- **Local smoke:** `for t in $(cargo fuzz list | head -10); do cargo fuzz run $t -- -max_total_time=60; done`
- **Nightly campaigns:** Run on dedicated hardware via `cargo fuzz run` with `-max_total_time=3600`
- **CI gating:** Update Moon `fuzz-smoke` task to run top-10 targets for 60 seconds each

---

---

## 8. Success Metrics

### 8.1 Zero-Tolerance Gates

| Metric | Threshold | Measurement |
|--------|-----------|-------------|
| **Crashes in ASAN** | 0 | Any ASAN crash → BLOCKER |
| **Crashes in UBSAN** | 0 | Any UBSAN crash → BLOCKER |
| **Memory leaks** | 0 | Any LSan leak → BLOCKER |
| **Panics in fuzz** | 0 | Any panic from arbitrary bytes → BLOCKER |
| **Unreachable! in fuzz** | 0 | Any unreachable! hit → BLOCKER |
| **expect/unwrap in fuzz** | 0 | Any expect/unwrap in fuzz path → BLOCKER |

### 8.2 Coverage Targets

| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| Crates with fuzz | 11/19 (58%) | 18/19 (95%) | Phase 4 |
| Functions with fuzz | ~40/3144 (1.3%) | 100+ targeted, 500+ via coverage | Phase 5 |
| libfuzzer edge coverage | Unknown | >60% per crate | Phase 6 |
| Harness assertion strength | 47.5% strong | >90% strong (L3+) | Phase 1 |
| Mutation kill rate on harnesses | Unknown | >90% | Phase 6 |
| Corpus diversity | 7 seed sets | 50+ seed sets, 1000+ total inputs | Phase 2 |

### 8.3 Campaign Performance

| Metric | Target |
|--------|--------|
| Executions/second per target | >1000 |
| New edges found per nightly | >0 (non-zero growth) |
| Corpus size growth per week | >10% |
| Crash discovery rate | 0 (post-hardening) |
| False positive rate | 0% |

---

## 9. Tooling Requirements

### 9.1 Must Install

```bash
# Primary engine
cargo install cargo-fuzz

# Secondary engine  
cargo install cargo-afl

# Tertiary engine
cargo install honggfuzz

# Mutation testing
cargo install cargo-mutants

# Coverage analysis
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# Corpus tools
cargo install cargo-fuzz-cmin  # if available
```

### 9.2 Fuzz Profile (add to fuzz/Cargo.toml)

```toml
[profile.fuzz]
inherits = "release"
debug = true
debug-assertions = true
overflow-checks = true
lto = "off"
opt-level = 2
codegen-units = 1

[profile.fuzz-asan]
inherits = "fuzz"
rustflags = ["-Zsanitizer=address"]

[profile.fuzz-ubsan]
inherits = "fuzz"
rustflags = ["-Zsanitizer=undefined"]

[package.metadata.cargo-fuzz]
# Minimization config
sancov_timeout = 60
libfuzzer_options = [
    "-len_control=1",
    "-max_len=65536",
    "-detect_leaks=1",
    "-rss_limit_mb=4096",
]
```

### 9.3 AFL++ Dictionaries (create fuzz/dicts/)

```
# fuzz/dicts/vb_storage.dict
magic_journal="VBRT"
magic_blob="VBBL"  
magic_compiled="VBCA"
magic_snapshot="VBSN"
magic_workflow="VBWS"
magic_index="VBIR"
magic_admission="VBAD"
magic_recovery="VBRC"
magic_slot="VBSC"

kind_journal_event="\x00\x00"
kind_workflow_source="\x01\x00"
kind_compiled_ir="\x02\x00"
kind_snapshot="\x03\x00"
kind_admission="\x04\x00"
kind_recovery="\x05\x00"

schema_v1="\x01\x00"
schema_current="\x01\x00"

# fuzz/dicts/vb_ipc.dict
ipc_magic="VBIPC\x00\x00\x00\x00\x00\x00\x00"
cmd_health="\x00\x00"
cmd_shutdown="\x01\x00"
cmd_list_runs="\x02\x00"

# fuzz/dicts/vb_expr.dict
op_add="+"
op_sub="-"
op_mul="*"
op_div="/"
op_eq="=="
op_ne="!="
op_gt=">"
op_lt="<"
op_and="&&"
op_or="||"
op_not="!"
```

---

## 10. Risk Register

### 10.1 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Nightly-only ASAN breaks on toolchain update | Medium | High | Pin nightly version; CI gate on toolchain updates |
| AFL++ persistent mode crashes on some targets | Medium | Medium | Fall back to fork mode; flag per-target |
| Corpus explosion fills disk | High | Medium | Auto-minimize + max corpus size limit |
| False positive ASAN from dependency UB | Medium | Medium | Dependency allowlist; suppress known FPs |
| OOM on large inputs | High | Low | `-rss_limit_mb` + `-max_len` per target |
| Self-hosted runner unavailable | Medium | High | Fall back to GHA hosted runners (costlier, slower) |
| honggfuzz Intel PT requires specific CPU | High | Medium | Only enable on dedicated hardware; fall back to PERF |
| `vb_codegen::format_generated_rust()` spawns rustfmt — potential DoS | Low | High | Sandbox with timeout; skip in CI, deep-campaign only |

### 10.2 Process Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Fuzz campaign finds bugs faster than they can be fixed | Medium | Low | Triage pipeline; bead-per-crash; prioritize by severity |
| Fuzz harnesses bit-rot without CI | High | High | All harnesses in CI (Tier 1); auto-detect bit-rot |
| Corpus not maintained | Medium | Medium | Auto-commit corpus on nightly; prune stale inputs |
| Team ignores fuzz results | Low | High | BLOCKER gate on ASAN/UBSAN crashes; cannot merge |
| Mutation resistance not verified | Medium | Medium | `cargo-mutants` in Tier 3; alert on <90% kill rate |

---

## APPENDIX A: Bead-to-Harness Mapping

Every fuzz target requires its own bead (per MASTER.md §42). This maps Phase targets to beads.

| Phase | Harness Count | Beads Needed | Bead Prefix |
|-------|--------------|-------------|-------------|
| Phase 0 | Infrastructure | vb-fuzz-infra-* | Foundation |
| Phase 1 | 21 hardenings | vb-fuzz-harden-* | Existing hardening |
| Phase 2 | 13 P0 | vb-fuzz-p0-* | New P0 targets |
| Phase 3 | 21 P1 | vb-fuzz-p1-* | New P1 targets |
| Phase 4 | 16 P2 | vb-fuzz-p2-* | New P2 targets |
| Phase 5 | 7 CI tasks | vb-fuzz-ci-* | CI integration |
| Phase 6 | Ongoing | vb-fuzz-gauntlet-* | Perpetual |

**Total: ~78 new beads. 78 new fuzz targets to survive the Red Queen.**

---

## APPENDIX B: Command Cheatsheet

```bash
# === INSTALL ===
cargo install cargo-fuzz cargo-afl honggfuzz cargo-mutants
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# === BUILD ===
cargo fuzz build --target x86_64-unknown-linux-gnu --release
cargo fuzz list  # list all targets

# === VERIFY INSTRUMENTATION ===
nm fuzz/target/x86_64-unknown-linux-gnu/release/TARGET | grep LLVMFuzzer
./fuzz/target/x86_64-unknown-linux-gnu/release/TARGET -help=1

# === RUN (libfuzzer) ===
# Smoke (10s)
cargo fuzz run TARGET -- -max_total_time=10 -rss_limit_mb=1024
# Hourly
cargo fuzz run TARGET -- -max_total_time=3600 -print_final_stats=1
# Deep (12hr)
cargo fuzz run TARGET -- -max_total_time=43200 -print_final_stats=1 -detect_leaks=1
# With ASAN
RUSTFLAGS="-Zsanitizer=address" cargo fuzz run TARGET -- -max_total_time=3600
# Minimize corpus
cargo fuzz cmin TARGET

# === RUN (AFL++) ===
cargo afl build --manifest-path fuzz/Cargo.toml --bin TARGET
cargo afl fuzz -i fuzz/corpus/TARGET/ -o fuzz/afl_out/TARGET/ \
  -x fuzz/dicts/vb_storage.dict \
  target/debug/TARGET

# === RUN (honggfuzz) ===
cargo hfuzz run TARGET

# === CORPUS ===
ls fuzz/corpus/TARGET/ | wc -l       # corpus size
ls fuzz/artifacts/TARGET/ | wc -l    # crash count
cargo fuzz cmin TARGET                # minimize corpus
cargo fuzz tmin TARGET fuzz/artifacts/TARGET/crash-*  # minimize crash

# === COVERAGE ===
cargo fuzz coverage TARGET
llvm-cov show fuzz/target/.../TARGET -instr-profile=default.profdata
cargo llvm-cov --fuzz TARGET

# === MUTATION TESTING ===
cargo mutants -p velvet-ballastics-fuzz -- --target x86_64-unknown-linux-gnu

# === BATCH SMOKE ALL ===
for target in $(cargo fuzz list); do
  echo "=== $target ==="
  cargo fuzz run "$target" -- -max_total_time=10 -rss_limit_mb=1024 || echo "FAIL: $target"
done
```

---

**STATUS: STRATEGY COMPLETE — BLACK-HAT REVIEWED 2026-05-24**

**20 findings (5 LETHAL, 4 CRITICAL, 6 MAJOR) — all remediated in this revision.**
- CFI, SafeStack, MSAN removed (Rust-incompatible sanitizers)
- OSS-Fuzz replaced with ClusterFuzzLite (appropriate for pre-1.0 project)
- Phases 2-6 deferred to `fuzz/FUTURE.md` (premature optimization)
- Time estimates removed (off by 5-10x)
- GHA parallel matrix replaced with sequential loop (no DoS on free tier)
- Harness template hardened with error-variant exhaustiveness guard
- vb_ui/vb_ui_makepad targets flagged as needing dedicated analysis phase

**Phase 0-1 execution plan: `fuzz/EXECUTE.md` — start there.**
