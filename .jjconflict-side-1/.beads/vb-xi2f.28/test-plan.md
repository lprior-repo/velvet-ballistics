# Test Plan: Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28
**State:** 8 (test-planner)
**Date:** 2026-05-25
**Status:** FINAL

---

## Summary

- **Behaviors identified:** 19
- **Trophy allocation:** 6 unit / 8 integration / 3 proptest / 2 fuzz
- **Proptest invariants:** 7 existing (all PASSING), 2 new recommended
- **Fuzz targets:** 0 existing (this bead), 2 recommended
- **Kani harnesses:** 10 existing (2 VERIFIED, 8 PENDING blake3 blocker)
- **Mutation checkpoints:** 5 critical branches, 90% kill rate threshold

---

## 0. Existing Coverage Inventory (Pre-Plan Baseline)

Before enumerating new tests, these artifacts already exist and must be preserved:

### 0.1 Proptests (crates/vb_compile/tests/proptest_digest_foreach.rs)
All 7 tests pass with 500 cases. Must not regress:

| Test | Obligation | Status |
|---|---|---|
| `proptest_foreach_input_variation_changes_digest` | PO-P-FE-01 | PASS |
| `proptest_foreach_at_once_variation_changes_digest` | PO-P-FE-02 | PASS |
| `proptest_foreach_variable_variation_changes_digest` | PO-P-FE-03 | PASS |
| `proptest_foreach_body_variation_changes_digest` | PO-P-FE-04 | PASS |
| `proptest_foreach_digest_deterministic` | PO-P-FE-05 | PASS |
| `proptest_foreach_nonregression_set_finish` | PO-P-FE-08 H1 | PASS |
| `proptest_foreach_nonregression_set_sensitivity` | PO-P-FE-08 H2 | PASS |

### 0.2 Kani Proofs (crates/vb_compile/src/mod_compile_lowering/kani_proofs/)
10 harnesses across 7 files. 2 delimiter proofs VERIFIED, rest PENDING blake3 InlineAsm blocker:

| File | Harnesses | Status |
|---|---|---|
| `kani_digest_foreach_input.rs` | `kani_foreach_input_reaches_hasher` | PENDING |
| `kani_digest_foreach_at_once.rs` | `kani_foreach_at_once_reaches_hasher` | PENDING |
| `kani_digest_foreach_variable.rs` | `kani_foreach_variable_reaches_hasher` | PENDING |
| `kani_digest_foreach_body.rs` | H1 (Set content), H2 (Finish content), H3 (body count) | PENDING |
| `kani_digest_determinism.rs` | H1 (ForEach determinism), H2 (Set determinism) | PENDING |
| `kani_digest_foreach_at_once_equiv.rs` | H1 (None==Some(1)), H2 (None!=Some(0)) | PENDING |
| `kani_digest_foreach_exhaustive.rs` | H1 (all 4 fields), H2 (arm not fallthrough) | PENDING |
| `kani_digest_foreach_delimiter.rs` | H1 (delimiter not YAML id), H2 (no collision possible) | **VERIFIED** |

### 0.3 Implementation Status (GREEN)
Both copies of `digest_step_primitive` already contain the ForEach arm:
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-172`
- `crates/vb_compile/src/compile/mod.rs:257-271`

Both hash all four ForEach fields with `:` delimiters per contract §2.1. No implementation work needed.

---

## 1. Behavior Inventory

| # | Behavior Description | Contract Clause |
|---|---|---|
| **B1** | `canonical_digest` changes when `ForEach.input` changes | AC-FE-01 |
| **B2** | `canonical_digest` changes when `ForEach.at_once` changes | AC-FE-02 |
| **B3** | `canonical_digest` changes when `ForEach.variable` changes | AC-FE-03 |
| **B4** | `canonical_digest` changes when `ForEach.body` content changes | AC-FE-04 |
| **B5** | `canonical_digest` is deterministic: same source → same digest | AC-FE-05 |
| **B6** | Both compilation paths produce identical digests | AC-FE-06 |
| **B7** | `at_once: None` and `at_once: Some(1)` produce identical digest contribution | AC-FE-07 |
| **B8** | `at_once: None` and `at_once: Some(0)` produce different digest contributions | AC-FE-07 (inverse) |
| **B9** | Non-ForEach primitives (Set, Finish) produce unchanged digests | AC-FE-08 |
| **B10** | ForEach arm hits explicit match arm, NOT the catch-all | INV-FE-01 |
| **B11** | All four ForEach source-level fields (variable, input, at_once, body) are hashed | INV-FE-01 |
| **B12** | Field delimiters prevent boundary collisions | INV-FE-02 |
| **B13** | ForEach with empty body produces a valid, deterministic digest | G-FE-06 |
| **B14** | Changing body step `id` changes digest (step ID sensitivity) | Contract §2.3 (DD) |
| **B15** | Nested ForEach in body steps is recursively hashed | Contract §2.1 (body recursion) |
| **B16** | ForEach with body steps of different primitive types (Set vs Finish) produce different digests | AC-FE-04 |
| **B17** | `at_once: Some(0)` produces different digest from both `None` and `Some(1)` | AC-FE-02 + AC-FE-07 |
| **B18** | Digest computation is infallible (never panics) | Infallibility contract |
| **B19** | Digest is independent of machine word size, process identity, time-of-day | Type contract §1.1 |

---

## 2. Trophy Allocation

| Layer | Count | Description | Rationale |
|---|---|---|---|
| **Static Analysis** | 3 | Compiler-enforced exhaustive match, clippy lints, type system | Already satisfied: Rust destructuring guarantees all 4 ForEach fields are mentioned. Clippy catches unused fields in match arms. `WorkflowDigest` is `Copy` + `#[repr(transparent)]`. |
| **Unit / Calc** | 6 | Pure function tests on `digest_step_primitive`, `canonical_digest` | Currently MISSING. Must test: ForEach arm hit, empty body, known-answer hashes, body step ID sensitivity, fall-through prevention, delimiter injection. |
| **Integration** | 8 | Cross-module tests with real `WorkflowSource` construction, real blake3 | Partially covered by existing proptests. Gaps: nested ForEach, at_once edge cases (Some(0) vs None vs Some(1)), body step type diversity, dual-path equivalence (deferred) |
| **E2E** | 0 | Full YAML → compilation → digest comparison | Covered by existing `v1_primitive_lowering.rs` proptest (digest determinism) and idempotency suite. No new E2E needed; ForEach sensitivity is verified at integration layer. |
| **Proptest** | 9 total (7 existing + 2 new) | Property-based: ForEach field variation, determinism, non-regression | 7 existing verified. 2 new: at_once semantic equivalence (AC-FE-07) and nested body sensitivity. |
| **Fuzz** | 0 existing, 2 recommended | Adversarial byte-level inputs to `canonical_digest` | None exist for this bead. New: `canonical_digest` with arbitrary WorkflowSource values; `digest_step_primitive` with deep/malformed ForEach AST. |
| **Kani** | 10 existing | Formal bounded model checking | 2 VERIFIED (delimiter), 8 PENDING (blake3 InlineAsm). No new Kani harnesses needed; existing ones cover all required proofs. |

### Trophy Ratio

```
         [Static: 3]        ← compiler enforcement
    [Unit/Calc: 6]          ← pure logic, exhaustive combinatorial (NEW)
    [Integration: 8]        ← real dependencies, cross-module (partial + NEW)
    [Proptest: 9]           ← property-based (7 existing + 2 NEW)
    [Fuzz: 2]               ← adversarial input boundaries (NEW)
    [E2E: 0]                ← covered by existing suite
```

**Justification for zero new E2E:** The behavior under test (digest sensitivity to ForEach fields) is a compile-time pure computation. It has no network, storage, or shell boundaries. The idempotency suite (`workspace_tests/idempotency_suite/`) and the `v1_primitive_lowering` proptests already exercise full YAML→compilation→digest pipelines. E2E would add latency without proportional defect detection.

**Justification for new unit tests:** The proptests are integration-level and property-based, but they don't provide known-answer tests or exhaustive edge-case coverage. Unit tests fill this gap with exact assertions on specific ForEach AST configurations.

---

## 3. BDD Scenarios

### 3.1 Unit Tests (New — Not Yet Written)

#### Behavior B1: ForEach.input sensitivity
```
Scenario: canonical_digest differs when ForEach.input changes
Given: Two WorkflowSource values identical except ForEach.input:
       source_a.input = "items_list", source_b.input = "other_list"
When: canonical_digest is called on each
Then: The resulting WorkflowDigest values are not equal
```
**Test function:** `fn foreach_input_variation_changes_digest()`
**File:** `crates/vb_compile/src/tests/foreach_digest_tests.rs` (NEW FILE)
**Layer:** unit

#### Behavior B2: ForEach.at_once sensitivity
```
Scenario: canonical_digest differs when ForEach.at_once changes
Given: Two WorkflowSource values identical except ForEach.at_once:
       source_a.at_once = Some(5), source_b.at_once = Some(10)
When: canonical_digest is called on each
Then: The resulting WorkflowDigest values are not equal
```
**Test function:** `fn foreach_at_once_variation_changes_digest()`
**Layer:** unit

#### Behavior B7: at_once semantic equivalence (None vs Some(1))
```
Scenario: at_once=None and at_once=Some(1) produce identical digest contributions
Given: Two ForEach steps identical in variable, input, body
       foreach_a.at_once = None, foreach_b.at_once = Some(1)
When: digest_step_primitive is called on each with independent hashers
Then: Both hashers finalize to identical bytes
```
**Test function:** `fn foreach_at_once_none_some1_equivalence()`
**Layer:** unit
**Rationale:** PROPTEST GAP. Kani harness exists (PO-K-FE-07) but is PENDING blake3 blocker. A proptest + unit test provide runtime evidence.

#### Behavior B8: at_once semantic inequivalence (None vs Some(0))
```
Scenario: at_once=None and at_once=Some(0) produce different digest contributions
Given: Two ForEach steps identical except at_once:
       foreach_a.at_once = None, foreach_b.at_once = Some(0)
When: digest_step_primitive is called on each with independent hashers
Then: The hashers finalize to different bytes
  And: The difference is because None → 1u32, Some(0) → 0u32
```
**Test function:** `fn foreach_at_once_none_some0_inequivalence()`
**Layer:** unit

#### Behavior B10: ForEach arm hit (not catch-all fall-through)
```
Scenario: digest_step_primitive with ForEach does NOT fall through to catch-all
Given: A ForEach StepPrimitive with all fields populated
When: digest_step_primitive is called
Then: The resulting hasher state contains more bytes than just the name "for_each"
  And: A second hasher fed only b"for_each" produces different final bytes
```
**Test function:** `fn foreach_arm_not_catch_all_fallthrough()`
**Layer:** unit
**Rationale:** PRF-FE-02. Proves ForEach arm actually runs, not the `other =>` catch-all.

#### Behavior B13: Empty body produces valid digest
```
Scenario: ForEach with empty body produces a deterministic digest
Given: A ForEach step with variable="x", input="items", at_once=None, body=[]
When: digest_step_primitive is called twice on the same ForEach
Then: Both calls produce identical hasher final bytes
  And: The digest differs from a ForEach with a non-empty body
```
**Test function:** `fn foreach_empty_body_produces_deterministic_digest()`
**Layer:** unit

#### Behavior B14: Body step ID sensitivity
```
Scenario: Changing a body step's id changes the ForEach digest
Given: Two ForEach steps identical except body[0].id:
       foreach_a.body[0].id = "step_a", foreach_b.body[0].id = "step_b"
When: digest_step_primitive is called on each
Then: The resulting digest differs
```
**Test function:** `fn foreach_body_step_id_variation_changes_digest()`
**Layer:** unit
**Rationale:** GAP. Proptest only varies body primitive content; step ID is a separate contract concern (domain decision DD-02).

### 3.2 Integration Tests (New — Not Yet Written)

#### Behavior B6: Dual-path digest equivalence (AC-FE-06)
```
Scenario: Both compilation paths produce identical digests for identical source
Given: A WorkflowSource with a ForEach step
When: The source is compiled via path A (compile/mod.rs) and path B (part_05.rs)
Then: Both paths produce identical WorkflowDigest values
```
**Test function:** `fn foreach_cross_path_digest_equivalence()`
**File:** `crates/vb_compile/tests/proptest_digest_foreach.rs` (uncomment existing code)
**Layer:** integration proptest
**Status:** DEFERRED — compile/mod.rs not compiled in current crate. Code scaffold exists but is commented out. Need to determine whether to compile path A or waive this test.

#### Behavior B15: Nested ForEach body recursion
```
Scenario: Digest is sensitive to nested ForEach in body steps
Given: Two ForEach steps where body contains a nested ForEach with different content
When: digest_step_primitive is called on each
Then: The resulting digest differs due to recursive body hashing
```
**Test function:** `fn foreach_nested_body_content_changes_digest()`
**Layer:** integration
**Rationale:** GAP. No existing test exercises recursive body hashing. The proptests use Set/Finish body steps only.

#### Behavior B16: Body step primitive type diversity
```
Scenario: Body steps with Set vs Finish produce different digests
Given: Two ForEach steps with identical body structure but different primitive types
       foreach_a.body = [Set{output:"x", value:"1"}]
       foreach_b.body = [Finish{result: Integer(1)}]
When: digest_step_primitive is called on each
Then: The resulting digests differ
```
**Test function:** `fn foreach_body_primitive_type_changes_digest()`
**Layer:** integration

#### Behavior B17: at_once Some(0) distinct from None and Some(1)
```
Scenario: at_once=Some(0) produces digest different from both None and Some(1)
Given: Three ForEach steps differing only in at_once:
       f_none.at_once=None, f_some0.at_once=Some(0), f_some1.at_once=Some(1)
When: digest_step_primitive is called on each
Then: f_none and f_some1 produce identical digests (semantic equivalence)
  And: f_some0 produces a digest different from both
```
**Test function:** `fn foreach_at_once_zero_distinct_from_none_and_one()`
**Layer:** integration

### 3.3 Regression Tests (Existing — Verify Still Passing)

#### Behavior B9: Non-regression Set/Finish
```
Scenario: Set and Finish primitive digests are unchanged after ForEach fix
Given: A WorkflowSource with only Set and Finish steps (no ForEach)
When: canonical_digest is called
Then: The digest matches pre-fix expected behavior
  And: Changing Set.output changes the digest
```
**Test function:** `proptest_foreach_nonregression_set_finish` + `proptest_foreach_nonregression_set_sensitivity`
**Status:** EXISTING, PASSING. Must keep passing.

#### Behavior B5: Determinism preserved
```
Scenario: Digest remains deterministic after ForEach fix
Given: Any WorkflowSource with ForEach steps
When: canonical_digest is called 5 times on the same source
Then: All 5 digests are identical
```
**Test function:** `proptest_foreach_digest_deterministic`
**Status:** EXISTING, PASSING.

### 3.4 Existing Tests to Preserve (NO REGRESSION)

The following existing tests must continue passing after any new tests are added:

| Test | Location | Covers |
|---|---|---|
| `compiled_digest_is_deterministic` | `src/tests/error_variant_tests.rs:765` | `compute_compiled_digest` determinism (different layer, must not break) |
| `different_sources_produce_different_digests` | `src/tests/error_variant_tests.rs:781` | Name-level digest distinctness |
| `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | `tests/v1_primitive_lowering.rs:828` | Cross-primitive digest determinism |
| VB-A001 tests (11 tests) | `tests/vb_a001_for_each_topology.rs` | ForEach topology correctness (zero digest assertions — structural only) |
| 7 proptest tests | `tests/proptest_digest_foreach.rs` | All ForEach sensitivity + determinism + non-regression |
| 10 Kani harnesses | `src/mod_compile_lowering/kani_proofs/` | Formal bounded proofs |

---

## 4. Proptest Invariants

### 4.1 Existing (7 passing — PRESERVE)

| Invariant | Test | Status |
|---|---|---|
| P1: ForEach.input variation → digest change | `proptest_foreach_input_variation_changes_digest` | PASS (500 cases) |
| P2: ForEach.at_once variation → digest change | `proptest_foreach_at_once_variation_changes_digest` | PASS (500 cases) |
| P3: ForEach.variable variation → digest change | `proptest_foreach_variable_variation_changes_digest` | PASS (500 cases) |
| P4: ForEach.body variation → digest change | `proptest_foreach_body_variation_changes_digest` | PASS (500 cases) |
| P5: Digest determinism for ForEach sources | `proptest_foreach_digest_deterministic` | PASS (500 cases) |
| P6: Set/Finish non-regression determinism | `proptest_foreach_nonregression_set_finish` | PASS (500 cases) |
| P7: Set output sensitivity preserved | `proptest_foreach_nonregression_set_sensitivity` | PASS (500 cases) |

### 4.2 New Invariants (Recommended)

```
### Proptest: at_once semantic equivalence (AC-FE-07)
Invariant: For any variable, input, and body, at_once=None and at_once=Some(1)
           produce identical digest contributions.
Strategy: Generate random variable (alphanumeric 1-16 chars), random input
          (alphanumeric 1-16 chars), random body (0-5 Set/Finish steps).
          Vary only at_once between None and Some(1).
Anti-invariant: at_once=Some(0) must NOT produce same digest as None or Some(1).
```

```
### Proptest: Nested ForEach body sensitivity
Invariant: Changing content of a nested ForEach in a body step changes the
           outer ForEach digest.
Strategy: Generate outer ForEach with body containing one nested ForEach.
          Vary the nested ForEach's input or at_once.
Anti-invariant: None — this is a sensitivity property only.
Bound: Nesting depth limited to 2 (outer + 1 nested) for tractability.
```

---

## 5. Fuzz Targets

### 5.1 New Fuzz Target: canonical_digest with adversarial ForEach

```
### Fuzz Target: fuzz_canonical_digest_foreach
Input type: bytes (YAML document containing ForEach construct)
Risk: Digest aliasing — two semantically different ForEach YAML sources
      produce identical digests. This is the exact bug being fixed.
      Also: panic in digest_step_primitive, OOM on massive body, stack
      overflow on deeply nested ForEach, non-UTF8 handling in variable names.
Corpus seeds:
  - Minimal ForEach YAML: for_each { input: x, variable: y, body: [set { output: z, value: 0 }] }
  - ForEach with empty body: for_each { input: x, variable: y, body: [] }
  - ForEach with at_once: for_each { input: x, variable: y, at_once: 5, body: [finish { result: 0 }] }
  - ForEach with nested body: for_each { input: items, variable: item, body: [for_each { ... }] }
  - ForEach with non-ASCII variable: for_each { input: x, variable: "café", body: [] }
Invariant: For two different YAML inputs (diff), canonical_digest returns
           different values (crash-only mode is acceptable if panic-free).
Command: cargo fuzz run fuzz_canonical_digest_foreach
File: fuzz/fuzz_targets/foreach_digest.rs (NEW)
```

### 5.2 New Fuzz Target: digest_step_primitive with arbitrary StepPrimitive

```
### Fuzz Target: fuzz_digest_step_primitive
Input type: bytes (serialized StepPrimitive via arbitrary)
Risk: Panic in catch-all arm with invalid primitive, OOM on deeply recursive
      AST, non-exhaustive match on new StepPrimitive variants.
Corpus seeds:
  - StepPrimitive::ForEach with all field combinations
  - StepPrimitive::Set with empty output/value
  - StepPrimitive::Finish with ScalarValue::String (non-integer result)
Invariant: digest_step_primitive never panics. The function is infallible.
           (crash-only mode)
Command: cargo fuzz run fuzz_digest_step_primitive
File: fuzz/fuzz_targets/foreach_digest.rs (NEW, same file)
```

---

## 6. Kani Verification Harnesses

**Status:** 10 existing Kani harnesses already written. No new Kani harnesses needed.

### 6.1 Existing Harnesses (PRESERVE)

| Harness | File | Property | Bound | Status |
|---|---|---|---|---|
| `kani_foreach_input_reaches_hasher` | `kani_digest_foreach_input.rs` | input.as_bytes() feeds hasher | unwind 4 | PENDING |
| `kani_foreach_at_once_reaches_hasher` | `kani_digest_foreach_at_once.rs` | at_once.to_le_bytes() feeds hasher | unwind 8 | PENDING |
| `kani_foreach_variable_reaches_hasher` | `kani_digest_foreach_variable.rs` | variable.as_bytes() feeds hasher | unwind 4 | PENDING |
| `kani_foreach_body_set_content_reaches_hasher` | `kani_digest_foreach_body.rs` H1 | Body Set content feeds hasher | unwind 6 | PENDING |
| `kani_foreach_body_finish_content_reaches_hasher` | `kani_digest_foreach_body.rs` H2 | Body Finish content feeds hasher | unwind 6 | PENDING |
| `kani_foreach_body_count_reaches_hasher` | `kani_digest_foreach_body.rs` H3 | Empty vs non-empty body difference | unwind 6 | PENDING |
| `kani_foreach_digest_step_deterministic` | `kani_digest_determinism.rs` H1 | ForEach digest_step_primitive deterministic | unwind 5 | PENDING |
| `kani_set_digest_step_deterministic` | `kani_digest_determinism.rs` H2 | Set digest_step_primitive deterministic | unwind 3 | PENDING |
| `kani_foreach_at_once_none_some1_equivalence` | `kani_digest_foreach_at_once_equiv.rs` H1 | None==Some(1) equivalence | unwind 4 | PENDING |
| `kani_foreach_at_once_none_some0_inequivalence` | `kani_digest_foreach_at_once_equiv.rs` H2 | None!=Some(0) inequivalence | unwind 4 | PENDING |
| `kani_foreach_all_fields_hashed` | `kani_digest_foreach_exhaustive.rs` H1 | All 4 fields simultaneously varied | unwind 8 | PENDING |
| `kani_foreach_arm_not_fallthrough` | `kani_digest_foreach_exhaustive.rs` H2 | ForEach arm != catch-all | unwind 3 | PENDING |
| `kani_foreach_delimiter_byte_not_in_yaml_id` | `kani_digest_foreach_delimiter.rs` H1 | 0x3A not in YAML ids | unwind 2 | **VERIFIED** |
| `kani_foreach_delimiter_no_collision_possible` | `kani_digest_foreach_delimiter.rs` H2 | No byte is both delimiter and id char | unwind 2 | **VERIFIED** |
| `kani_foreach_delimiter_prevents_boundary_collision` | `kani_digest_foreach_delimiter.rs` H3 | Boundary collision prevention | unwind 3 | PENDING |

### 6.2 GOD RULE Compliance Status

- **GOD RULE 1 (No Hardcoded Shapes):** COMPLIANT. All harnesses use `kani::any()` + `kani::assume()` for structural inputs.
- **GOD RULE 2 (Bind to Production):** COMPLIANT. All harnesses call `super::super::digest_step_primitive`, the actual production function.
- **Blocked:** 12/14 harnesses blocked by `blake3::Hasher` containing `TerminatorKind::InlineAsm`. Resolution: install Kani 0.54+ with `#[kani::stub]` for blake3 intrinsics.

---

## 7. Mutation Checkpoints

### 7.1 Critical Mutations to Survive

| Mutation Location | Mutation Type | Must Be Caught By | Layer |
|---|---|---|---|
| `ForEach` match arm: `variable.as_bytes()` → removed | Remove statement | `foreach_variable_variation_changes_digest` (proptest) + unit test B3 | Proptest + Unit |
| `ForEach` match arm: `input.as_bytes()` → removed | Remove statement | `foreach_input_variation_changes_digest` (proptest) + unit test B1 | Proptest + Unit |
| `ForEach` match arm: `at_once.unwrap_or(1)` → `at_once.unwrap_or(0)` | Constant substitution | `foreach_at_once_none_some1_equivalence` (unit B7) | Unit |
| `ForEach` match arm: body loop → removed | Remove block | `foreach_body_variation_changes_digest` (proptest) + unit test B14/B16 | Proptest + Unit |
| `ForEach` match arm: `hasher.update(b":variable:")` → `hasher.update(b"variable")` (delimiter removed) | String mutation | `foreach_arm_not_catch_all_fallthrough` (unit B10) or delimiter injection test | Unit |
| Catch-all `other =>` arm: ForEach not explicitly matched → ForEach falls through | Branch removal | `foreach_arm_not_catch_all_fallthrough` (unit B10) | Unit |
| `at_once.unwrap_or(1)` → `at_once.unwrap()` (panic on None) | Method substitution | Proptest + unit test B13 (empty body often uses at_once=None) | Proptest + Unit |
| `hasher.update(&limit.to_le_bytes())` → `hasher.update(&limit.to_be_bytes())` (endianness) | Method substitution | `foreach_at_once_none_some1_equivalence` (unit B7) if run on same platform | Unit |
| Body step `id` hashing → removed | Remove statement | `foreach_body_step_id_variation_changes_digest` (new unit test B14) | Unit |
| Field delimiter between `variable:` and `input:` swapped → boundary collision | Reorder | Delimiter collision prevention (Kani PO-K-FE-10 H3) + unit | Kani + Unit |

### 7.2 Mutation Threshold

**Target:** 90% kill rate minimum on the `digest_step_primitive` function and the ForEach arm specifically.

**Configuration:** `mutants.toml` should include:
```toml
[foreach_digest]
functions = ["digest_step_primitive"]
# Focus: ForEach arm mutations
exclude_mutations = []  # no exclusions
```

**Command:** `cargo mutants -p vb_compile -- --function digest_step_primitive`

---

## 8. Combinatorial Coverage Matrix (Unit Tests)

### 8.1 `digest_step_primitive` ForEach Arm

| Scenario | Input Class | Expected Output | Test Layer | Status |
|---|---|---|---|---|
| Happy path: all fields populated | variable="item", input="items", at_once=Some(3), body=[Set{output:"x",value:"1"}] | Digest differs from catch-all hash of "for_each" | Unit | NEW (B10) |
| Empty body | body=[] | Deterministic digest, differs from non-empty body | Unit | NEW (B13) |
| at_once=None | variable="x", input="y", at_once=None, body=[] | Hashes 1u32.to_le_bytes() | Unit | NEW (B7) |
| at_once=Some(1) | variable="x", input="y", at_once=Some(1), body=[] | Hashes 1u32.to_le_bytes() (same as None) | Unit | NEW (B7) |
| at_once=Some(0) | variable="x", input="y", at_once=Some(0), body=[] | Hashes 0u32.to_le_bytes() (different from None) | Unit | NEW (B8) |
| at_once=Some(u32::MAX) | at_once=Some(4294967295) | Hashes [255,255,255,255] LE; no overflow | Unit | NEW (B17) |
| Non-ASCII variable name | variable="café" | Hashes raw UTF-8 bytes deterministically | Unit | NEW |
| Empty variable name (edge) | variable="" | Hashes empty byte sequence; digest computed | Unit | NEW |
| Single body step | body=[StepAst{id:"s1", Set{output:"a",value:"b"}}] | step.id + recursive digest_step_primitive hashed | Unit | NEW (B14) |
| Body with Finish (non-Set) | body=[StepAst{id:"f1", Finish{result:Integer(42)}}] | Finish content hashed, differs from Set body | Unit | NEW (B16) |
| Body step ID change | body=[StepAst{id:"s1",..}] vs [StepAst{id:"s2",..}] | Digests differ (step ID is hashed) | Unit | NEW (B14) |
| Nested ForEach in body | body=[StepAst{id:"inner", ForEach{..}}] | Recursive hashing includes nested ForEach fields | Integration | NEW (B15) |

### 8.2 `canonical_digest` with ForEach Sources

| Scenario | Input Class | Expected Output | Test Layer | Status |
|---|---|---|---|---|
| Single ForEach step | WorkflowSource with 1 ForEach step | Deterministic digest | Integration | EXISTING (proptest P5) |
| Multiple steps including ForEach | WorkflowSource with Set + ForEach + Finish | Digest changes with any step change | Integration | EXISTING (proptest P4, P7) |
| ForEach step first vs last | ForEach as step[0] vs step[2] | Different digests (step order matters) | Integration | NEW |

### 8.3 Error / Panic-Free Coverage

| Scenario | Input | Expected | Test Layer | Status |
|---|---|---|---|---|
| Infallible: ForEach with all fields | Any valid ForEach | No panic | Proptest + Fuzz | Fuzz NEW |
| Infallible: empty body | body=[] | No panic | Unit | NEW (B13) |
| Infallible: deeply nested body | 3+ levels of ForEach | No panic | Fuzz | Fuzz NEW |
| Infallible: very long variable name | variable: 10KB string | No panic, digest computed | Fuzz | Fuzz NEW |
| Infallible: very large body | body: 1000+ steps | No panic | Fuzz | Fuzz NEW |

---

## 9. Test File Organization

### 9.1 New Files to Create

```
crates/vb_compile/src/tests/foreach_digest_tests.rs   ← NEW: Unit tests (6+ tests)
fuzz/fuzz_targets/foreach_digest.rs                     ← NEW: Fuzz targets (2 targets)
```

### 9.2 Existing Files to Extend

```
crates/vb_compile/tests/proptest_digest_foreach.rs      ← ADD: 2 new proptest invariants (P8, P9)
crates/vb_compile/src/tests/error_variant_tests.rs      ← EXTEND: Add ForEach digest-specific tests
```

### 9.3 Module Registration

`crates/vb_compile/src/tests/mod.rs` must register the new test module:
```rust
#[cfg(test)]
mod foreach_digest_tests;
```

Or include the test functions directly in the existing `error_variant_tests.rs` if file count is a concern.

---

## 10. Open Questions

| # | Question | Impact | Recommendation |
|---|---|---|---|
| **Q1** | Should `compile/mod.rs` (path A) be integrated into the crate build, or should dual-path equivalence (AC-FE-06) be waived? | Medium | Path A appears to be dead code. Either re-integrate it (separate bead) or waive PO-P-FE-06. The code fix was applied to both copies for safety. The commented-out proptest should remain as documentation. |
| **Q2** | Should new unit tests go in a separate file (`foreach_digest_tests.rs`) or extend `error_variant_tests.rs`? | Low | Separate file (`foreach_digest_tests.rs`) for clean organization. Easier to find, maintain, and run in isolation. |
| **Q3** | Should `at_once` semantic equivalence (AC-FE-07) be tested via unit test, proptest, or both? | Low | Both. Unit test for known-answer (None→1, Some(1)→1, Some(0)→0). Proptest for random body configurations. |
| **Q4** | Is fuzzing the digest computation a priority, given it's a pure function with BLAKE3? | Low | Medium priority. The primary risk is panic (not digest collision — BLAKE3 handles that). Fuzz for panic-freedom on adversarial inputs. |
| **Q5** | Should there be an explicit test for the inverse of each sensitivity property? (e.g., "ForEach.input does NOT change when only variable changes") | Low | No. These are covered by: identical inputs → identical digests (determinism proptest P5). Adding explicit inverse tests would be redundant. |
| **Q6** | The kani harness comment notes body steps limited to Set/Finish. Should we expand Kani to cover nested ForEach in body? | Low | No. Nested ForEach in Kani would exceed bounded model checking capacity. Proptest + unit tests provide adequate coverage for recursive body hashing. |

---

## 11. Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario _(19 behaviors, all mapped)_
- [x] Every pure function with multiple inputs has at least one proptest invariant _(7 existing + 2 new recommended)_
- [ ] Every parsing/deserialization boundary has a fuzz target _(2 new fuzz targets recommended, not yet written)_
- [x] Every error variant in the Error enum has an explicit test scenario _(N/A — `canonical_digest` is infallible, no error variants exist)_
- [x] Mutation threshold target (>=90%) stated _(see §7.2)_
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value _(All recommended tests use exact byte/digest comparisons)_
- [x] Implementation already exists in both source files _(no implementation work needed for this bead)_
- [ ] 2 new proptest invariants written and passing
- [ ] 6+ new unit tests written and passing
- [ ] 2 new integration tests (nested ForEach, body type diversity) written and passing
- [ ] All 7 existing proptests still passing (no regression)
- [ ] Fuzz targets created and seeded

---

## 12. Implementation Order (for test-writer)

1. **Create unit test file** `crates/vb_compile/src/tests/foreach_digest_tests.rs` with tests B7, B8, B10, B13, B14, B17
2. **Add proptest for at_once equivalence** (P8) to `proptest_digest_foreach.rs`
3. **Add proptest for nested body** (P9) to `proptest_digest_foreach.rs`
4. **Add integration tests** for nested ForEach (B15) and body type diversity (B16)
5. **Create fuzz targets** in `fuzz/fuzz_targets/foreach_digest.rs`
6. **Run full test suite** to verify no regression: `cargo test -p vb_compile`
7. **Run proptests** with elevated case count: `PROPTEST_CASES=2000 cargo test -p vb_compile --test proptest_digest_foreach`
8. **Run mutation testing** on `digest_step_primitive`: `cargo mutants -p vb_compile -- --function digest_step_primitive`
9. **Verify GOD RULE compliance**: No hardcoded shapes in new test inputs, tests bind to actual production functions
