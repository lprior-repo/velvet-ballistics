# Test Plan Review: vb-qi37.13.3

## Reviewer: test-reviewer (Mode 2: Suite Inquisition)
## State: 8 → 9

---

## 1. Test Plan Adequacy

| Criterion | Assessment | Evidence |
|-----------|------------|----------|
| Behavior coverage | 38/38 behaviors mapped | test-plan.md §1 — all behaviors have test functions |
| Error variant completeness | 15/15 EmitterError variants | test-plan.md §9 — exact `matches!` for all variants |
| Boundary coverage | u64 boundary, max_payload_len boundary, header byte offsets | Combinatorial Matrix §8 |
| BDD scenario coverage | All Given/When/Then scenarios written | test-plan.md §3 — 18 BDD scenarios |
| Proptest invariants | 5 invariants (PROP-EMIT-001/002/003/004/006/007) | test-plan.md §4 |
| Fuzz targets | 3 targets (encode_postcard, validate_no_ansi, encode_yaml) | test-plan.md §5 |
| Kani harnesses | 9 (not integrated — FINDING-KANI from proof-review) | test-plan.md §6 |

**VERDICT: ADEQUATE.** The test plan comprehensively covers all emitter behaviors with appropriate layered testing (unit, integration, proptest, fuzz).

---

## 2. Test Suite Inquisition (26 Tests)

### 2.1 Test Execution Results

```
cargo test -p vb_ui_model --test emitter_missing_tests
RESULT: 24 passed, 2 failed
```

### 2.2 Passing Tests (24) — Validated Correct

| Test | Behavior | Evidence |
|------|----------|----------|
| `encode_yaml_handles_string_values` | String JSON values | PASS |
| `encode_yaml_handles_nested_strings` | Nested string objects | PASS |
| `encode_yaml_handles_f64_as_string_representation` | f64 as string | PASS |
| `encode_yaml_handles_f64_special_values` | f64 Infinity/-Infinity | PASS |
| `encode_yaml_handles_u64_within_i64_max` | u64 ≤ i64::MAX | PASS |
| `encode_yaml_returns_error_for_invalid_json` | Non-serializable | PASS |
| `encode_yaml_complex_nested_structure` | Deeply nested JSON | PASS |
| `encode_yaml_handles_unicode_strings` | Unicode strings | PASS |
| `decode_postcard_rejects_header_length_mismatch` | HeaderLengthMismatch | PASS |
| `decode_postcard_rejects_truncated_payload` | UnexpectedEof mid-read | PASS |
| `encode_postcard_rejects_payload_one_byte_over` | PayloadTooLarge boundary | PASS |
| `encode_postcard_with_max_payload_len_exactly` | Exact max boundary | PASS |
| `encode_postcard_zero_payload` | Empty payload roundtrip | PASS |
| `validate_no_ansi_detects_just_esc_byte` | ESC 0x1B alone | PASS |
| `validate_no_ansi_detects_ansi_in_middle_of_text` | ANSI in middle | PASS |
| `validate_no_ansi_detects_ansi_at_start` | ANSI at start | PASS |
| `validate_no_ansi_allows_multibyte_utf8_without_ansi` | UTF-8 without ANSI | PASS |
| `yaml_envelope_from_envelope_all_kinds_have_correct_name` | All 6 EnvelopeKind names | PASS |
| `yaml_envelope_preserves_schema_version_command_exit_code` | Field preservation | PASS |
| `cli_constants_are_correct` | All CLI constants verified | PASS |
| `cli_magic_bytes_are_vbli` | VBLI magic bytes | PASS |
| `emitter_error_payload_too_large_display` | Error Display impl | PASS |
| `emitter_error_bad_magic_display` | Error Display impl | PASS |
| `emitter_error_migration_required_display` | Error Display impl | PASS |

### 2.3 Failing Tests (2) — NOT Test Design Flaws

| Test | Expected Behavior | Actual Behavior | Root Cause |
|------|-----------------|-----------------|------------|
| `encode_yaml_returns_error_for_u64_exceeding_i64_max` | `Err(YamlEncodeFailed)` | `Ok("---...\n9223372036854775807")` | Production bug at emitter.rs:199 |
| `encode_yaml_json_value_to_yaml_u64_overflow` | `Err(YamlEncodeFailed)` | `Ok("---\n9223372036854775807")` | Same production bug |

**Finding: REAL BEHAVIOR BUG — u64→i64 silent truncation**

Production code at `emitter.rs:198-200`:
```rust
} else if let Some(u) = n.as_u64() {
    let val = i64::try_from(u).unwrap_or(i64::MAX);
    Ok(Yaml::Value(Scalar::Integer(val)))
```

**Contract violation:** BDD scenario in test-plan.md §3 (Behavior 10) explicitly states:
> "Given: A serde_json::Number from a u64 > i64::MAX / When: json_value_to_yaml processes the number / Then: Returns Err(EmitterError::YamlEncodeFailed), NOT silent truncation to i64::MAX"

The test correctly identifies the bug. The bug is a **data corruption** issue: a u64 value of `(i64::MAX as u64) + 1` (i.e., 9223372036854775808) is silently encoded as i64::MAX (9223372036854775807), producing incorrect YAML output without any error signal.

---

## 3. Bug Disposition

**Decision: Fix the production bug, not the tests.**

| Option | Action | Verdict |
|--------|--------|---------|
| Document as known issue | Leave bug in place, mark as non-blocking | REJECTED — silent truncation is a data corruption defect, not an acceptable known issue |
| Fix production code | Change emitter.rs:199 to return `Err(EmitterError::YamlEncodeFailed)` | REQUIRED |

**Required fix at `emitter.rs:199`:**
```rust
// CURRENT (buggy):
let val = i64::try_from(u).unwrap_or(i64::MAX);
Ok(Yaml::Value(Scalar::Integer(val)))

// REQUIRED:
i64::try_from(u).map(Yaml::ValueScalar::Integer)
    .map_err(|_| EmitterError::YamlEncodeFailed)?
```

---

## 4. Pre-Existing Suite Status

| Suite | Tests | Result |
|-------|-------|--------|
| `cargo test -p vb_ui_model --lib` (emitter.rs) | 41 | PASS |
| `cargo test -p vb_ui_model --test emitter_proptest` | 24 | PASS |
| `cargo clippy -p vb_ui_model -- -D warnings` | — | PASS |

---

## 5. Advancement Gate

| Gate | Status | Evidence |
|------|--------|----------|
| Test plan adequate | ✅ | 38 behaviors, 15 error variants, all layers |
| Tests execute | ✅ | 24 pass, 2 fail (bug detected, not test flaw) |
| Failing tests are real bugs | ✅ | emitter.rs:199 silent truncation confirmed |
| Bug fix is straightforward | ✅ | Single `map_err` replaces `unwrap_or` |
| Pre-existing suites clean | ✅ | 41 + 24 tests pass |

**VERDICT: ADVANCE TO STATE 9 (test-reviewer) — CONDITIONAL ON BUG FIX**

The 2 failing tests are correctly written and correctly identify a real production bug. The bug must be fixed before this bead can advance to landing-skill. The fix is a single-line `map_err` replacement in `emitter.rs:199`.

---

*Mode 2 evidence complete.*
