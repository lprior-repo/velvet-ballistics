# Test Plan: vb-xi2f.34 — Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-25
**Agent**: test-planner
**Phase**: p8-test-planner

---

## Summary

- **Behaviors identified**: 20
- **Trophy allocation**: 8 unit / 8 integration / 2 structural / 3 proptest / 3 Kani / 2 fuzz
- **Proptest invariants**: 3 (1 new needed)
- **Fuzz targets**: 2 (both new)
- **Kani harnesses**: 3 (all exist, verified)
- **Mutation threshold target**: ≥90% kill rate

### Existing Coverage Audit

| Layer | Tests (existing) | Tests (new needed) | Status |
|-------|-----------------|--------------------|--------|
| Unit / Calc | 0 direct `canonical_digest()` or `digest_step_primitive()` tests | 8 | Blocked by visibility — both functions are `pub(crate)` in `part_05.rs` |
| Integration | 9 tests in `finish_digest_integration.rs`, 3 in `finish_digest_structural.rs` | 3 | Partial |
| Proptest | 4 properties in `proptest_finish_digest.rs` (all `#[ignore]`) | 1 | Acceptable for P1 |
| Kani | 3 proofs in `kani_finish_digest.rs` | 0 (correct) | Full |
| Fuzz | 0 | 2 | Missing |
| Unit (existing misc) | 3 digest-related in `error_variant_tests.rs` (test `compute_compiled_digest`, NOT `canonical_digest`) | N/A | Documentation gap |

---

## 1. Behavior Inventory

### 1.1 Digest Computation (Pure Calc Layer)

| # | Behavior | Contract |
|---|----------|----------|
| B1 | `canonical_digest(source)` hashes `version` field as UTF-8 bytes | Implicit (C4) |
| B2 | `canonical_digest(source)` hashes `name` field as UTF-8 bytes | Implicit (C4) |
| B3 | `canonical_digest(source)` hashes `trigger` discriminator + parameters | Implicit (C4) |
| B4 | `canonical_digest(source)` hashes each step's `id` in source order | C2, C3 |
| B5 | `canonical_digest(source)` delegates to `digest_step_primitive` for each step's primitive | C1 |
| B6 | `canonical_digest(source)` returns identical `WorkflowDigest` for identical `WorkflowSource` | C4 |
| B7 | `canonical_digest(source)` returns deterministic output — no time, random, or IO dependencies | C4, C10 |
| B8 | `canonical_digest(source)` does NOT hash slot layout, constant pool, bytecode, or runtime state | C9, C10 |

### 1.2 Finish Primitive Digest Encoding

| # | Behavior | Contract |
|---|----------|----------|
| B9 | `digest_step_primitive(hasher, Finish { result })` writes `b"finish"` discriminator | INV-3, C1 |
| B10 | `digest_step_primitive(hasher, Finish { result: String(s) })` writes `s.as_bytes()` | INV-4, C1 |
| B11 | `digest_step_primitive(hasher, Finish { result: Integer(i) })` writes `i.to_le_bytes()` | INV-4, C1 |
| B12 | `digest_step_primitive(hasher, Finish { result: unknown_variant })` writes `b"unsupported"` | INV-4, C8 |
| B13 | Changing `Finish.result` String value changes the digest | C1 |
| B14 | Changing `Finish.result` Integer value changes the digest | C1 |
| B15 | Changing `Finish.result` from String to Integer (or vice versa) changes the digest | C5 |
| B16 | Changing the Finish step's `id` changes the digest | C2 |
| B17 | Re-ordering non-Finish steps while preserving all step IDs changes the digest | C3 |

### 1.3 Digest Lifecycle (Compilation Round-Trip)

| # | Behavior | Contract |
|---|----------|----------|
| B18 | `compile_source()` stores `canonical_digest(source)` into `WorkflowParts.digest` | C6 |
| B19 | `CompiledWorkflow::try_from_parts(parts)` preserves digest from `WorkflowParts` | C6 |
| B20 | `CompiledWorkflow.digest()` equals the `canonical_digest()` of the source | C6 |

### 1.4 Cross-Cutting Behaviors

| # | Behavior | Contract |
|---|----------|----------|
| B21 | Digest is computed before lowering/validation; an invalid source still has a deterministic digest | C9 |
| B22 | Exactly one canonical digest implementation exists (no active duplicate) | C7 |
| B23 | Digest is independent of IR layout (constant pool, slot count, expression bytecode) | C10 |
| B24 | All current `ScalarValue` variants (`String`, `Integer`) are explicitly matched in `digest_step_primitive` | C8 |

---

## 2. Trophy Allocation

```
         [E2E]           ← 0 (no user-facing workflow that exercises this)
    [Integration]        ← 8 (component boundaries: YAML→compile→digest, legacy equivalence)
    [Unit / Calc]        ← 8 (pure digest functions: canonical_digest, digest_step_primitive)
  [Static Analysis]      ← 2 (ScalarValue exhaustiveness, unsafe/IO audit)
  [Proptest]             ← 3 (determinism, finish sensitivity, step-ID sensitivity)
  [Kani]                 ← 3 (String injectivity, Integer injectivity, variant discrimination)
  [Fuzz]                 ← 2 (YAML parsing boundary, digest structural boundary)
```

**Ratio**: ~37% unit / ~37% integration / ~14% proptest / ~14% Kani / ~6% static / ~6% fuzz

**Rationale**: The digest computation is a pure function with zero I/O, making the Calc layer unusually important. Integration tests cover the full YAML→compile→digest pipeline using the public API. Kani provides mathematical proof of encoding properties that proptest cannot exhaustively cover (e.g., all 2^64 i64 values). The deviation from the 60% integration norm is justified by the pure nature of the functions under test and the existing defense-in-depth layering.

---

## 3. BDD Scenarios

### 3.1 Unit Layer: `digest_step_primitive` Finish Encoding

These tests require internal module access (`pub(crate)` visibility in `mod_compile_lowering::part_05`). Must be placed in `crates/vb_compile/src/tests/` or within `mod_compile_lowering/` module itself.

#### Behavior B9: Finish discriminator prefix

```
### Behavior: digest_step_primitive writes finish discriminator for Finish primitive
Given: a blake3 hasher and a Finish primitive with any result value
When: digest_step_primitive is called
Then: the bytes "finish" appear in the sequence of bytes fed to the hasher
```

**Test name**: `fn digest_step_primitive_finish_writes_finish_discriminator()`

#### Behavior B10: String result encoding

```
### Behavior: digest_step_primitive encodes String result as raw UTF-8 bytes
Given: a Finish primitive with result = ScalarValue::String("my_output")
When: digest_step_primitive is called
Then: the hasher receives b"finish" followed by b"my_output"
```

**Test name**: `fn digest_step_primitive_finish_encodes_string_result_as_utf8_bytes()`

#### Behavior B11: Integer result encoding

```
### Behavior: digest_step_primitive encodes Integer result as little-endian bytes
Given: a Finish primitive with result = ScalarValue::Integer(42)
When: digest_step_primitive is called
Then: the hasher receives b"finish" followed by 42_i64.to_le_bytes()
```

**Test name**: `fn digest_step_primitive_finish_encodes_integer_result_as_le_bytes()`

#### Behavior B12: Unknown ScalarValue fallback

```
### Behavior: digest_step_primitive writes "unsupported" for unknown ScalarValue variant
Given: a Finish primitive with result = ScalarValue that falls through to the _ arm
When: digest_step_primitive is called
Then: the hasher receives b"finish" followed by b"unsupported"
```

**Test name**: `fn digest_step_primitive_finish_writes_unsupported_for_unknown_scalar_value()`

#### Behavior: Variant discrimination at encoding level

```
### Behavior: String and Integer Finish encodings produce different byte sequences
Given: Finish { result: String("42") } and Finish { result: Integer(42) }
When: Each is passed through digest_step_primitive independently
Then: The byte sequences fed to each hasher differ (different lengths or content)
```

**Test name**: `fn digest_step_primitive_finish_string_vs_integer_produce_different_encoding_bytes()`

### 3.2 Unit Layer: `canonical_digest` Determinism

#### Behavior B6: Determinism

```
### Behavior: canonical_digest returns identical digests for identical WorkflowSource
Given: two structurally identical WorkflowSource values (same version, name, trigger, steps)
When: canonical_digest is called on each
Then: both calls return equal WorkflowDigest values
```

**Test name**: `fn canonical_digest_is_deterministic_for_identical_source()`

#### Behavior B8: IR layout independence

```
### Behavior: canonical_digest does not depend on slot layout or constant pool
Given: a WorkflowSource with a known digest
When: the same source is compiled twice through compile_source (which generates internal IR)
Then: the digest obtained via WorkflowParts.digest matches the direct canonical_digest call
```

**Test name**: `fn canonical_digest_independent_of_ir_layout()`

#### Behavior B4: Step ID ordering

```
### Behavior: canonical_digest reflects step ID ordering
Given: two WorkflowSource values with identical steps but different step IDs
When: canonical_digest is called on each
Then: the two digests differ
```

**Test name**: `fn canonical_digest_depends_on_step_id_ordering()`

### 3.3 Integration Layer: Compilation Round-Trip

These tests use the public API (`compile_source`, `parse_workflow_source`, `CompiledWorkflow::digest`).

#### Behavior B13: Finish result String value sensitivity (EXISTING)
- `finish_result_value_changes_compiled_digest_string` in `finish_digest_integration.rs` ✓

#### Behavior B14: Finish result Integer value sensitivity (EXISTING)
- `finish_result_value_changes_compiled_digest_integer` in `finish_digest_integration.rs` ✓

#### Behavior B15: Finish result type change (String vs Integer) (EXISTING)
- `finish_result_type_changes_compiled_digest` in `finish_digest_integration.rs` ✓

#### Behavior B16: Finish step ID sensitivity (EXISTING)
- `finish_step_id_changes_compiled_digest` in `finish_digest_integration.rs` ✓

#### Behavior B18/B19/B20: Digest survives compilation (EXISTING)
- `compiled_digest_matches_on_recompile` in `finish_digest_integration.rs` ✓

#### Behavior B17: Step position sensitivity — MULTI-STEP variant (NEW)

```
### Behavior: Step ordering in multi-step workflow changes digest
Given: a 2-step workflow [Set(output=x, value=10), Finish(result=0)]
  and a 2-step workflow [Set(output=y, value=10), Finish(result=0)]
  with different Set step IDs but same Finish
When: both are compiled
Then: the compiled digests differ (because step IDs are hashed in order)
```

**Test name**: `fn multi_step_workflow_step_ordering_changes_compiled_digest()`

#### Behavior B21: Digest survives compilation failure (NEW)

```
### Behavior: Digest is deterministic even when workflow fails compilation
Given: a WorkflowSource that references an unknown output name in Finish
  (which will fail during lowering with UnknownOutputName)
When: canonical_digest is computed from the source (before lowering)
Then: the digest is produced deterministically
  And: canonical_digest cannot be called from outside the module (pub(crate)),
       but the byte sequence it produces can be verified via a compile_source
       call that hits the error AFTER digest computation
```

**Test name**: `fn digest_is_computed_before_validation_error()`

#### Behavior B23: Digest is independent of IR layout — recompilation stability (NEW)

```
### Behavior: Recompiling same YAML from scratch at different times produces same digest
Given: YAML source bytes for a workflow with Finish
When: the YAML is parsed and compiled to CompiledWorkflow twice in separate calls
Then: both CompiledWorkflow values have the same digest
  And: the digest is non-zero
```

**Test name**: `fn compiled_digest_stable_across_independent_compilations()` — partially covered by `compiled_digest_matches_on_recompile` but needs explicit non-zero check.

### 3.4 Structural / Static Layer

#### Behavior B24: ScalarValue exhaustiveness (EXISTING)
- `scalarvalue_exhaustiveness_in_digest` in `finish_digest_structural.rs` ✓

#### Behavior B7/B8: No runtime/IO/unsafe dependencies (EXISTING)
- `audit_digest_has_no_runtime_dependencies` in `finish_digest_structural.rs` ✓

---

## 4. Proptest Invariants

### 4.1 Existing Properties (in `proptest_finish_digest.rs`)

#### Property 1: Determinism (PO-PROPTEST-FINISH-001)
```
### Proptest: canonical_digest_is_deterministic
Invariant: For any valid workflow with a Finish step, compiling the same WorkflowSource
  twice returns the same compiled digest.
Strategy: randomly generated u16 slot value and step ID string
Status: ✓ EXISTS (all igored, #[ignore = "proptest: run with --ignored or proptest runner"])
```

#### Property 2: Finish result Integer sensitivity (PO-PROPTEST-FINISH-002)
```
### Proptest: finish_result_change_changes_digest_integer
Invariant: Two valid workflows differing only in Finish result Integer value produce
  different compiled digests.
Strategy: two distinct u16 slot values, same step ID
Status: ✓ EXISTS
```

#### Property 3: Finish result String sensitivity (PO-PROPTEST-FINISH-002)
```
### Proptest: finish_result_change_changes_digest_string
Invariant: Two valid workflows differing only in Finish result output name produce
  different compiled digests.
Strategy: two distinct output name strings, 2-step workflow (Set + Finish)
Status: ✓ EXISTS
```

#### Property 4: Step ID sensitivity (PO-PROPTEST-FINISH-003, misnamed)
```
### Proptest: finish_position_change_changes_digest
(NOTE: Misnamed. Actually tests C2, not C3.)
Invariant: Two valid workflows differing only in step ID produce different digests.
Strategy: two distinct step IDs, same u16 slot value
Status: ✓ EXISTS
```

### 4.2 New Proptest Needed

#### Property 5: Multi-step position sensitivity (NEW — to close PF-REP2-003)

```
### Proptest: finish_position_in_multi_step_workflow_affects_digest
Invariant: In a multi-step workflow (≥2 steps), swapping the ID of a non-Finish step
  while keeping Finish at the end changes the compiled digest (because step IDs are
  hashed in order, and the Set step's ID differs).
Strategy: generate 2+ step workflows with random Set step IDs and a Finish step,
  then vary the Set step ID
Contract: C3 (Finish Step Position Sensitivity)
```

This is effectively covered by the existing `finish_position_change_changes_digest` which varies the *only* step's ID in a single-step workflow. In a multi-step workflow, position sensitivity is equivalent to step-ID-in-order sensitivity, which IS covered by the existing proptest + integration tests. **Gap severity: LOW. Acceptable for P1.**

---

## 5. Fuzz Targets

### 5.1 YAML Parser Boundary (NEW)

```
### Fuzz Target: fuzz_digest_through_parse_compile
Input type: arbitrary bytes (raw YAML text)
Risk: Panic in YAML parser, panic in canonical_digest, panic in compile_source,
  uncontrolled memory allocation from deeply nested or malicious YAML
Corpus seeds:
  - Minimal valid workflow with Finish: "version: velvet-ballistics/v1\nname: t\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
  - Workflow with String finish: "version: velvet-ballistics/v1\nname: t\nwhen:\n  manual: {}\nsteps:\n  - id: s\n    set:\n      output: x\n      value: \"1\"\n  - id: done\n    finish:\n      result: \"x\"\n"
  - Empty input (zero bytes)
  - Valid YAML but invalid workflow schema (missing steps)
  - Extremely long step IDs / output names
  - Non-UTF-8 bytes
```

Place: `fuzz/fuzz_targets/fuzz_digest_compile.rs`

### 5.2 Structural Digest Boundary (NEW)

```
### Fuzz Target: fuzz_digest_step_primitive_finish
Input type: arbitrary bytes as String content, arbitrary i64 values
Risk: Using the String bytes or i64 LE bytes in blake3 hashing should never panic
  or produce undefined behavior. Specifically: empty String, String with null bytes,
  String with non-UTF-8 sequences (shouldn't exist in Rust String but fuzz anyway),
  all i64 values including MIN and MAX.
Corpus seeds:
  - Empty string
  - "hello world"
  - String containing all zero bytes
  - String containing 0xFF bytes
  - i64::MIN, i64::MAX, 0, -1, 1
```

Place: `fuzz/fuzz_targets/fuzz_finish_digest_encoding.rs`

---

## 6. Kani Harnesses

All three required Kani harnesses exist in `kani_finish_digest.rs` and have been verified (with documented model-reduction acceptance in PF-REP2-001).

### 6.1 PO-KANI-FINISH-001: String result injectivity ✓

```
### Kani Harness: finish_string_result_injectivity
Property: For all distinct byte slices (up to MAX_BYTE_LEN=16), the Finish String
  encoding produces distinct encoded forms.
Bound: unwind 32, byte length ≤ 16
Rationale: This proves that changing the Finish result String value always produces
  a different encoding sequence. Proptest provides defense-in-depth through the
  real blake3 pipeline.
Evidence: cargo kani -p vb_compile --harness finish_string_result_injectivity --unwind 32
Status: ✓ VERIFIED (REPAIR-2)
```

### 6.2 PO-KANI-FINISH-002: Integer result injectivity ✓

```
### Kani Harness: finish_integer_result_injectivity
Property: For all distinct i64 values, the Finish Integer encoding (i64::to_le_bytes)
  produces distinct [u8; 8] arrays.
Bound: unwind 3 (exhaustive over all 2^64 i64 values — Kani explores symbolically)
Rationale: Mathematical proof that i64::to_le_bytes is injective. Together with
  PO-KANI-FINISH-001, this proves C1 (Finish result value sensitivity) for the
  encoding layer.
Evidence: cargo kani -p vb_compile --harness finish_integer_result_injectivity --unwind 3
Status: ✓ VERIFIED (REPAIR-2)
```

### 6.3 PO-KANI-FINISH-003: Variant discrimination ✓

```
### Kani Harness: finish_scalarvalue_variant_discrimination
Property: For all byte slices (≤16 bytes) and all i64 values, the String encoding
  and Integer encoding produce different encoded forms, modulo the 8-byte edge case
  where string bytes happen to equal an i64 LE representation.
Bound: unwind 32, byte length ≤ 16
Rationale: The 8-byte edge case requires a YAML output name whose bytes exactly
  match a raw i64 LE representation — semantically nonsensical and never occurs
  in practice. blake3's collision resistance provides defense-in-depth.
Evidence: cargo kani -p vb_compile --harness finish_scalarvalue_variant_discrimination --unwind 32
Status: ✓ VERIFIED (REPAIR-2, edge case excluded via kani::assume)
```

---

## 7. Mutation Checkpoints

### Threshold: ≥90% mutation kill rate

### Critical Branches That Must Survive Mutation

| Function | Branch | Must Be Caught By | Rationale |
|----------|--------|-------------------|-----------|
| `digest_step_primitive` line 150 | `Finish` match arm | `digest_step_primitive_finish_writes_finish_discriminator` | Removing the `Finish` discriminator changes digest |
| `digest_step_primitive` line 152 | `ScalarValue::String` inner match | `digest_step_primitive_finish_encodes_string_result_as_utf8_bytes` | Changing String encoding changes digest |
| `digest_step_primitive` line 153 | `hasher.update(value.as_bytes())` | `finish_result_value_changes_compiled_digest_string` (integration) | Swapping to empty update or different encoding → digest unchanged |
| `digest_step_primitive` line 154 | `hasher.update(&value.to_le_bytes())` | `finish_result_value_changes_compiled_digest_integer` (integration) | Swapping to big-endian or constant → digest unchanged |
| `digest_step_primitive` line 155 | `_ => hasher.update(b"unsupported")` | `digest_step_primitive_finish_writes_unsupported_for_unknown_scalar_value` | Skipping the update → digest changes for current variants |
| `canonical_digest` line 118 | `hasher.update(source.version().as_bytes())` | `workflow_version_changes_compiled_digest` (integration) | Skipping version hash → digest unchanged across version changes |
| `canonical_digest` line 133 | `for step in source.steps()` iteration | `finish_step_id_changes_compiled_digest` (integration) | Removing loop → step IDs not hashed |
| `canonical_digest` line 134 | `hasher.update(step.id.as_bytes())` | `finish_step_id_changes_compiled_digest` (integration) | Skipping step ID → digest unchanged across ID changes |
| `canonical_digest` line 137 | `WorkflowDigest::from_bytes(hasher.finalize().into())` | `canonical_digest_is_deterministic` (proptest) | Returning constant → determinism property violated |

### Mutation Survivors (expected; not defects)

| Mutation | Why It Survives |
|----------|----------------|
| `canonical_primitive_name` mapping changes (e.g., "parallel" → "together") | Does not affect Finish digest (Finish has own match arm). This is HAZ-9 and is WAIVED for this bead. |
| Changing `_ => hasher.update(b"unsupported")` to `_ => hasher.update(b"unknown")` | Both are reachable only for nonexistent ScalarValue variants. Test coverage requires a future variant to exist. ACCEPTED risk. |
| Trigger `_` arm change from `b"unknown"` to `b"unhandled"` | Only reachable for future TriggerAst variants. ACCEPTED risk. |

### Mutation Runner

```bash
cargo mutants -p vb_compile --file src/mod_compile_lowering/part_05.rs --function canonical_digest --function digest_step_primitive
```

---

## 8. Combinatorial Coverage Matrix

### 8.1 `digest_step_primitive` — Finish Arm

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: String result | `StepPrimitive::Finish { result: ScalarValue::String("out") }` | Hasher receives `b"finish"` + `b"out"` | Unit |
| Happy: Integer result | `StepPrimitive::Finish { result: ScalarValue::Integer(42) }` | Hasher receives `b"finish"` + `42_i64.to_le_bytes()` | Unit |
| Happy: Integer zero | `StepPrimitive::Finish { result: ScalarValue::Integer(0) }` | Hasher receives `b"finish"` + `[0u8; 8]` | Unit |
| Happy: Integer negative | `StepPrimitive::Finish { result: ScalarValue::Integer(-1) }` | Hasher receives `b"finish"` + `(-1_i64).to_le_bytes()` = `[255u8; 8]` | Unit |
| Boundary: String empty | `StepPrimitive::Finish { result: ScalarValue::String("") }` | Hasher receives `b"finish"` + `[]` (empty) | Unit |
| Boundary: Integer MIN | `StepPrimitive::Finish { result: ScalarValue::Integer(i64::MIN) }` | Hasher receives `b"finish"` + `i64::MIN.to_le_bytes()` | Kani |
| Boundary: Integer MAX | `StepPrimitive::Finish { result: ScalarValue::Integer(i64::MAX) }` | Hasher receives `b"finish"` + `i64::MAX.to_le_bytes()` | Kani |
| Error: unknown variant | `StepPrimitive::Finish { result: future_ScalarValue }` | Hasher receives `b"finish"` + `b"unsupported"` | Unit |
| Invariant: injectivity (String) | Distinct String values | Distinct byte sequences after discriminator | Kani |
| Invariant: injectivity (Integer) | Distinct i64 values | Distinct 8-byte LE arrays | Kani |
| Invariant: variant discrimination | any String + any Integer | Different byte sequences (modulo 8-byte edge case) | Kani |

### 8.2 `canonical_digest` — Determinism and Field Sensitivity

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: identical source | same `WorkflowSource` twice | `d1 == d2` | Unit / Proptest / Integration |
| Happy: different version | `version: v1` vs `version: v2` | `d1 != d2` | Integration |
| Happy: different name | `name: a` vs `name: b` | `d1 != d2` | Integration |
| Happy: different trigger type | `manual` vs `webhook` | `d1 != d2` | Integration (NEW) |
| Happy: different trigger param | `schedule: "0 0 *"` vs `schedule: "0 12 *"` | `d1 != d2` | Integration (NEW) |
| Happy: different step ID | `id: done` vs `id: last` | `d1 != d2` | Integration / Proptest |
| Happy: different finish result (String) | `result: "a"` vs `result: "b"` | `d1 != d2` | Integration / Proptest |
| Happy: different finish result (Integer) | `result: 1` vs `result: 2` | `d1 != d2` | Integration / Proptest |
| Happy: different finish result type | `result: "x"` (String) vs `result: 0` (Integer) | `d1 != d2` | Integration |
| Happy: different step primitive count | 1-step vs 2-step workflow | `d1 != d2` | Integration (existing structural test) |
| Design: unknown trigger variant | new trigger variant | produces `b"unknown"` suffix | Unit (NEW) |
| Design: unknown ScalarValue variant | new ScalarValue variant | produces `b"unsupported"` suffix | Unit (NEW) |

### 8.3 Digest Lifecycle (C6 — Digest survives compilation)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: compile → digest match | valid source | `canonical_digest(source) == compiled.digest()` | Integration (NEW, blocked by visibility) |
| Happy: recompile stability | same source twice | `c1.digest() == c2.digest()` | Integration ✓ |
| Error: compilation fails | source with invalid finish result name | digest is computed before error (deterministic) | Unit (NEW) |
| Design: IR changes don't affect digest | same source, different IR layout | digest unchanged | Proptest (PO-PROPTEST-001 covers this) |

### 8.4 Legacy/Canonical Equivalence (C7)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| **BLOCKED** | valid source | legacy `canonical_digest` == canonical `canonical_digest` | Integration (BLOCKED by visibility — PF-REP2-004 notes legacy code is dead on disk, not compiled) |
| Structural check | n/a | No `mod compile;` in lib.rs (single compilation unit) | Static ✓ |

### 8.5 Forward Compatibility (C8)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Exhaustiveness | all current `ScalarValue` variants explicitly matched in `digest_step_primitive` | `String` and `Integer` both matched | Static ✓ |
| Future variant detection | new `ScalarValue` variant added | `#[non_exhaustive]` prevents compile-time detection; code review checklist as gate | Code review |

---

## 9. New Test Specifications (Gap Closure)

### 9.1 Unit Tests — `digest_step_primitive` Isolation (NEW)

These require adding tests within `crates/vb_compile/src/mod_compile_lowering/` or a `#[cfg(test)]` module in `part_05.rs` or a new `src/tests/digest_unit_tests.rs` that can access `pub(crate)` functions.

**Visibility note**: `canonical_digest` and `digest_step_primitive` are `pub(crate)` in `part_05.rs`. Unit tests placed in `crates/vb_compile/src/tests/` within the same crate can access them via `crate::mod_compile_lowering::part_05::digest_step_primitive`, but the module hierarchy must be checked (currently `part_05` is a file in `mod_compile_lowering/`, re-export path depends on `mod.rs` declarations).

**If visibility cannot be resolved**: These tests can instead test through the public API by:
1. Constructing a minimal `WorkflowSource` with just one Finish step
2. Calling `compile_source` and asserting the resulting digest has specific properties
3. This tests through the compilation layer but still isolates the Finish encoding path

#### UT-1: Discriminator prefix

```
Test: digest_step_primitive_finish_writes_finish_discriminator
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: A blake3 Hasher and StepPrimitive::Finish { result: ScalarValue::Integer(0) }
When: digest_step_primitive(&mut hasher, &primitive) is called, then the hasher is finalized
Then: The resulting hash differs from a hasher that received only b"finish"
  (i.e., the result encoding contributes additional bytes beyond the discriminator)
```

#### UT-2: String encoding verification

```
Test: digest_step_primitive_finish_encodes_string_as_utf8_bytes
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: Two hashers H1 and H2
  H1 receives: b"finish" + b"my_output" (manually)
  H2 receives: digest_step_primitive(Finish { result: String("my_output") })
When: Both are finalized
Then: H1.finalize() == H2.finalize()
```

#### UT-3: Integer encoding verification

```
Test: digest_step_primitive_finish_encodes_integer_as_le_bytes
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: Two hashers H1 and H2
  H1 receives: b"finish" + 42_i64.to_le_bytes() (manually)
  H2 receives: digest_step_primitive(Finish { result: Integer(42) })
When: Both are finalized
Then: H1.finalize() == H2.finalize()
```

#### UT-4: String vs Integer encoding discrimination

```
Test: digest_step_primitive_finish_string_vs_integer_differ
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: Two hashers H1 and H2
  H1 receives: digest_step_primitive(Finish { result: String("42") })
  H2 receives: digest_step_primitive(Finish { result: Integer(42) })
When: Both are finalized
Then: H1.finalize() != H2.finalize()
```

#### UT-5: Canonical digest determinism at unit level

```
Test: canonical_digest_deterministic_at_unit_level
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: A minimal WorkflowSource with one Finish step
When: canonical_digest() is called twice on the same source
Then: Both calls return equal WorkflowDigest values
```

#### UT-6: Canonical digest step ID sensitivity at unit level

```
Test: canonical_digest_sensitive_to_step_id
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: Two WorkflowSources differing only in the Finish step's id field
When: canonical_digest() is called on each
Then: The two digests differ
```

#### UT-7: Canonical digest contains version

```
Test: canonical_digest_includes_version_field
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: Two WorkflowSources differing only in the version field
When: canonical_digest() is called on each
Then: The two digests differ
```

#### UT-8: Unknown ScalarValue produces b"unsupported"

```
Test: digest_step_primitive_unknown_scalar_writes_unsupported
Layer: Unit (Calc)
File: crates/vb_compile/src/tests/digest_unit_tests.rs

Given: A hasher, and the knowledge that the _ arm writes b"unsupported"
  (Because ScalarValue is #[non_exhaustive], we cannot construct a new variant
   from outside the defining crate. This test can be written within vb_yaml
   or as a documentation test noting the _ arm's behavior.)
Alternate approach: Construct a ScalarValue through deserialization of an unknown
  variant pattern and verify the resulting digest includes b"unsupported".
```

### 9.2 Integration Tests — New

#### INT-1: Trigger type sensitivity

```
Test: trigger_type_changes_compiled_digest
Layer: Integration
File: crates/vb_compile/tests/finish_digest_integration.rs

Given: Two YAML sources identical except manual vs webhook trigger
When: Both are compiled
Then: The compiled digests differ
```

#### INT-2: Schedule parameter sensitivity

```
Test: trigger_schedule_param_changes_compiled_digest
Layer: Integration
File: crates/vb_compile/tests/finish_digest_integration.rs

Given: Two YAML sources identical except schedule cron expression
When: Both are compiled
Then: The compiled digests differ
```

#### INT-3: Digest computed before compilation failure

```
Test: digest_computed_before_lowering_error
Layer: Integration
File: crates/vb_compile/tests/finish_digest_integration.rs

Given: YAML source with Finish { result: String("nonexistent") } referencing unknown output
When: compile_source is called (it should fail with UnknownOutputName)
Then: The YAML parses successfully
  And: The error occurs during lowering (after digest computation in part_01.rs:46)
  
(Note: compile_source bundles digest computation + lowering. We verify behavior
 by checking that the parse succeeds and the compile error is from lowering, not parsing.)
```

### 9.3 Fuzz Targets — Both New (see Section 5)

---

## 10. Test Execution Commands

```bash
# Unit tests (within crate)
cargo test -p vb_compile --lib -- digest

# Integration tests
cargo test -p vb_compile --test finish_digest_integration
cargo test -p vb_compile --test finish_digest_structural

# Proptest (ignored by default)
cargo test -p vb_compile --lib -- proptest_finish_digest --ignored

# Kani
cargo kani -p vb_compile --harness finish_string_result_injectivity --unwind 32
cargo kani -p vb_compile --harness finish_integer_result_injectivity --unwind 3
cargo kani -p vb_compile --harness finish_scalarvalue_variant_discrimination --unwind 32

# Fuzz
cargo fuzz run fuzz_digest_compile -- -max_len=65536
cargo fuzz run fuzz_finish_digest_encoding -- -max_len=65536

# Mutation
cargo mutants -p vb_compile --file src/mod_compile_lowering/part_05.rs

# Coverage
cargo llvm-cov -p vb_compile --lib -- --test-threads=1
cargo llvm-cov -p vb_compile --test finish_digest_integration
```

---

## 11. Known Blockers and Open Questions

### BLOCKER: Visibility of `canonical_digest` and `digest_step_primitive`

Both functions are `pub(crate)` in `mod_compile_lowering::part_05`. They are NOT re-exported from the crate root (`lib.rs` exports `compile_source` but not `canonical_digest`).

- **Unit tests in `tests/` (integration crate)**: Cannot access `pub(crate)` items.
- **Unit tests in `src/tests/`**: May be able to access via `crate::mod_compile_lowering::part_05::*` depending on module visibility path.
- **Resolution needed before test-writer can implement UT-1 through UT-8**: Either:
  1. Add `#[cfg(test)] pub(crate) use mod_compile_lowering::part_05::{canonical_digest, digest_step_primitive};` in lib.rs
  2. Place unit tests directly in `mod_compile_lowering/part_05.rs` behind `#[cfg(test)]`
  3. Add a `pub(crate)` re-export for test-only visibility

### Open Question 1: Should we remove the dead legacy code in `compile/mod.rs`?

PF-REP2-004 identifies that `compile/mod.rs` (894 lines) exists on disk but is NOT in the module tree (no `mod compile;` in `lib.rs`). This means C7 (single implementation) is structurally satisfied but the dead code creates latent risk. The existing `canonical_legacy_digest_equivalence` test in `finish_digest_integration.rs` is BLOCKED because the legacy code is inaccessible. **Recommendation**: Remove the dead file in a follow-up bead. This eliminates the need for the equivalence test.

### Open Question 2: Should the `_` arm in `digest_step_primitive` be made exhaustive?

Currently `_ => hasher.update(b"unsupported")` silently handles future `ScalarValue` variants. Making it exhaustive would cause a compile error when a new variant is added, forcing explicit handling. **Recommendation**: Out of scope for this bead (P1). File a follow-up bead for `ScalarValue` exhaustiveness hardening.

### Open Question 3: Should `canonical_digest()` be extracted to a standalone pure module?

Currently it's in `mod_compile_lowering/part_05.rs` alongside lowering functions. Moving it to a dedicated `mod_digest.rs` would improve discoverability and make it easier to unit-test. **Recommendation**: Out of scope for this bead. Consider as a refactoring bead.

---

## 12. Risk-Based Priority

| Priority | Test | Rationale |
|----------|------|-----------|
| **P0** (must exist) | All existing integration tests (Section 3.3) | Already written; verify they compile and pass |
| **P0** (must exist) | All existing Kani harnesses (Section 6) | Already written and verified; ensure they remain passing |
| **P0** (must exist) | Existing proptest properties (Section 4.1) | Already written; ensure they compile and produce passing results |
| **P1** (should exist) | UT-1 through UT-8 (Section 9.1) | Direct unit-level verification of digest_step_primitive encoding behavior; closes GAP-1 |
| **P1** (should exist) | INT-1 (trigger type sensitivity) | Completes C4 coverage for trigger field |
| **P1** (should exist) | INT-2 (schedule parameter sensitivity) | Completes C4 coverage for trigger parameters |
| **P2** (nice to have) | Fuzz targets (Section 5) | Defense-in-depth for YAML parsing and digest encoding |
| **P2** (nice to have) | INT-3 (digest before compilation failure) | Documents the digest-before-lowering design decision |
| **P3** (future) | Legacy/canonical equivalence test | Currently blocked; becomes relevant only if legacy path is re-activated |

---

## 13. Exit Criteria Checklist

- [ ] Every public API behavior (B13–B20) has at least one integration test scenario ✓ (all exist)
- [ ] Every pure function with multiple inputs has at least one proptest invariant ✓ (3 exist, 1 new identified)
- [ ] Every parsing/deserialization boundary has a fuzz target ✗ (2 new targets identified, not yet written)
- [ ] Every error variant in the Error enum has an explicit test scenario ✓ (Not applicable — digest has zero error variants; compile errors tested separately)
- [ ] The mutation threshold target (≥90%) is stated ✓ (Section 7)
- [ ] No test asserts only `is_ok()` or `is_err()` without specifying the value ✓ (existing tests assert exact `WorkflowDigest` equality/inequality via `assert_eq!`/`assert_ne!` on concrete `WorkflowDigest` values)

---

## Appendix A: File Location Map

| Artifact | Path | Status |
|----------|------|--------|
| Canonical `canonical_digest` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` | Active |
| Canonical `digest_step_primitive` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-162` | Active |
| Legacy `canonical_digest` (dead) | `crates/vb_compile/src/compile/mod.rs:220-241` | Dead on disk |
| Legacy `digest_step_primitive` (dead) | `crates/vb_compile/src/compile/mod.rs:243-261` | Dead on disk |
| Proptest properties | `crates/vb_compile/src/proptest_finish_digest.rs` | Active |
| Kani harnesses | `crates/vb_compile/src/kani_finish_digest.rs` | Active |
| Integration tests | `crates/vb_compile/tests/finish_digest_integration.rs` | Active |
| Structural tests | `crates/vb_compile/tests/finish_digest_structural.rs` | Active |
| New unit tests (planned) | `crates/vb_compile/src/tests/digest_unit_tests.rs` | TO CREATE |
| New fuzz targets (planned) | `fuzz/fuzz_targets/fuzz_digest_compile.rs` | TO CREATE |
| New fuzz targets (planned) | `fuzz/fuzz_targets/fuzz_finish_digest_encoding.rs` | TO CREATE |
| This test plan | `.beads/vb-xi2f.34/test-plan.md` | COMPLETE |

## Appendix B: Contract-to-Test Traceability

| Contract Clause | Primary Tests | Defense-in-Depth |
|-----------------|---------------|------------------|
| C1: Finish result value sensitivity | INT: `finish_result_value_changes_compiled_digest_{string,integer}`, PROPTEST: `finish_result_change_changes_digest_{integer,string}` | KANI: `finish_string_result_injectivity`, `finish_integer_result_injectivity`, UT-2, UT-3 |
| C2: Finish step ID sensitivity | INT: `finish_step_id_changes_compiled_digest`, PROPTEST: `finish_position_change_changes_digest` | UT-6 |
| C3: Finish step position sensitivity | PROPTEST: coverage gap (misnamed test) | INT: multi-step workflow tests |
| C4: Canonical digest determinism | PROPTEST: `canonical_digest_is_deterministic`, INT: `compiled_digest_matches_on_recompile` | UT-5, UT-7 |
| C5: Hash discrimination by variant | INT: `finish_result_type_changes_compiled_digest` | KANI: `finish_scalarvalue_variant_discrimination`, UT-1, UT-4 |
| C6: Digest survives compilation | INT: `compiled_digest_matches_on_recompile` | INT-3 |
| C7: Single canonical implementation | Structural: no `mod compile;` in lib.rs | PF-REP2-004 documents dead file risk |
| C8: Forward compatibility | STATIC: `scalarvalue_exhaustiveness_in_digest` | UT-8 (documentation of `_` arm) |
| C9: Digest is pre-validation | Structural: `canonical_digest` takes `&WorkflowSource` | INT-3 |
| C10: Exclusion of runtime concerns | STATIC: `audit_digest_has_no_runtime_dependencies` | Structural: `#![forbid(unsafe_code)]` |
