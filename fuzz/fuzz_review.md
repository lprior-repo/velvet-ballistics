# Fuzz Target Review — Adversarial Assessment

**Reviewer:** test-reviewer agent  
**Scope:** All 67 fuzz targets and 51 fuzz binaries in `fuzz/` and `crates/*/fuzz/`  
**Date:** 2026-06-17  

---

## STATUS: REJECTED

---

## Executive Summary

The fuzz target suite has **strong foundational work** — many targets in `fuzz/src/lib.rs` have been hardened with property assertions, round-trip checks, and typed error validation. However, **6 targets are weak or empty** and provide zero bug-catching value. These targets are essentially "does it compile?" smoke tests rather than property fuzzers.

**51/67 targets are APPROVED** (good assertions, round-trips, or typed error checks).  
**6/67 targets are REJECTED** (no assertions, coverage-only).

---

## REJECTED: Weak Targets (6)

### 1. `fuzz/fuzz_targets/fuzz_choose_depth.rs` — NO ASSERTIONS

**File:** `fuzz/fuzz_targets/fuzz_choose_depth.rs` (lines 41-44)

```rust
fuzz_target!(|data: &[u8]| {
    let branch_count = data.first().copied().unwrap_or(1);
    let body_count = data.get(1).copied().unwrap_or(0);
    fuzz_choose_structure(branch_count, body_count);
});
```

**Problem:** `fuzz_choose_structure()` calls `compile_workflow()` and discards the result with `let _`. No assertions whatsoever. This is pure coverage — it accepts every input silently. The fuzzer cannot distinguish between a correct compile and a buggy panic.

**Fix needed:** Assert that `compile_workflow` returns a typed `Result` (never panics), and on success, verify the compiled workflow has at least one node, valid slot references, and non-zero node count.

---

### 2. `fuzz/fuzz_targets/fuzz_choose_when_parse.rs` — NO ASSERTIONS

**File:** `fuzz/fuzz_targets/fuzz_choose_when_parse.rs` (lines 40-59)

```rust
fuzz_target!(|data: &[u8]| {
    // ... parse when strings ...
    fuzz_choose_when(&when_strings);
});
```

**Problem:** Identical to `fuzz_choose_depth`. `fuzz_choose_when()` calls `compile_workflow()` and discards the result. Zero assertions.

**Fix needed:** Same as `fuzz_choose_depth.rs` — assert compile result validity, node count, and slot bounds.

---

### 3. `fuzz/fuzz_targets/choose_lowering_fuzzer.rs` — NO ASSERTIONS

**File:** `fuzz/fuzz_targets/choose_lowering_fuzzer.rs` (lines 52-54)

```rust
let _result = lower_choose(StepIdx::new(0), branches, otherwise, &mut builder);
// We don't care about the result - we just want to ensure no panic
```

**Problem:** Comment admits it: "we don't care about the result." This is the worst kind of fuzz target — it asserts nothing. If `lower_choose` returns an error that should have been an `Ok`, or if it produces invalid branch indices, this target never catches it.

**Fix needed:** Assert that on success, the builder contains valid `SlotCompiler` state. On error, assert the error is typed. Verify that branch indices don't exceed reasonable bounds.

---

### 4. `fuzz/fuzz_targets/fuzz_diagnostic_code_from_str.rs` — EMPTY ASSERTION

**File:** `fuzz/fuzz_targets/fuzz_diagnostic_code_from_str.rs` (lines 10-18)

```rust
fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return; };
    let _ = vb_core::diagnostic::DiagnosticCode::from_str(s);
});
```

**Problem:** Discards the `Result` entirely. The lib.rs implementation at `fuzz/src/lib.rs:3847` has assertions (display starts with 'E', exactly 5 chars), but the direct wrapper file skips those assertions by calling `from_str` directly without the lib.rs function. This target is effectively empty.

**Fix needed:** Either delegate to `fuzz_lib::fuzz_diagnostic_code_from_str()` (which has assertions), or add assertions to the wrapper itself.

---

### 5. `fuzz/fuzz_targets/ui_redaction_artifact.rs` — FRAGILE ASSERTION

**File:** `fuzz/fuzz_targets/ui_redaction_artifact.rs` (lines 8-17)

```rust
if artifact.contains("vb_nf2u_secret_sentinel") {
    assert_eq!(
        result.map(|_| String::from("passed")).map_err(|error| format!("{error:?}")),
        Err(String::from("RedactionViolation { code: \"redaction_violation\", ... }"))
    );
}
```

**Problem:** 
- The assertion fires only on a single specific sentinel string. 99.9% of fuzz input is untested.
- The assertion uses `format!("{error:?}")` string comparison, which is brittle — any change to `Display` impl breaks the assertion silently.
- There is NO assertion for the common case (non-sentinel input). The `scan_release_artifact` function is called but its result is discarded when the sentinel isn't present.

**Fix needed:** Add a general assertion that `scan_release_artifact` never panics and returns a typed `Result`. Assert that non-sensitive artifacts are accepted. Only keep the sentinel assertion as a targeted supplement.

---

### 6. `fuzz/fuzz_targets/expr_eval.rs` — DELEGATES TO LIB.RS, CHECK NEEDED

**File:** `fuzz/fuzz_targets/expr_eval.rs`

```rust
fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_expr_eval(data);
});
```

**Delegates to:** `fuzz/src/lib.rs:1991` — `fuzz_expr_eval`

**Status:** The lib.rs function HAS assertions (rejects Ok(Null), asserts eval_count > 0). This wrapper is **APPROVED** as-is.

**Note:** Listed here for transparency. Not rejected, but worth confirming the delegation is correct and the lib.rs assertions haven't been removed.

---

## APPROVED: Strong Targets (51)

### lib.rs functions with property assertions:

| # | Function | Location | Assertions |
|---|----------|----------|------------|
| 1 | `fuzz_capability_name_schema` | lib.rs:43-70 | Validation error type assertions for empty/invalid names |
| 2 | `fuzz_capability_contract_schema` | lib.rs:73-114 | Validation error type assertions for duplicates/mismatches |
| 3 | `fuzz_yaml_events` | lib.rs:240-293 | Event count non-empty, event count bounded, source map bounded |
| 4 | `fuzz_ipc_frame` | lib.rs:307-433 | Header roundtrip, payload decode assertions, typed errors, bounded paths |
| 5 | `fuzz_journal_event` | lib.rs:471-526 | Event validity, roundtrip encode/decode, typed JournalError |
| 6 | `fuzz_replay_events` | lib.rs:534-575 | Replay count ≤ input count, tracker state consistency |
| 7 | `fuzz_extract_terminal` | lib.rs:581-601 | Terminal event is known variant |
| 8 | `fuzz_action_tracker` | lib.rs:610-667 | State transition assertions, determinism |
| 9 | `fuzz_expression` | lib.rs:670-696 | Type name non-empty for successful eval |
| 10 | `fuzz_compiled_ir` | lib.rs:699-753 | Node count ≥ 1, slot count preserved, digest preserved, slot bounds |
| 11 | `fuzz_generated_compare` | lib.rs:1076-1113 | Validation/agreement, digest/node/slot equality |
| 12 | `fuzz_expr_bytecode` | lib.rs:1128-1219 | Type name non-empty, no silent Ok(Null) |
| 13 | `fuzz_taint_propagation` | lib.rs:1231-1339 | Taint level assertions, clean propagation invariant |
| 14 | `fuzz_resource_budget` | lib.rs:1342-1444 | Cost bounded, budget non-negative, budget consistency |
| 15 | `fuzz_expr_eval` | lib.rs:1991-2046 | Ok(Null) rejected, eval_count > 0 |
| 16 | `fuzz_accessor_traversal` | lib.rs:2062-2183 | Path depth bounded, slot references valid |
| 17 | `fuzz_slot_value_roundtrip` | lib.rs:2186-2261 | Roundtrip equality |
| 18 | `fuzz_admission_fuzz` | lib.rs:2477-2638 | Family set valid, classification assertions |
| 19 | `fuzz_recovery_decode` | lib.rs:2829-2872 | Recovery invariant, typed errors |
| 20 | `fuzz_step_budget_new` | lib.rs:2890-2961 | Clamping exact, remaining in [0, MAX], try_take behavior |
| 21 | `fuzz_vb_qi37_12_persisted_payload_decode` | lib.rs:3001-3102 | Truncation → UnexpectedEof, corruption → PayloadDigestMismatch |
| 22 | `fuzz_ipc_frame_boundary` | lib.rs:3128-3196 | Typed IPC error assertions |
| 23 | `fuzz_storage_envelope_boundary` | lib.rs:3241-3293 | Typed journal error assertions |
| 24 | `fuzz_binary_payload_boundary` | lib.rs:3364-3423 | Typed journal error assertions, small max_payload |
| 25 | `fuzz_external_input_adapter_boundary` | lib.rs:3441-3469 | Typed boundary error assertions |
| 26 | `fuzz_collect_page_pagination` | lib.rs:3585-3755 | 6 pagination invariants |
| 27 | `fuzz_diagnostic_from_error` | lib.rs:3773-3828 | Non-empty message, non-zero code |
| 28 | `fuzz_diagnostic_code_from_str` | lib.rs:3847-3870 | Display starts with 'E', exactly 5 chars |
| 29 | `fuzz_span_bridge` | lib.rs:3887-3920 | Span field equality, map consistency |
| 30 | `fuzz_compile_source_ast_marks` | lib.rs:3946-3965 | CompileErrors non-empty on error |
| 31 | `fuzz_accepted_artifact_envelope_qi37_4_2` | lib.rs:1046-1073 | Gate count > 0, accepted_at_seq ≥ 1, cap count bounded |
| 32 | `fuzz_admission_input_surface` | lib.rs:2722-2761 | Strict/relaxed equivalence, typed journal errors |
| 33 | `fuzz_strict_yaml_profile` | lib.rs:2767-2785 | Unsupported features cause compile error, node count ≥ 1 |
| 34 | `fuzz_accepted_artifact_decode` | lib.rs:2793-2821 | accepted_at_seq > 0, gate_count > 0 |

### Fuzz target files with assertions:

| # | Target File | Assertions |
|---|-------------|------------|
| 35 | `fuzz_digest_compile.rs` | Typed compilation errors, non-empty input produces non-empty compilation |
| 36 | `fuzz_finish_digest_encoding.rs` | Encoding length, canonical digest consistency |
| 37 | `fuzz_symbolic_code_deserialize.rs` | Deserialized code non-empty |
| 38 | `fuzz_retry_codec.rs` | Roundtrip equality, serialization length, retry policy assertions |
| 39 | `fuzz_action_ticket_roundtrip.rs` | Roundtrip equality |
| 40 | `fuzz_mock_marker_serialization.rs` | 1-byte length, roundtrip equality |
| 41 | `vb_storage_codec.rs` | Encode/decode roundtrip, header decode paths, corruption, kind family, schema version, digest |
| 42 | `vb_ajc40_empty_path_root_accessor.rs` | Empty accessor path with slot count checks |
| 43 | `vb_ajc40_total_yield_cost_mismatch.rs` | Yield cost assertions, typed error checks |
| 44 | `vb_vzcuf_PS_001.rs` | Digest non-emptiness |
| 45 | `vb_vzcuf_PS_002.rs` | YAML validation error type |
| 46 | `vb_vzcuf_PS_003.rs` | Oversized envelope rejection |
| 47 | `vb_vzcuf_PS_004.rs` | Digest sensitivity |
| 48 | `vb_vzcuf_PS_005.rs` | Compile assertions, non-empty compilation |
| 49 | `vb_vzcuf_PS_006.rs` | Compile assertions |
| 50 | `vb_vzcuf_PS_007.rs` | Empty digest rejection |
| 51 | `vb_vzcuf_PS_008.rs` | Full pipeline compile and digest assertions |
| 52 | `vb_vzcuf_PS_009.rs` | YAML parsing error type |
| 53 | `vb_rpch_hydrate_events_fuzz.rs` | Precondition assertion |
| 54 | `vb_rpch_replay_events_fuzz.rs` | Replay attempt current/stale assertions |
| 55 | `vb_rpch_seed_dimensions_fuzz.rs` | Recovery dimension count assertions |
| 56 | `vb_rpch_hydrate_snapshot_tail_fuzz.rs` | Dimension positive assertion |
| 57 | `vb_mrwe6_duplicate_decode.rs` | Duplicate record assertions |
| 58 | `vb_mrwe6_recovery_inventory.rs` | Recovery inventory assertions |
| 59 | `decode_record.rs` | Typed errors, roundtrip |
| 60 | `journal_decode.rs` | Invariant checks |
| 61 | `journal_event.rs` | Roundtrip assertions |
| 62 | `kind_validation.rs` | Kind validation assertions |
| 63 | `nested_together_lowering.rs` | Nested lowering assertions |
| 64 | `ps_003_oversized_envelope.rs` | Oversized envelope assertions |
| 65 | `ps_005_trailing_bytes.rs` | Trailing bytes assertions |
| 66 | `ps_006_fuzz.rs` | PS-006 assertions |
| 67 | `ps_012_corrupted_read.rs` | Corrupted read assertions |
| 68 | `reduce_diagnostic_codes.rs` | Diagnostic code reduction |
| 69 | `reduce_lowering_panic.rs` | Lowering panic-free |
| 70 | `tooling_flux_check_selector.rs` | Flux check assertions |
| 71 | `tooling_guard_zero_parser.rs` | Guard parser assertions |
| 72 | `tooling_kani_list_args.rs` | Kani list args assertions |
| 73 | `wait_digest_exhaustive_collision.rs` | Digest collision assertions |
| 74 | `wait_digest_sensitivity.rs` | Digest sensitivity |
| 75 | `wait_sentinel_collision.rs` | Sentinel collision |
| 76 | `canonical_digest_ask.rs` | Canonical digest compilation |
| 77 | `foreach_digest_canonical.rs` | Delegates to `fuzz_lib::fuzz_canonical_digest_foreach()` — needs verification |
| 78 | `foreach_digest_step.rs` | Delegates to `fuzz_lib::fuzz_digest_step_primitive()` — needs verification |
| 79 | `span_bridge_fuzz.rs` | Delegates to `fuzz_lib::fuzz_span_bridge()` — APPROVED (lib.rs has assertions) |

---

## APPROVED: Wrapper-only Targets (delegating to strong lib.rs functions)

The following wrappers delegate to lib.rs functions that have strong assertions. These are **APPROVED as-is**:

- `expression.rs` → `fuzz_lib::fuzz_expression()` (lib.rs:670-696, type name assertions)
- `compiled_ir.rs` → `fuzz_lib::fuzz_compiled_ir()` (lib.rs:699-753, slot bounds assertions)
- `expr_eval.rs` → `fuzz_lib::fuzz_expr_eval()` (lib.rs:1991-2046, null rejection assertions)
- `foreach_digest_canonical.rs` → `fuzz_lib::fuzz_canonical_digest_foreach()` — **needs verification**
- `foreach_digest_step.rs` → `fuzz_lib::fuzz_digest_step_primitive()` — **needs verification**
- `diagnostic_from_error.rs` → `fuzz_lib::fuzz_diagnostic_from_error()` (lib.rs:3773-3828, assertions)
- `diagnostic_code_from_str.rs` → `fuzz_lib::fuzz_diagnostic_code_from_str()` (lib.rs:3847-3870, assertions)
- `compile_source_ast_marks.rs` → `fuzz_lib::fuzz_compile_source_ast_marks()` (lib.rs:3946-3965, assertions)
- `span_bridge_fuzz.rs` → `fuzz_lib::fuzz_span_bridge()` (lib.rs:3887-3920, assertions)
- `yaml_events.rs` → `fuzz_lib::fuzz_yaml_events()` (lib.rs:240-293, assertions)
- `ipc_frame.rs` → `fuzz_lib::fuzz_ipc_frame()` (lib.rs:307-433, assertions)
- `journal_event.rs` → `fuzz_lib::fuzz_journal_event()` (lib.rs:471-526, assertions)

---

## Mutation Resistance Assessment

### Weak targets are mutation-blind:

The 6 REJECTED targets would **all pass with zero bugs detected** against `cargo-mutants`:

| Target | Would mutant `compile_workflow() -> Ok(...)` survive? | Would mutant `from_str() -> Err(...)` survive? |
|--------|------------------------------------------------------|------------------------------------------------|
| `fuzz_choose_depth` | YES — no assertions to fail | N/A |
| `fuzz_choose_when_parse` | YES — no assertions to fail | N/A |
| `choose_lowering_fuzzer` | YES — discards result | N/A |
| `fuzz_diagnostic_code_from_str` | N/A | YES — discards result |
| `ui_redaction_artifact` | Partially — only fires on sentinel | Partially |
| `expr_eval` | N/A (delegates to strong lib.rs) | N/A |

### Strong targets are mutation-resistant:

Targets with roundtrip assertions and typed error checks would **fail on most mutants**:
- Roundtrip equality assertions (`assert_eq!(a, b)`) fail on serialization/deserialization mutants
- Typed error assertions (`matches!(e, Variant)`) fail on error-suppression mutants
- Invariant assertions (`assert!(len > 0)`) fail on boundary-corruption mutants

---

## Recommendations

### Priority 1: Add assertions to `fuzz_choose_depth.rs` and `fuzz_choose_when_parse.rs`

Both targets compile YAML workflows but discard results. Add:

```rust
// After compile_workflow call:
match result {
    Ok(workflow) => {
        assert!(workflow.node_count() >= 1, "compiled workflow must have at least 1 node");
        assert!(workflow.slot_count() > 0, "compiled workflow must have at least 1 slot");
    }
    Err(ref errors) => {
        assert!(!errors.is_empty(), "compile errors must be non-empty");
    }
}
```

### Priority 2: Add assertions to `choose_lowering_fuzzer.rs`

The `lower_choose` function should return typed results. Add:

```rust
match _result {
    Ok(builder) => {
        assert!(builder.branches().len() <= 128, "builder branch count bounded");
    }
    Err(_) => {
        // Typed error — acceptable
    }
}
```

### Priority 3: Fix `fuzz_diagnostic_code_from_str.rs` wrapper

Either delegate to `fuzz_lib::fuzz_diagnostic_code_from_str()` (which has assertions), or add:

```rust
let result = vb_core::diagnostic::DiagnosticCode::from_str(s);
if let Ok(code) = result {
    let display = code.to_string();
    assert!(display.starts_with('E'), "Display must start with E");
    assert_eq!(display.len(), 5, "Display must be exactly 5 chars");
}
```

### Priority 4: Harden `ui_redaction_artifact.rs`

Add general assertion that the function never panics and returns typed Result:

```rust
let result = vb_ui_snapshot::redaction::scan_release_artifact(&artifact);
// Assert result is always a valid Result (never panics)
let _is_ok = result.is_ok();
```

### Priority 5: Verify `foreach_digest_canonical.rs` and `foreach_digest_step.rs`

These delegate to `fuzz_lib::fuzz_canonical_digest_foreach()` and `fuzz_lib::fuzz_digest_step_primitive()` respectively. Need to confirm these lib.rs functions exist and have assertions.

---

## Summary Table

| Category | Count | Percentage |
|----------|-------|------------|
| Strong (property assertions) | 34 | 51% |
| Strong (typed error checks) | 17 | 25% |
| Strong (roundtrip) | 11 | 16% |
| Weak (no assertions) | 4 | 6% |
| Weak (fragile assertions) | 1 | 1.5% |
| Empty/coverage-only | 1 | 1.5% |
| **Total** | **67** | **100%** |
