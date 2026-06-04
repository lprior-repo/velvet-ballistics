# Velvet Ballistics — Fuzz Infrastructure Inventory

**Generated:** 2026-05-24
**Package:** `velvet-ballistics-fuzz` v0.1.0 (edition 2024)
**libfuzzer-sys:** 0.4.12 (verified in Cargo.lock)

---

## 1. Package Metadata & Feature Flags

| Field | Value |
|-------|-------|
| `name` | `velvet-ballistics-fuzz` |
| `edition` | `2024` |
| `publish` | `false` |
| `autobins` | `false` |
| `[features]` | `fuzz = []` (unstable feature gate) |
| `cargo-fuzz` | `true` (in `[package.metadata]`, workaround for TOML parser) |
| `[lib]` name | `fuzz_lib` (path: `src/lib.rs`, 3010 lines) |

**Lints:** `unsafe_code = "forbid"`, `unused_must_use = "deny"`, `unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`

---

## 2. Dependencies

| Dependency | Version/Path | Notes |
|------------|------------|-------|
| `libfuzzer-sys` | `0.4` **(0.4.12 locked)** | libFuzzer runtime |
| `postcard` | `1` (features: `alloc`) | Binary serialization (primary fuzz codec) |
| `blake3` | `1` | Hashing for digest computation |
| `bytes` | `1` (features: `serde`) | Byte buffers |
| `tempfile` | `3` | Temporary directories for journal fuzzing |
| `vb_boundary_inventory` | `../crates/vb_boundary_inventory` | Boundary inventory fuzz |
| `vb_core` | `../crates/vb_core` | Core types, engine, workflow |
| `vb_expr` | `../crates/vb_expr` | Expression lexer/parser |
| `vb_storage` | `../crates/vb_storage` | Journal, codec, admission |
| `vb_runtime` | `../crates/vb_runtime` | Runtime admission, primitives |
| `vb_validate` | `../crates/vb_validate` | Verifier gates |
| `vb_yaml` | `../crates/vb_yaml` | YAML event parser |
| `vb_compile` | `../crates/vb_compile` | YAML compile |
| `vb_ipc` | `../crates/vb_ipc` | IPC frame encode/decode |
| `vb_ui_model` | `../crates/vb_ui_model` | UI model envelope |

---

## 3. `[[bin]]` Entries (Total: 38)

### 3.1 From `fuzz_targets/` — Proper libfuzzer harnesses (1 entry)

| # | Name | Path | Harness Type |
|---|-------|------|-------------|
| 1 | `vb_f04l_yaml_compiler_compile` | `fuzz_targets/vb_f04l_yaml_compiler_compile.rs` | libfuzzer (`#![no_main]` + `fuzz_target!()`) |

### 3.2 From `src/bin/` — Stdin-based harnesses (37 entries)

All 37 follow the **identical** stdin-based pattern:
```rust
#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    run_with_stdin(fuzz_lib::fuzz_TARGETNAME)
}
// ... replicated run_with_stdin + write_stderr helpers in every file
```

| # | Name | Path | Invokes |
|---|-------|------|---------|
| 2 | `vb_qi37_12_persisted_payload_decode` | `src/bin/vb_qi37_12_persisted_payload_decode.rs` | `fuzz_vb_qi37_12_persisted_payload_decode` |
| 3 | `journal_event` | `src/bin/journal_event.rs` | `fuzz_journal_event` |
| 4 | `capability_name_schema` | `src/bin/capability_name_schema.rs` | `fuzz_capability_name_schema` |
| 5 | `capability_contract_schema` | `src/bin/capability_contract_schema.rs` | `fuzz_capability_contract_schema` |
| 6 | `compiled_ir` | `src/bin/compiled_ir.rs` | `fuzz_compiled_ir` |
| 7 | `expression` | `src/bin/expression.rs` | `fuzz_expression` |
| 8 | `generated_compare` | `src/bin/generated_compare.rs` | `fuzz_generated_compare` |
| 9 | `collect_page` | `src/bin/collect_page_pagination.rs` | `fuzz_collect_page_pagination` |
| 10 | `yaml_events` | `src/bin/yaml_events.rs` | `fuzz_yaml_events` |
| 11 | `ipc_frame` | `src/bin/ipc_frame.rs` | `fuzz_ipc_frame` |
| 12 | `strict_artifact_decoder` | `src/bin/strict_artifact_decoder.rs` | `fuzz_strict_artifact_decoder` |
| 13 | `digest_coherence` | `src/bin/digest_coherence.rs` | `fuzz_digest_coherence` |
| 14 | `readback_family_set` | `src/bin/readback_family_set.rs` | `fuzz_readback_family_set` |
| 15 | `admission_input_surface` | `src/bin/admission_input_surface.rs` | `fuzz_admission_input_surface` |
| 16 | `accepted_artifact_decode` | `src/bin/accepted_artifact_decode.rs` | `fuzz_accepted_artifact_decode` |
| 17 | `accepted_artifact_envelope_qi37_4_2` | `src/bin/accepted_artifact_envelope_qi37_4_2.rs` | `fuzz_accepted_artifact_envelope_qi37_4_2` |
| 18 | `accessor_traversal` | `src/bin/accessor_traversal.rs` | `fuzz_accessor_traversal` |
| 19 | `action_tracker` | `src/bin/action_tracker.rs` | `fuzz_action_tracker` |
| 20 | `admission_flow` | `src/bin/admission_flow.rs` | `fuzz_admission_flow` |
| 21 | `admission_fuzz` | `src/bin/admission_fuzz.rs` | `fuzz_admission_fuzz` |
| 22 | `binary_payload_fuzz_boundary` | `src/bin/binary_payload_fuzz_boundary.rs` | `fuzz_binary_payload_boundary` |
| 23 | `budget_compute` | `src/bin/budget_compute.rs` | `fuzz_budget_compute` |
| 24 | `expr_bytecode` | `src/bin/expr_bytecode.rs` | `fuzz_expr_bytecode` |
| 25 | `expr_eval` | `src/bin/expr_eval.rs` | `fuzz_expr_eval` |
| 26 | `external_input_adapter_fuzz` | `src/bin/external_input_adapter_fuzz.rs` | `fuzz_external_input_adapter_boundary` |
| 27 | `extract_terminal` | `src/bin/extract_terminal.rs` | `fuzz_extract_terminal` |
| 28 | `ipc_decode` | `src/bin/ipc_decode.rs` | `fuzz_ipc_decode` |
| 29 | `ipc_frame_fuzz_boundary` | `src/bin/ipc_frame_fuzz_boundary.rs` | `fuzz_ipc_frame_boundary` |
| 30 | `recovery_decode` | `src/bin/recovery_decode.rs` | `fuzz_recovery_decode` |
| 31 | `replay_events` | `src/bin/replay_events.rs` | `fuzz_replay_events` |
| 32 | `resource_budget` | `src/bin/resource_budget.rs` | `fuzz_resource_budget` |
| 33 | `slot_value_roundtrip` | `src/bin/slot_value_roundtrip.rs` | `fuzz_slot_value_roundtrip` |
| 34 | `step_budget_new` | `src/bin/step_budget_new.rs` | `fuzz_step_budget_new` |
| 35 | `storage_envelope_fuzz_boundary` | `src/bin/storage_envelope_fuzz_boundary.rs` | `fuzz_storage_envelope_boundary` |
| 36 | `strict_yaml_profile` | `src/bin/strict_yaml_profile.rs` | `fuzz_strict_yaml_profile` |
| 37 | `taint_propagation` | `src/bin/taint_propagation.rs` | `fuzz_taint_propagation` |
| 38 | `vb_ui_model_postcard_decode` | `src/bin/vb_ui_model_postcard_decode.rs` | `fuzz_vb_ui_model_postcard_decode` |
| 39 | `verifier_gates` | `src/bin/verifier_gates.rs` | `fuzz_verifier_gates` |

**Note:** `verifier_gates` is the 39th bin listed in `src/bin/` but it's entry 39 in the Cargo.toml `[[bin]]` list (there are 38 `[[bin]]` entries — counted: 1 from fuzz_targets + 37 from src/bin = 38 total). Correction: rechecking, there are **38** `[[bin]]` entries total.

### 3.3 Orphan files in `src/bin/` — NOT declared in `[[bin]]` (8 files)

These files exist on disk but have no `[[bin]]` entry — they are dead code, unreachable by cargo-fuzz:

1. `src/bin/aggregate_artifact_budget.rs`
2. `src/bin/aggregate_workflow_budget.rs`
3. `src/bin/boundary_evidence_reference.rs`
4. `src/bin/boundary_inventory_parser.rs`
5. `src/bin/boundary_metadata.rs`
6. `src/bin/recover_runtime_frame_seed_contract.rs`
7. `src/bin/structured_status_render_hostile.rs`
8. `src/bin/xtask_parse_argv_hostile.rs`
9. `src/bin/xtask_parse_options_hostile.rs`

### 3.4 Orphan files in `fuzz_targets/` — NOT declared in `[[bin]]` (11 files)

These are proper libfuzzer harnesses (`#![no_main]` + `fuzz_target!()`) but `cargo-fuzz` cannot see them because they lack `[[bin]]` entries:

1. `fuzz_targets/check_doc_taint_consistency_accepts_arbitrary_markdown.rs`
2. `fuzz_targets/decode_record.rs`
3. `fuzz_targets/expr_eval.rs`
4. `fuzz_targets/journal_event.rs` (collides with `src/bin/journal_event.rs`)
5. `fuzz_targets/lex_expr.rs`
6. `fuzz_targets/ui_redaction_artifact.rs`
7. `fuzz_targets/vb_5xs4_generated_source_mapping.rs`
8. `fuzz_targets/vb_5xs4_inventory_report.rs`
9. `fuzz_targets/vb_5xs4_label_sufficiency.rs`
10. `fuzz_targets/vb_5xs4_scan_source_text.rs`
11. `fuzz_targets/vb_storage_codec.rs`

---

## 4. `fuzz/src/lib.rs` — `pub fn fuzz_*` Function Inventory

**File:** `fuzz/src/lib.rs` (3010 lines)

### 4.1 Functions with STRONG assertions

These have concrete `assert!()` / `assert_eq!()` on behavioral invariants:

| Function | Lines | Key Assertions |
|----------|-------|---------------|
| `fuzz_capability_name_schema` | 29–53 | Asserts empty name → `CapabilityNameEmpty`, invalid → `CapabilityNameInvalid`, valid → `is_ok()` |
| `fuzz_capability_contract_schema` | 56–94 | Asserts `CapabilityNameEmpty`, `CapabilityNameInvalid`, `CapabilityActionMismatch`, `CapabilityDuplicate` |
| `fuzz_ipc_frame` | 189–277 | Header encode/decode roundtrip byte equality, payload length match/mismatch assertions, typed error path assertions |
| `fuzz_journal_event` | 315–373 | `is_valid()` assertion on decoded events, roundtrip encode/decode assertion, exhaustive typed error match |
| `fuzz_expression` | 418–444 | `type_name` non-empty assertion on evaluation result |
| `fuzz_compiled_ir` | 447–499 | Node count ≥ 1, slot count ≥ 1, digest preservation, node count match, **all slot bounds in all node kinds** (30+ node kind assertions) |
| `fuzz_generated_compare` | 793–830 | Validation/workflow-construction agreement, independent decode digest/node/slot equality |
| `fuzz_taint_propagation` | 921–1030 | Taint monotonicity (output ≥ max input), Clean→Clean invariant |
| `fuzz_resource_budget` | 1055–1175 | Zero-budget → zero transitions + `StepBudgetExhausted`, executed ≤ budget invariants |
| `fuzz_step_budget_new` | 2317–2388 | `remaining ≤ MAX_STEP_BUDGET`, exact clamping math, try_take increment/decrement correctness |
| `fuzz_strict_artifact_decoder` | 1965–1997 | `gate_count > 0`, `accepted_at_seq ≥ 1`, `node_count ≤ u16::MAX` |
| `fuzz_slot_value_roundtrip` | 1862–1904 | Roundtrip byte equality, `display_with_store` non-empty, `type_name` non-empty, deterministic `is_true` |
| `fuzz_vb_ui_model_postcard_decode` | 2398–2423 | `schema_version ≥ 1`, diagnostic/payload field exclusivity assertions |
| `fuzz_vb_qi37_12_persisted_payload_decode` | 2432–2533 | Truncated → `UnexpectedEof`, corrupted → `PayloadDigestMismatch` |
| `fuzz_ipc_frame_boundary` | 2559–2627 | `HeaderDecodeFailed` on partial frame, `InvalidMagic`/`HeaderDecodeFailed` on wrong magic, typed error exhaustiveness |
| `fuzz_storage_envelope_boundary` | 2672–2854 | `UnexpectedEof` on empty, truncated → `UnexpectedEof`/`HeaderLengthMismatch`, typed error exhaustiveness |
| `fuzz_binary_payload_boundary` | 2795–2854 | `UnexpectedEof` on empty, payload size boundary assertions |
| `fuzz_external_input_adapter_boundary` | 2872–2900 | `is_err()` on empty inventory, typed error exhaustiveness |
| `fuzz_strict_yaml_profile` | 2244–2256 | Unsupported YAML features must cause compile error, workflow `node_count ≥ 1` |

### 4.2 Functions that are COVERAGE-ONLY (panic-freedom only)

These verify that functions never panic but make no behavioral assertions:

| Function | Lines | Notes |
|----------|-------|-------|
| `fuzz_yaml_events` | 180–186 | Calls validate/parse/build — no assertions |
| `fuzz_replay_events` | 376–382 | Calls replay — no assertions |
| `fuzz_extract_terminal` | 385–390 | Calls extract_terminal — no assertions |
| `fuzz_action_tracker` | 393–415 | Exercises tracker state transitions — no assertions |
| `fuzz_accepted_artifact_envelope_qi37_4_2` | 781–790 | Field access only, explicitly documented as coverage-only |
| `fuzz_expr_bytecode` | 840–909 | Evaluates arbitrary bytecode — no assertions |
| `fuzz_verifier_gates` | 1188–1242 | Drops all gate results — no assertions |
| `fuzz_budget_compute` | 1362–1409 | Computes budget — only `let _` on results, explicitly coverage-only |
| `fuzz_admission_flow` | 1486–1594 | Submits to journal — no assertions on results |
| `fuzz_expr_eval` | 1604–1637 | Evaluates expressions — explicitly coverage-only |
| `fuzz_accessor_traversal` | 1650–1853 | Traverses accessor paths — no assertions |
| `fuzz_admission_fuzz` | 1915–1946 | Submits decoded WorkflowParts — explicitly coverage-only |
| `fuzz_digest_coherence` | 2009–2058 | `let _result` on admission — explicitly coverage-only |
| `fuzz_admission_input_surface` | 2209–2239 | `let _strict` / `let _relaxed` — explicitly coverage-only |
| `fuzz_readback_family_set` | 2070–2125 | `let _classification` — explicitly coverage-only |
| `fuzz_accepted_artifact_decode` | 2261–2279 | `let _result` — explicitly coverage-only |
| `fuzz_recovery_decode` | 2283–2299 | `let _summary` / `let _seed` — explicitly coverage-only |
| `fuzz_collect_page_pagination` | 2935–3010 | `let _result` — no assertions |

### 4.3 Typed error assertion helpers

| Function | Lines | Asserts exhaustiveness on |
|----------|-------|--------------------------|
| `assert_typed_ipc_error` | 2630–2651 | `IpcError` (13 variants + wildcard) |
| `assert_typed_journal_error` | 2727–2778 | `JournalError` (30+ variants + wildcard) |
| `assert_typed_boundary_error` | 2903–2925 | `BoundaryInventoryError` (13 variants + wildcard) |
| `assert_malformed_decode_is_typed` | 2516–2533 | `JournalError` decode variants (11 + wildcard) |

---

## 5. `fuzz/fuzz_targets.rs` — Bridge Module (101 lines)

This file provides callable Rust wrappers and C ABI stubs. Located at `fuzz/fuzz_targets.rs` (top-level in fuzz package).

**Callable wrappers** (delegate to `fuzz_lib::fuzz_*`):
- `yaml_events(data)`, `ipc_frame(data)`, `journal_event(data)`, `expression(data)`, `compiled_ir(data)`
- `generated_compare(data)`, `expr_bytecode(data)`, `taint_propagation(data)`, `resource_budget(data)`
- `expr_eval(data)`, `accessor_traversal(data)`, `slot_value_roundtrip(data)`, `admission_fuzz(data)`
- `vb_ui_model_postcard_decode(data)`

**C ABI stubs** (stub implementations returning 0):
- `LLVMFuzzerTestOneInputYamlEvents`, `LLVMFuzzerTestOneInputIpcFrame`, `LLVMFuzzerTestOneInputJournalEvent`, `LLVMFuzzerTestOneInputExpression`, `LLVMFuzzerTestOneInputCompiledIr`

All tagged `#[unsafe(no_mangle)] pub extern "C"`, return `0`.

---

## 6. Corpus Directory

**Location:** `fuzz/corpus/`

| Corpus | Status |
|--------|--------|
| `compiled_ir/` | Present |
| `decode_record/` | Present |
| `expr_eval/` | Present |
| `ipc_frame/` | Present |
| `journal_event/` | Present |
| `vb_f04l_yaml_compiler_compile/` | Present |
| `yaml_events/` | Present |
| All other targets | **MISSING** (no seed corpus) |

---

## 7. Automation

### 7.1 Moon CI: `fuzz-smoke` task

Defined in `.moon/tasks/all.yml`. Builds and runs the original 4 targets for 1 second each:

```yaml
fuzz-smoke:
  build: cargo fuzz build --target x86_64-unknown-linux-gnu
  run targets: yaml_events, ipc_frame, journal_event, compiled_ir
  each: cargo fuzz run "${target}" --target x86_64-unknown-linux-gnu -- -max_total_time=1
  timeout: 30s per target
  inputs: fuzz/**/*
  outputs: target/fuzz-smoke/**
```

**Key finding:** Only 4 of 38 declared targets are run in CI. The other 34 declared targets are **never run by any automation**.

### 7.2 `scripts/fuzz-minimization.sh`

Simple wrapper that passes libfuzzer minimization flags:
```bash
cargo fuzz run "$TARGET" \
    --target x86_64-unknown-linux-gnu \
    -- \
    -len_control=1 \
    -minimize_contribs=1 \
    "$@"
```

### 7.3 No `fuzz/scripts/` directory

There is no `fuzz/scripts/` directory; automation lives in the repo-root `scripts/fuzz-minimization.sh`.

---

## 8. `fuzz/REDO_PLAN.md` — Critical Context

This document (114 lines) describes the **known broken state**:

1. **All `src/bin/*.rs` binaries are stdin-based**, not libfuzzer — they lack ASAN, coverage feedback, and corpus mutation
2. **Most `fuzz_targets/*.rs` files are undeclared** in Cargo.toml → cargo-fuzz ignores them
3. **The original 12-hour run** used plain Rust binaries (no sanitizers, no coverage), only proving panic-freedom
4. **Phase plan**: Convert 6 strongest assertion targets to proper libfuzzer harnesses in `fuzz_targets/`, wire into Cargo.toml with `_fuzz` suffixes, seed corpus, run 12 hours

---

## 9. Architecture Summary

```
fuzz/
├── Cargo.toml              ← 38 [[bin]] entries, 1 proper libfuzzer, 37 stdin-based
├── Cargo.lock              ← libfuzzer-sys 0.4.12
├── src/
│   ├── lib.rs              ← 3010 lines, ~40 fuzz functions (19 strong, 21 coverage-only)
│   └── bin/                ← 47 files, 38 declared + 9 orphans
├── fuzz_targets/           ← 12 files, 1 declared + 11 orphans (proper libfuzzer harnesses)
├── fuzz_targets.rs          ← Bridge: 14 callable wrappers + 5 empty C ABI stubs
├── corpus/                 ← 7 seed corpora (24 targets MISSING)
├── README.md               ← Explains layout & musl/gnu target issue
└── REDO_PLAN.md            ← Known-broken-state plan
```

### Key Risks

1. **37 targets are stdin-based, not libfuzzer** — no ASAN, no coverage feedback, no corpus evolution
2. **11 proper libfuzzer harnesses are unreachable** — missing `[[bin]]` entries
3. **24 targets have no seed corpus** — libfuzzer starts from scratch
4. **Only 4 of 38 targets run in CI** — 34 targets never exercised
5. **Code duplication**: `run_with_stdin` pattern duplicated in every `src/bin/*.rs` file
6. **C ABI stubs in `fuzz_targets.rs`** are non-functional (return 0 without calling anything)
7. **`fuzz/` feature flag** gates all `src/bin/` targets but has no effect on `fuzz_targets/` harnesses
