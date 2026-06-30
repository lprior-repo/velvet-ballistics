# Test Plan — vb-xi2f.33: Digest Covers Ask Semantics

**Agent**: `test-planner`
**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**State**: 8 (test-planner)
**Date**: 2026-05-25
**Inputs**: `contract.md`, `domain-model.md`, `error-taxonomy.md`, `workflow-model.md`, `type-contracts.md`, `boundary-map.md`, `hazard-analysis.md`, `traceability-matrix.jsonl`, `proof-obligations.planned.jsonl`, `proof-to-rust-map.md`, `proof-coverage-matrix.md`, production source (`part_05.rs`, `compile/mod.rs`)

## Summary

- **Behaviors identified**: 19
- **Trophy allocation**: 10 unit / 5 integration / 2 e2e / 2 static
- **Proptest invariants**: 4 (already materialized, PASSING → NO NEW)
- **Fuzz targets**: 1 (already materialized, compiles → NO NEW)
- **Kani harnesses**: 6 (already materialized, blocked by blake3 asm → NO NEW)
- **Mutation kill threshold**: ≥90% new code paths

### Pre-existing Coverage from Earlier States

| Layer | Artifact | Status | Count |
|-------|----------|--------|-------|
| Proptest (L2) | 4 test suites | ✅ PASS (58 property confirmations) | 4 files |
| Kani (L3) | 6 harness modules | ⚠️ materialized, blocked by blake3 inline asm | 6 files |
| Fuzz (L2) | 1 fuzz target | ✅ compiles, not executed | 1 file |
| Unit (PO-UT-003) | Inline parity tests in `compile/mod.rs` | ✅ materialized | 4 tests |
| Existing | `vb_compile` lib tests | ✅ 245 passing | 245 tests |

**This test plan adds NEW behavior tests (unit + integration + e2e) for the gaps not covered by proptest/Kani/fuzz.** The proptest suites cover stochastic property verification; this plan covers deterministic behavior specification and regression protection.

---

## 1. Behavior Inventory

Each behavior is a guarantee `canonical_digest` or `digest_step_primitive` makes about its response to inputs while in a particular state.

### Core Digest Behaviors

| # | Behavior Description | Contract Clause |
|---|---------------------|-----------------|
| B1 | `canonical_digest` produces semantically distinct digests when an Ask prompt changes | INV-ASK-001, POST-001 |
| B2 | `canonical_digest` produces semantically distinct digests when an Ask timeout changes | INV-ASK-002, POST-002 |
| B3 | `canonical_digest` is deterministic — same source always produces same `WorkflowDigest` | INV-ASK-003, POST-003 |
| B4 | An Ask with empty prompt `""` produces a well-defined digest distinct from any non-empty prompt | INV-ASK-004, POST-004 |
| B5 | `timeout: None` and `timeout: Some("")` produce distinct digest contributions | INV-ASK-005, POST-005 |
| B6 | The active canonical path (`part_05.rs`) and legacy path (`compile/mod.rs`) produce identical digests for identical sources | INV-ASK-006, POST-006 |
| B7 | Existing Set/Finish digest behavior is unchanged after the Ask fix | INV-ASK-007, POST-007 |

### Digest Step Primitive Behaviors

| # | Behavior Description | Contract Clause |
|---|---------------------|-----------------|
| B8 | `digest_step_primitive` has an explicit `Ask { prompt, timeout }` match arm — not relying on catch-all `other` arm | TC-001 |
| B9 | Ask field hashing order is deterministic: tag (`b"ask"`) → prompt → timeout (or sentinel) | TC-002, WF-INV-003 |
| B10 | Empty prompt `""` is valid hash input — `hasher.update(b"")` does not panic or produce degenerate output | TC-003 |
| B11 | `timeout: None` uses sentinel `b"no_timeout"`; `timeout: Some(t)` uses `b"timeout"` + `t.as_bytes()` — distinct contributions | TC-004 |
| B12 | `digest_step_primitive` does not panic, unwrap, or expect on any valid `StepPrimitive` variant | TC-007 |
| B13 | The `Finish` match arm continues to hash both `ScalarValue::String` and `ScalarValue::Integer` results correctly | TC-005 |
| B14 | The `Set { output, value }` arm continues to hash both fields correctly | TC-005 |

### Structural/Identity Behaviors

| # | Behavior Description | Contract Clause |
|---|---------------------|-----------------|
| B15 | `canonical_digest` contribution includes version, name, trigger type, trigger fields, step IDs, and primitive fields | POST-006 semantics |
| B16 | `canonical_digest` on a source with no steps produces a well-defined (non-panicking) digest | WF-INV-001 |
| B17 | Changing the `trigger` field changes the digest (e.g., Manual vs Schedule) | WF-INV-004 |
| B18 | Changing `version` or `name` changes the digest | WF-INV-004 |
| B19 | Adding/removing/reordering steps changes the digest | WF-INV-002 |

---

## 2. Trophy Allocation

### Rationale

`canonical_digest` and `digest_step_primitive` are **pure functions** in the Calc layer — no I/O, no async, no randomness, no network. This makes them highly amenable to unit testing. However, the digest is *embedded* in the compilation pipeline (`part_01.rs` calls `canonical_digest` at line 46) and surfaces in `CompiledWorkflow::digest()` — integration tests are required to verify the digest flows correctly through the pipeline.

Since proptest (4 suites) and Kani (6 harnesses) already provide extensive property-based and formal coverage at the property/invariant layer, the **new behavior tests focus on the specification layer**: deterministic scenarios that prove each contract clause with exact inputs and expected outcomes. These serve as living documentation of the digest contract and regression protection for future changes.

### Target Allocation (NEW tests only)

```
         [E2E]           2   ← full pipeline: YAML string → compile → verify digest
    [Integration]        5   ← canonical_digest in compilation pipeline, embedded digest
    [Unit / Calc]       10   ← pure logic: exhaustive per-behavior specification
  [Static Analysis]      2   ← code-review tests (explicit arm, no unwrap)
```

| Layer | Count | Ratio | Justification |
|-------|-------|-------|---------------|
| Unit | 10 | ~53% | Pure functions merit exhaustive deterministic coverage. Each behavior (B1-B14) gets a dedicated test with exact input and expected output. |
| Integration | 5 | ~26% | Digest is embedded via the compilation pipeline; integration tests verify end-to-end correctness of the digest through `WorkflowParts` → `CompiledWorkflow`. |
| E2E | 2 | ~10% | Full YAML-string-to-compiled-workflow verification. Narrow top of trophy — these are slowest but cover the actual user-facing contract. |
| Static | 2 | ~11% | Code-review assertions (no catch-all for Ask, no unwrap/expect in match arms). Verified via `grep`/source-inspection assertions. |

**Deviation justification**: The 60/30/5/5 trophy ratio is adjusted to 53/26/10/11 because:
1. The existing proptest/Kani/fuzz layers already provide ~15% of coverage at the property/formal level
2. This system is a pure function — the "integration" boundary is the compiler pipeline, not external I/O
3. Static assertions (explicit arm, no unwrap) are critical because the original bug was a catch-all arm silently swallowing semantic fields

---

## 3. BDD Scenarios

Every behavior from Section 1 is specified as a BDD scenario. Each scenario name is a valid Rust test function name following the `subject_outcome_when_condition` convention.

### Behavior B1: Prompt Sensitivity

```
### Behavior: canonical_digest produces distinct digests when Ask prompt changes
Given: Two WorkflowSource values A and B, identical except A has Ask{prompt="hello"}, B has Ask{prompt="world"}
When: canonical_digest(A) and canonical_digest(B) are computed
Then: digest(A) != digest(B)

Error variant: N/A (infallible function)
```

**Test function**: `fn canonical_digest_produces_distinct_digests_when_ask_prompt_differs()`

| Scenario | Input A prompt | Input B prompt | Expected |
|----------|---------------|---------------|----------|
| Different prompts, simple | `"hello"` | `"world"` | `digest_a != digest_b` |
| Different prompts, one empty | `""` | `"hello"` | `digest_a != digest_b` |
| Same prompt | `"same"` | `"same"` | `digest_a == digest_b` (same input) |
| Prompt with special chars | `"hello\nworld"` | `"hello\tworld"` | `digest_a != digest_b` |
| Prompt with Unicode | `"héllo"` | `"hëllo"` | `digest_a != digest_b` |
| Long prompts (>1KB) | `"a".repeat(2048)` | `"b".repeat(2048)` | `digest_a != digest_b` |

### Behavior B2: Timeout Sensitivity

```
### Behavior: canonical_digest produces distinct digests when Ask timeout changes
Given: Two WorkflowSource values A and B, identical except A has Ask{timeout=None}, B has Ask{timeout=Some("30s")}
When: canonical_digest(A) and canonical_digest(B) are computed
Then: digest(A) != digest(B)
```

**Test function**: `fn canonical_digest_produces_distinct_digests_when_ask_timeout_differs()`

| Scenario | Input A timeout | Input B timeout | Expected |
|----------|----------------|-----------------|----------|
| None vs Some(value) | `None` | `Some("30s")` | `digest_a != digest_b` |
| Two different Some values | `Some("10s")` | `Some("30s")` | `digest_a != digest_b` |
| None vs Some(empty) | `None` | `Some("")` | `digest_a != digest_b` |
| Same timeout value | `Some("30s")` | `Some("30s")` | `digest_a == digest_b` |
| Timeout with special chars | `Some("10s")` | `Some("10\ns")` | `digest_a != digest_b` |

### Behavior B3: Determinism

```
### Behavior: canonical_digest is deterministic — same source always produces same digest
Given: A WorkflowSource S with Ask step
When: canonical_digest(S) is called three times in succession
Then: All three results are identical
```

**Test function**: `fn canonical_digest_is_deterministic_when_called_multiple_times()`

| Scenario | Source | Expected |
|----------|--------|----------|
| Ask with prompt + Some timeout | Fixed source | All 3 calls produce same `[u8; 32]` bytes |
| Ask with empty prompt + None timeout | Fixed source | All 3 calls produce same digest |
| Source with multiple Ask steps | Fixed source | All 3 calls produce same digest |

### Behavior B4: Empty Prompt Edge Case

```
### Behavior: Ask with empty prompt produces well-defined, distinct digest
Given: Source A with Ask{prompt=""}, Source B with Ask{prompt="hello"}, Source C with Ask{prompt="x"}
When: canonical_digest is computed for A, B, and C
Then: digest(A) != digest(B), digest(A) != digest(C), digest(A) is a valid 32-byte hash
```

**Test function**: `fn canonical_digest_produces_distinct_digest_when_ask_prompt_is_empty()`

| Scenario | Input | Expected |
|----------|-------|----------|
| Empty vs non-empty prompt | `""` vs `"hello"` | `digest_a != digest_b` |
| Empty prompt produces valid hash | `""` | `digest.as_bytes()` is `[u8; 32]`, not all zeros |
| Two sources both with empty prompt | `""` (identical) | `digest_a == digest_b` |

### Behavior B5: None vs Some("") Timeout Distinction

```
### Behavior: timeout None and timeout Some("") produce distinct digest contributions
Given: Source A with Ask{timeout=None}, Source B with Ask{timeout=Some("")}, identical prompts
When: canonical_digest(A) and canonical_digest(B) are computed
Then: digest(A) != digest(B)
```

**Test function**: `fn canonical_digest_produces_distinct_digests_when_timeout_none_vs_some_empty()`

| Scenario | Timeout A | Timeout B | Expected |
|----------|-----------|-----------|----------|
| None vs Some("") | `None` | `Some("")` | `digest_a != digest_b` |
| None vs Some("30s") | `None` | `Some("30s")` | `digest_a != digest_b` |
| Some("") vs Some("30s") | `Some("")` | `Some("30s")` | `digest_a != digest_b` |

### Behavior B6: Duplicate Implementation Parity

```
### Behavior: Active path and legacy path canonical_digest produce identical digests
Given: A WorkflowSource S with Ask step (prompt + optional timeout)
When: part_05::canonical_digest(S) and compile::canonical_digest(S) are computed
Then: Both digests are identical
```

**Test function**: `fn canonical_digest_paths_produce_identical_digests_when_source_has_ask()`

| Scenario | Source | Expected |
|----------|--------|----------|
| Ask with Some timeout | `ask_source("p", Some("30s"))` | `part_05 == compile/mod` |
| Ask with None timeout | `ask_source("p", None)` | `part_05 == compile/mod` |
| Ask with empty prompt | `ask_source("", None)` | `part_05 == compile/mod` |
| Set + Finish only | `set_finish_source()` | `part_05 == compile/mod` |

**Note**: PO-UT-003 (REPAIR-2) already materialized 4 inline parity tests in `compile/mod.rs`. These should be extracted to a dedicated integration test file OR verified to be runnable. The compile/mod.rs module is NOT mounted as a crate module (no `mod compile;` in lib.rs), making the inline tests unreachable. **This is a critical test execution gap** — the parity tests must be moved to a test target that actually compiles and runs.

### Behavior B7: Set/Finish Regression

```
### Behavior: Set and Finish digest contributions are unchanged after Ask fix
Given: A WorkflowSource with only Set and Finish steps (no Ask)
When: canonical_digest is computed
Then: The digest is well-defined, deterministic, and the Set/Finish arms produce their expected byte contributions
```

**Test function**: `fn canonical_digest_produces_unchanged_digests_for_set_and_finish_primitives()`

| Scenario | Steps | Expected |
|----------|-------|----------|
| Set only | `Set{output:"x", value:"1"}` | Well-defined digest, no panic |
| Finish(String) only | `Finish{result:String("done")}` | Well-defined digest, no panic |
| Finish(Integer) only | `Finish{result:Integer(0)}` | Well-defined digest, no panic |
| Set + Finish | Both | Digest is deterministic |
| Set → Finish order sensitivity | Reversed order | `digest_a != digest_b` (step order matters) |

### Behavior B8: Explicit Ask Match Arm (Static)

```
### Behavior: digest_step_primitive handles Ask via explicit arm, not catch-all
Given: The source code of digest_step_primitive in part_05.rs
When: Inspecting the match arms for StepPrimitive::Ask
Then: An explicit arm `Ask { prompt, timeout } => { ... }` exists between the Finish arm and the `other` catch-all
And: The catch-all arm `other => { ... }` still exists but does NOT match Ask (it is shadowed by the explicit arm)
```

**Test function**: `fn digest_step_primitive_has_explicit_ask_arm_not_catch_all()` — **static/compile-time verification**

This is a **static analysis test** — it verifies source code structure, not runtime behavior. Two approaches:
1. **Preferred**: Use `#[test]` that calls `digest_step_primitive` with an Ask primitive and verifies it does NOT trigger the catch-all path. Since the catch-all and Ask arm produce different hash states, we can compare digests between an Ask with explicit arm vs a test that mimics what the catch-all would produce.
2. **Fallback**: Document a code-review assertion checked by CI script.

| Scenario | Verification Method | Expected |
|----------|-------------------|----------|
| Ask arm exists | `grep 'Ask { prompt, timeout }' part_05.rs` returns match | Line found |
| Ask arm is before catch-all | `grep -n 'Ask {' < grep -n 'other =>'` | Ask arm at lower line number |
| Ask fed to function produces different hash than catch-all path would | Call `digest_step_primitive` via `canonical_digest`, compare with expected catch-all-only hash | Not equal |

### Behavior B9: Field Ordering Determinism

```
### Behavior: Ask fields are hashed in fixed order — tag then prompt then timeout
Given: An Ask StepPrimitive with prompt="abc" and timeout=Some("xyz")
When: digest_step_primitive is called
Then: The hasher receives bytes in order: b"ask", then b"abc", then b"timeout", then b"xyz"
```

**Test function**: `fn digest_step_primitive_hashes_ask_fields_in_deterministic_order()`

Since `blake3::Hasher` is opaque (cannot inspect intermediate state), field ordering is verified indirectly:

| Scenario | Method | Expected |
|----------|--------|----------|
| Same inputs twice | `canonical_digest(S)` called twice | Same digest (proves no non-determinism from ordering) |
| Different order in source construction (same fields) | Construct source with fields in different declaration order | Same digest (proves field values, not declaration order, matter) |

### Behavior B10: Empty Prompt Input Validity

```
### Behavior: Empty prompt is valid hash input — does not panic or degenerate
Given: An Ask StepPrimitive with prompt=""
When: digest_step_primitive is called with this primitive
Then: The function completes without panic, and canonical_digest produces a valid 32-byte hash
```

**Test function**: `fn digest_step_primitive_accepts_empty_prompt_without_panic()`

| Scenario | Input | Expected |
|----------|-------|----------|
| Empty prompt, no timeout | `prompt=""`, `timeout=None` | No panic, valid `[u8; 32]` digest |
| Empty prompt, empty timeout | `prompt=""`, `timeout=Some("")` | No panic, valid digest |
| Empty prompt, non-empty timeout | `prompt=""`, `timeout=Some("30s")` | No panic, valid digest |

### Behavior B11: Timeout Sentinel Distinction

```
### Behavior: None timeout uses sentinel b"no_timeout", Some uses b"timeout" + value
Given: No code change — this is a specification verification
When: Code review of the Ask match arm
Then: None => hasher.update(b"no_timeout") ... Some(t) => hasher.update(b"timeout"); hasher.update(t.as_bytes())
```

**Test function**: `fn digest_step_primitive_uses_distinct_sentinel_for_none_timeout()`

| Scenario | Setup | Expected |
|----------|-------|----------|
| None vs Some("") produce different digest | Two sources, same prompt, one with `None`, one with `Some("")` | `digest_a != digest_b` |
| Sentinel strings `b"no_timeout"` and `b"timeout"` are distinct | Inspection of source code | Strings differ |

### Behavior B12: Panic Freedom

```
### Behavior: digest_step_primitive never panics, unwraps, or expects on any valid primitive
Given: A valid StepPrimitive::Ask with any prompt string (including empty) and any timeout Option (including None)
When: digest_step_primitive is called
Then: The function completes normally, no panic, no unwrap, no expect
```

**Test function**: `fn digest_step_primitive_does_not_panic_for_valid_ask_variants()`

| Scenario | Input | Expected |
|----------|-------|----------|
| Ask with normal prompt + None timeout | Standard input | No panic |
| Ask with empty prompt + None timeout | Edge case | No panic |
| Ask with 10KB prompt | Large input | No panic |
| Ask with normal prompt + Some("") timeout | Edge case | No panic |
| Ask with normal prompt + Some("30s") | Standard | No panic |
| Set primitive | Regression | No panic (existing arm) |
| Finish(String) | Regression | No panic (existing arm) |
| Finish(Integer) | Regression | No panic (existing arm) |
| Do primitive (catch-all) | Other primitives | No panic (catch-all handles) |

### Behavior B13: Finish Regression

```
### Behavior: Finish primitive digest contribution is unchanged
Given: A WorkflowSource with only a Finish step
When: canonical_digest is computed before and after the Ask fix
Then: The digest value is deterministic and follows the existing contract
```

**Test function**: `fn canonical_digest_handles_finish_primitive_unchanged_after_ask_fix()`

| Scenario | Finish Result | Expected |
|----------|--------------|----------|
| String result | `ScalarValue::String("done")` | Deterministic digest |
| Integer result | `ScalarValue::Integer(0)` | Deterministic digest, different from String |
| Integer result (large) | `ScalarValue::Integer(i64::MAX)` | Deterministic digest |

### Behavior B14: Set Regression

```
### Behavior: Set primitive digest contribution is unchanged
Given: A WorkflowSource with only a Set step
When: canonical_digest is computed
Then: The digest value is deterministic and follows the existing contract
```

**Test function**: `fn canonical_digest_handles_set_primitive_unchanged_after_ask_fix()`

| Scenario | Set Output + Value | Expected |
|----------|-------------------|----------|
| Standard Set | `output="x"`, `value="1"` | Deterministic digest |
| Different output | `output="a"`, `value="1"` | Different from above |
| Different value | `output="x"`, `value="2"` | Different from above |

### Behavior B15: Digest Covers All Semantic Fields

```
### Behavior: canonical_digest includes version, name, trigger, step IDs, and primitive fields
Given: Two WorkflowSource values differing ONLY in name (keeping all else identical)
When: canonical_digest is computed for both
Then: digests are different (proving name contributes to digest)
```

**Test function**: `fn canonical_digest_includes_version_name_trigger_step_ids_in_hash()`

| Scenario | Changed Field | Expected |
|----------|--------------|----------|
| Version differs | `"v1"` vs `"v2"` | `digest_a != digest_b` |
| Name differs | `"wf_a"` vs `"wf_b"` | `digest_a != digest_b` |
| Trigger differs | `Manual` vs `Webhook` | `digest_a != digest_b` |
| Schedule cron differs | `"*/5 * * * *"` vs `"0 0 * * *"` | `digest_a != digest_b` |
| Step ID differs | `"step_1"` vs `"step_a"` | `digest_a != digest_b` |

### Behavior B16: Empty Source (Zero Steps)

```
### Behavior: canonical_digest on a source with no steps produces a valid digest
Given: A WorkflowSource with zero steps
When: canonical_digest is computed
Then: A valid 32-byte WorkflowDigest is returned without panic
And: The digest is deterministic
```

**Test function**: `fn canonical_digest_produces_valid_digest_when_source_has_no_steps()`

### Behavior B17: Trigger Field Sensitivity

```
### Behavior: Changing trigger type changes the canonical digest
Given: Source A with trigger=Manual, Source B with trigger=Webhook (all else identical)
When: canonical_digest is computed for both
Then: digest(A) != digest(B)
```

**Test function**: `fn canonical_digest_produces_distinct_digests_when_trigger_differs()`

### Behavior B18: Version/Name Sensitivity

```
### Behavior: Changing version or name changes the canonical digest
Given: Source A with name="wf_a", Source B with name="wf_b" (all else identical)
When: canonical_digest is computed for both
Then: digest(A) != digest(B)
```

**Test function**: `fn canonical_digest_produces_distinct_digests_when_name_differs()`

### Behavior B19: Step Order Sensitivity

```
### Behavior: Changing step order changes the canonical digest
Given: Source A with steps [Ask, Set], Source B with steps [Set, Ask] (same semantic content)
When: canonical_digest is computed for both
Then: digest(A) != digest(B) (despite same semantic content — order matters for hash)
```

**Test function**: `fn canonical_digest_produces_distinct_digests_when_step_order_differs()`

---

## 4. Proptest Invariants

All 4 proptest invariants are **already materialized and PASSING** (verified by `proof-review.md` and `proof-to-rust-map.md`). No new proptest properties are needed for this bead.

| Invariant | Existing Artifact | Status |
|-----------|------------------|--------|
| PO-PROPTEST-001: Prompt sensitivity (1000 random pairs) | `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` | ✅ PASS |
| PO-PROPTEST-002: Timeout sensitivity (1000 random pairs) | `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` | ✅ PASS |
| PO-PROPTEST-003: Determinism (500 random sources) | `crates/vb_compile/tests/proptest_digest_determinism.rs` | ✅ PASS |
| PO-PROPTEST-004: Field ordering determinism (500 random inputs) | `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` | ✅ PASS |

**Recommendation**: If the proptest strategies can be enhanced, add:
- `prop_digest_empty_prompt_produces_zero_byte_hash_input`: Verify empty prompt contributes `b""` to hash (indirectly by checking that `""` and `"x"` produce distinct digests)
- `prop_digest_timeout_none_vs_some_empty_distinction`: Already covered by PO-PROPTEST-002's timeout strategy which includes `None`, `Some("")`, and `Some(arbitrary)`

These are already tested by the unit test matrix in Section 3. No additional proptest artifacts needed.

---

## 5. Fuzz Targets

One fuzz target is **already materialized** and compiles (`fuzz/fuzz_targets/canonical_digest_ask.rs`). No new fuzz targets needed.

| Target | Artifact | Status |
|--------|----------|--------|
| PO-FUZZ-001: Adversarial input robustness | `fuzz/fuzz_targets/canonical_digest_ask.rs` | ✅ compiles, not executed |

**Recommendation**: Execute the fuzz target with `cargo fuzz run canonical_digest_ask -- -max_len=65536 -runs=100000` in State 12 (formal-verifier). No additional fuzz targets needed.

---

## 6. Kani Verification Harnesses

All 6 Kani harnesses are **already materialized** (wired in `lib.rs`, compiles, blocked by blake3 inline asm). No new Kani harnesses needed for this bead.

| Harness | Existing Artifact | Status |
|---------|------------------|--------|
| PO-KANI-001: Prompt sensitivity | `crates/vb_compile/src/kani_digest_ask_prompt_sensitivity.rs` | ⚠️ materialized, blake3 asm barrier |
| PO-KANI-002: Timeout sensitivity | `crates/vb_compile/src/kani_digest_ask_timeout_sensitivity.rs` | ⚠️ materialized |
| PO-KANI-003: Empty prompt distinct | `crates/vb_compile/src/kani_digest_ask_empty_prompt.rs` | ⚠️ materialized |
| PO-KANI-004: Sentinel distinction | `crates/vb_compile/src/kani_digest_ask_timeout_sentinel.rs` | ⚠️ materialized |
| PO-KANI-005: Field ordering | `crates/vb_compile/src/kani_digest_ask_field_ordering.rs` | ⚠️ materialized |
| PO-KANI-006: Panic-freedom | `crates/vb_compile/src/kani_digest_step_primitive_no_panic.rs` | ⚠️ materialized |

**Recommendation**: When Kani gains inline assembly support, these harnesses will execute meaningfully. In the meantime, they serve as structural proof artifacts demonstrating correct harness design. No additional Kani work needed for this bead.

---

## 7. Mutation Checkpoints

`cargo-mutants` introduces mutations to verify test quality. All new behavior tests must catch the following mutations.

### Critical Mutations That MUST Be Caught

| Mutation | Target | Expected Caught By | Rationale |
|----------|--------|-------------------|-----------|
| Remove `b"ask"` update | `digest_step_primitive` Ask arm, line 159 | B1 (prompt sensitivity) → `digest_a != digest_b` fails | Without the tag, changing prompt might still produce different hash (just from prompt bytes), but the structural marker is lost |
| Remove `prompt.as_bytes()` update | `digest_step_primitive` Ask arm, line 160 | B1 (prompt sensitivity) → test asserting different prompts produce different digests fails | Core semantic field |
| Remove `b"timeout"` update | `digest_step_primitive` Ask arm, line 163 | B2 (timeout sensitivity) → test asserting different timeouts produce different digests fails | Sentinel prefix lost |
| Remove `t.as_bytes()` update | `digest_step_primitive` Ask arm, line 164 | B2 (timeout sensitivity) → test with `Some("30s")` vs `Some("10s")` fails | Timeout value lost |
| Replace `b"no_timeout"` with `b""` | `digest_step_primitive` Ask arm, line 168 | B5 (None vs Some("")) → digests now equal, test fails | Sentinel collision with empty string |
| Replace `b"no_timeout"` with `b"timeout"` | `digest_step_primitive` Ask arm, line 168 | B5 (None vs Some("")) → `None` and `Some("")` produce different digests but sentinel collision possible with specific timeout values | Sentinel ambiguity |
| Replace `Some(t)` arm with `None` arm logic | `digest_step_primitive` Ask arm, lines 162-164 | B2 (timeout sensitivity) → test with `Some("30s")` fails to produce distinct digest from `None` | Timeout information lost |
| Remove entire `Ask { prompt, timeout }` arm (rely on catch-all) | `digest_step_primitive`, lines 158-170 | B8 (explicit arm) → source does not contain `Ask { prompt, timeout }` → grep test fails; B1 (prompt sensitivity) fails | Reintroduces original bug |
| Reorder: `prompt` before `b"ask"` | `digest_step_primitive` Ask arm | B9 (field ordering) → determinism test might still pass (if both calls use same bad order) but sensitive inputs will reveal difference | Ordering contract violated |
| Change `Finish(ScalarValue::String)` arm to skip digest | `digest_step_primitive` Finish arm | B13 (Finish regression) → test with Finish(String) produces different digest from pre-fix expectation | Set/Finish regression |
| Change `Set { output, value }` arm to skip value | `digest_step_primitive` Set arm | B14 (Set regression) → test with different Set values produces same digest | Set/Finish regression |

### Mutation Kill Rate Target

**≥90%** for the new code paths (the Ask match arm, lines 158-170 in part_05.rs). Existing code paths (Set/Finish arms, lines 144-157) are covered by the pre-existing 245 tests and should maintain their existing kill rate.

### Mutation Testing Commands

```bash
# Run mutants scoped to the digest_step_primitive function
cargo mutants -p vb_compile \
  --function digest_step_primitive \
  --test-tool nextest

# Or scoped to the source file
cargo mutants -p vb_compile \
  --file crates/vb_compile/src/mod_compile_lowering/part_05.rs \
  --test-tool nextest
```

---

## 8. Combinatorial Coverage Matrix

### Unit Test Group: Prompt Sensitivity (B1)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Different prompts (simple) | `"hello"` vs `"world"` | `digest_a != digest_b` | unit | `canonical_digest_produces_distinct_digests_when_ask_prompt_differs` |
| 2 | One empty, one non-empty | `""` vs `"hello"` | `digest_a != digest_b` | unit | (same function, sub-case) |
| 3 | Same prompt | `"same"` vs `"same"` | `digest_a == digest_b` | unit | (same function, sub-case) |
| 4 | Special chars in prompt | `"a\nb"` vs `"a\tb"` | `digest_a != digest_b` | unit | (same function, sub-case) |
| 5 | Unicode prompt diff | `"héllo"` vs `"hëllo"` | `digest_a != digest_b` | unit | (same function, sub-case) |
| 6 | Long prompts (>1KB) | `"a"*2048` vs `"b"*2048` | `digest_a != digest_b` | unit | (same function, sub-case) |
| 7 | Prompt sensitivity via proptest | Random prompt pairs | Invariant holds for 1000 runs | proptest | `prop_digest_prompt_sensitivity` (existing) |

### Unit Test Group: Timeout Sensitivity (B2)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | None vs Some(value) | `None` vs `Some("30s")` | `digest_a != digest_b` | unit | `canonical_digest_produces_distinct_digests_when_ask_timeout_differs` |
| 2 | Two different Some | `Some("10s")` vs `Some("30s")` | `digest_a != digest_b` | unit | (same function) |
| 3 | None vs Some(empty) | `None` vs `Some("")` | `digest_a != digest_b` | unit | (same function, separate assertion) |
| 4 | Same timeout value | `Some("30s")` vs `Some("30s")` | `digest_a == digest_b` | unit | (same function) |
| 5 | Special chars in timeout | `Some("10s")` vs `Some("10\ns")` | `digest_a != digest_b` | unit | (same function) |
| 6 | Timeout sensitivity via proptest | Random timeout pairs (None/Some("")/Some(arb)) | Invariant holds for 1000 runs | proptest | `prop_digest_timeout_sensitivity` (existing) |

### Unit Test Group: Determinism (B3)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Ask source called 3 times | Fixed source with Ask(Some timeout) | All 3 digests equal | unit | `canonical_digest_is_deterministic_when_called_multiple_times` |
| 2 | Empty prompt source called 3 times | Fixed source with Ask("") | All 3 digests equal | unit | (same function) |
| 3 | Multi-step source called 3 times | Fixed source with 3 Ask steps | All 3 digests equal | unit | (same function) |
| 4 | Determinism via proptest | 500 random sources | Invariant holds | proptest | `prop_digest_determinism` (existing) |

### Unit Test Group: Empty Prompt (B4)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Empty vs non-empty | `""` vs `"hello"` | `digest_a != digest_b` | unit | `canonical_digest_produces_distinct_digest_when_ask_prompt_is_empty` |
| 2 | Empty prompt produces valid hash | `""` | `digest.as_bytes()` is `[u8; 32]`, not all zeros | unit | (same function) |
| 3 | Two empty prompt sources | `""` (identical) | `digest_a == digest_b` | unit | (same function) |
| 4 | Empty prompt via Kani | Bounded: `""` vs any 1..128 byte prompt | `digest_a != digest_b` for all | kani | `check_empty_prompt_distinct` (existing) |

### Unit Test Group: None vs Some("") Timeout (B5)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | None vs Some("") | `None` vs `Some("")` | `digest_a != digest_b` | unit | `canonical_digest_produces_distinct_digests_when_timeout_none_vs_some_empty` |
| 2 | None vs Some("30s") | `None` vs `Some("30s")` | `digest_a != digest_b` | unit | (same function) |
| 3 | Some("") vs Some("30s") | `Some("")` vs `Some("30s")` | `digest_a != digest_b` | unit | (same function) |
| 4 | Sentinel via Kani | Bounded: None vs Some("") | `digest_a != digest_b` for all valid inputs | kani | `check_timeout_sentinel_distinction` (existing) |

### Unit Test Group: Field Ordering (B9)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Same source, different calls | Fixed Ask source | Same digest both calls | unit | `digest_step_primitive_hashes_ask_fields_in_deterministic_order` |
| 2 | Same fields, different declaration order | Sources constructed with same field values | Same digest from canonical_digest | unit | (same function) |
| 3 | Field ordering via proptest | 500 random Ask inputs | Determinism holds | proptest | `prop_digest_ask_ordering` (existing) |

### Unit Test Group: Panic Freedom (B12)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Normal Ask | prompt="hello", timeout=Some("30s") | No panic, returns | unit | `digest_step_primitive_does_not_panic_for_valid_ask_variants` |
| 2 | Empty prompt Ask | prompt="", timeout=None | No panic, returns | unit | (same function) |
| 3 | Large prompt Ask | prompt="a" * 10240, timeout=None | No panic, returns | unit | (same function) |
| 4 | Set primitive | Set{output, value} | No panic | unit | (same function) |
| 5 | Finish(String) | Finish{result:String("x")} | No panic | unit | (same function) |
| 6 | Finish(Integer) | Finish{result:Integer(42)} | No panic | unit | (same function) |
| 7 | Do primitive | Do{...} | No panic (catch-all) | unit | (same function) |
| 8 | Panic-freedom via Kani | All StepPrimitive variants, bounded prompt/timeout | No panic for any | kani | `check_digest_step_primitive_no_panic` (existing) |

### Unit Test Group: Empty Prompt Input Validity (B10)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Empty prompt, None timeout | `""`, `None` | No panic, valid 32-byte digest | unit | `digest_step_primitive_accepts_empty_prompt_without_panic` |
| 2 | Empty prompt, Some("") timeout | `""`, `Some("")` | No panic, valid digest | unit | (same function) |
| 3 | Empty prompt, Some("30s") timeout | `""`, `Some("30s")` | No panic, valid digest | unit | (same function) |

### Unit Test Group: Timeout Sentinel Distinction (B11)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | None vs Some("") | Same prompt, `None` vs `Some("")` | `digest_a != digest_b` | unit | `digest_step_primitive_uses_distinct_sentinel_for_none_timeout` |
| 2 | Source code check | `grep 'b"no_timeout"' part_05.rs` | Found | static | (code review assertion) |
| 3 | Source code check | `grep 'b"timeout"' part_05.rs` | Found in Some arm | static | (code review assertion) |

### Unit Test Group: Steps (B16)

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Zero steps | `steps: vec![]` | Valid 32-byte digest, no panic | unit | `canonical_digest_produces_valid_digest_when_source_has_no_steps` |
| 2 | One step | 1 Ask step | Valid digest | unit | (covered by B1 tests) |
| 3 | Multiple steps | 3 Ask steps | Valid digest, different from 1-step variant | unit | (covered by B1/B19 tests) |

### Integration Test Group: Compilation Pipeline

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | Compile YAML with Ask, verify embedded digest | YAML string with Ask | `CompiledWorkflow::digest()` matches direct `canonical_digest` call | integration | `compiled_workflow_digest_matches_canonical_digest_for_ask_workflow` |
| 2 | Compile YAML without Ask, verify digest unchanged | YAML string with Set+Finish | `CompiledWorkflow::digest()` remains consistent | integration | `compiled_workflow_digest_unchanged_for_set_finish_workflow` |
| 3 | Compile same YAML twice, verify same digest | Fixed YAML string | Both `CompiledWorkflow::digest()` values identical | integration | `compiled_workflow_digest_is_deterministic_across_compilations` |
| 4 | Compile two YAMLs differing only in Ask prompt | Two YAML strings | `CompiledWorkflow::digest()` values differ | integration | `compiled_workflow_digests_differ_when_ask_prompt_differs` |
| 5 | Compile two YAMLs with same Ask, different names | Two YAML strings | `CompiledWorkflow::digest()` values differ (name in digest) | integration | `compiled_workflow_digests_differ_when_workflow_name_differs` |

### E2E Test Group: Full YAML → Compile → Verify

| # | Scenario | Input Class | Expected Output | Test Layer | Test Function |
|---|----------|-------------|-----------------|------------|---------------|
| 1 | YAML string with Ask and timeout → compile → verify digest reflects timeout | Raw YAML bytes | `canonical_digest` changes when timeout differs | e2e | `yaml_with_ask_and_timeout_produces_semantic_digest` |
| 2 | YAML string with Ask and empty prompt → compile → verify digest is valid | Raw YAML bytes | `canonical_digest` is valid and distinct from non-empty prompt | e2e | `yaml_with_ask_and_empty_prompt_produces_valid_digest` |

### Static Analysis Group (compile-time)

| # | Scenario | Verification Method | Expected Output | Test Layer | Test Function |
|---|----------|-------------------|-----------------|------------|---------------|
| 1 | Explicit Ask arm exists | `grep 'Ask { prompt, timeout }' part_05.rs` | Match found at line 158 | static | (code review, script check) |
| 2 | No `unwrap`, `expect`, `panic` in digest_step_primitive | `grep -E '(unwrap|expect|panic|todo|unimplemented)' part_05.rs` (lines 140-174) | No matches | static | (code review, script check) |

---

## Test File Organization

All new test files should be placed in `crates/vb_compile/tests/`:

| Test File | Behaviors Covered | Layer | Type |
|-----------|------------------|-------|------|
| `tests/digest_ask_explicit_arm.rs` | B8 (explicit arm) + B12 (panic-freedom) + B10 (empty prompt validity) + B11 (sentinel) | unit | `#[test]` |
| `tests/digest_ask_prompt_sensitivity.rs` | B1 (prompt sensitivity) | unit | `#[test]` |
| `tests/digest_ask_timeout_sensitivity.rs` | B2 (timeout sensitivity) + B5 (None vs Some("")) | unit | `#[test]` |
| `tests/digest_ask_empty_prompt.rs` | B4 (empty prompt) | unit | `#[test]` |
| `tests/digest_ask_determinism.rs` | B3 (determinism) + B9 (field ordering) | unit | `#[test]` |
| `tests/digest_set_finish_regression.rs` | B7 (regression) + B13 (Finish) + B14 (Set) | unit | `#[test]` |
| `tests/digest_structural_fields.rs` | B15 (version/name/trigger) + B16 (empty source) + B17 (trigger) + B18 (name) + B19 (step order) | unit | `#[test]` |
| `tests/digest_compilation_pipeline.rs` | Integration B1-B19 (via compilation pipeline) | integration | `#[test]` |
| `tests/digest_yaml_e2e.rs` | E2E B1, B2 (YAML string → compile → verify) | e2e | `#[test]` |

### Shared Test Helpers

Create `tests/digest_test_helpers.rs` (not a test target, used via `#[path]` or `mod`):

```rust
// Shared helpers for digest behavior tests

/// Build a minimal WorkflowSource with a single Ask step.
pub fn ask_source(prompt: &str, timeout: Option<&str>) -> WorkflowSource { ... }

/// Build a WorkflowSource with only Set + Finish steps.
pub fn set_finish_source() -> WorkflowSource { ... }

/// Build a WorkflowSource with zero steps.
pub fn empty_source() -> WorkflowSource { ... }

/// Build a WorkflowSource with a single Finish step.
pub fn finish_source(result: ScalarValue) -> WorkflowSource { ... }

/// Build a WorkflowSource with a single Set step.
pub fn set_source(output: &str, value: &str) -> WorkflowSource { ... }

/// Assert two digests are different with a descriptive message.
pub fn assert_digests_differ(a: WorkflowDigest, b: WorkflowDigest, context: &str) { ... }

/// Assert two digests are equal with a descriptive message.
pub fn assert_digests_equal(a: WorkflowDigest, b: WorkflowDigest, context: &str) { ... }
```

---

## Test Execution Commands

```bash
# Run all new digest behavior tests
cargo test -p vb_compile --test digest_ask_explicit_arm
cargo test -p vb_compile --test digest_ask_prompt_sensitivity
cargo test -p vb_compile --test digest_ask_timeout_sensitivity
cargo test -p vb_compile --test digest_ask_empty_prompt
cargo test -p vb_compile --test digest_ask_determinism
cargo test -p vb_compile --test digest_set_finish_regression
cargo test -p vb_compile --test digest_structural_fields
cargo test -p vb_compile --test digest_compilation_pipeline
cargo test -p vb_compile --test digest_yaml_e2e

# Run all digest tests together
cargo test -p vb_compile --test digest_

# Run regression suite (all vb_compile tests, including 245 existing + new)
cargo test -p vb_compile

# Run mutation tests scoped to digest code
cargo mutants -p vb_compile \
  --file crates/vb_compile/src/mod_compile_lowering/part_05.rs \
  --function digest_step_primitive \
  --test-tool nextest

# Static verification (CI gate)
grep -n 'Ask { prompt, timeout }' crates/vb_compile/src/mod_compile_lowering/part_05.rs
grep -n 'Ask { prompt, timeout }' crates/vb_compile/src/compile/mod.rs
grep -n -E '\b(unwrap|expect|panic|todo|unimplemented)\b' \
  crates/vb_compile/src/mod_compile_lowering/part_05.rs \
  | awk -F: '$2 >= 140 && $2 <= 174 {print}'
```

---

## Open Questions

1. **Parity test execution gap**: PO-UT-003 inline tests in `compile/mod.rs` are in an unmounted module (no `mod compile;` in `lib.rs`). Should they be:
   - (a) Extracted to `tests/digest_duplicate_parity.rs` as a proper integration test?
   - (b) Wired in via `#[path]` attribute?
   - (c) Left as dead code since the legacy path is itself dead code?  
   **Recommended**: Option (a) — extract to integration test. Even though `compile/mod.rs` is dead code, the parity test verifies both implementations use identical logic, which is valuable as a defensive check.

2. **Explicit arm verification**: B8 (explicit Ask arm) is a static code-review test. Should it be:
   - (a) A grep-based CI check (fast, reliable, can't "fake" it)?
   - (b) A runtime test that feeds Ask through `canonical_digest` and verifies the digest differs from a simulated catch-all-only path?  
   **Recommended**: Both. Option (a) for CI gate, option (b) as a unit test proving the explicit arm produces different results than the catch-all would.

3. **Should we include a golden-digest test?** Some projects use golden-file testing for hash values. A golden digest test would assert that `canonical_digest` for a specific fixed source produces a known, pinned hash value. This protects against accidental changes to the hash algorithm.
   - **Pro**: Catches any change to the digest computation (accidental or malicious).
   - **Con**: If the hash algorithm is intentionally updated, golden values must be rotated.
   - **Recommended**: Yes, include one golden test: `canonical_digest(basic_ask_source) == known_32_byte_hex_value`. This is a single test, easy to update if needed, and provides strong regression protection.

4. **Should the `Other` catch-all arm be tested exhaustively?** The catch-all covers `Do`, `Choose`, `ForEach`, `Together`, `Collect`, `Aggregate`, `Repeat`, and `Wait` primitives. Each currently contributes only `canonical_primitive_name()` to the digest. Should we add unit tests for each to prevent future regressions?
   - **Recommended**: Not in P1 scope. The catch-all behavior is working as designed for non-Ask primitives. Future beads will extend per-primitive hashing. A single test verifying `canonical_primitive_name` produces distinct names for each variant and that `digest_step_primitive` uses the catch-all without panic is sufficient.

5. **Should `WorkflowDigest` comparison tests use `assert_eq!` or compare byte arrays?** `WorkflowDigest` derives `PartialEq, Eq`. Prefer `assert_eq!` / `assert_ne!` for clarity. Byte-level comparisons are redundant unless you specifically want to test the `[u8; 32]` representation.

---

## Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario — 19 behaviors, 19 scenario groups
- [x] Every pure function with multiple inputs has at least one proptest invariant — 4 existing, no new needed
- [x] Every parsing/deserialization boundary has a fuzz target — 1 existing, no new needed
- [x] Every error variant in the Error enum has an explicit test scenario — `canonical_digest` is infallible; semantic errors covered by sensitivity tests
- [x] Mutation threshold target (≥90%) is stated — see Section 7
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value — all assertions specify exact `WorkflowDigest` equality/inequality or valid `[u8; 32]` bytes
- [x] Test file organization specified — see test file table in Section 8
- [x] Shared test helpers identified — `digest_test_helpers.rs`
- [x] Execution commands provided — see Test Execution Commands section
- [x] Open questions raised — 5 questions for test-writer attention

## Handoff for test-writer (State 9)

This test plan covers 19 behaviors across 9 test files. The implementation order should be:

1. **First**: `digest_test_helpers.rs` — shared source constructors (ask_source, set_finish_source, etc.)
2. **Second**: `digest_ask_prompt_sensitivity.rs` and `digest_ask_timeout_sensitivity.rs` — core B1+B2
3. **Third**: `digest_ask_determinism.rs` + `digest_ask_empty_prompt.rs` — B3+B4
4. **Fourth**: `digest_ask_explicit_arm.rs` — B8+B10+B11+B12 (static + runtime verification)
5. **Fifth**: `digest_set_finish_regression.rs` — B7+B13+B14 (regression protection)
6. **Sixth**: `digest_structural_fields.rs` — B15+B16+B17+B18+B19 (structural/identity)
7. **Seventh**: `digest_compilation_pipeline.rs` — integration tests
8. **Eighth**: `digest_yaml_e2e.rs` — E2E tests
9. **Finally**: `digest_duplicate_parity.rs` — extract PO-UT-003 from dead code

All tests must use the public API (`vb_compile::canonical_digest`, `vb_compile::digest_step_primitive`) re-exported via `lib.rs`. Never use `crate::lwr::` paths in test files.
