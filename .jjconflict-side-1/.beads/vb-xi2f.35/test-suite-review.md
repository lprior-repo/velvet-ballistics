# Test Suite Review: vb-xi2f.35 — ResourceContract Digest Coverage

**Review type**: Suite review (implementation + tests)
**Test count**: 2747 pass (all-crate), 13,642 `#[test]` annotations across workspace
**Bead**: vb-xi2f.35 — P1: digest covers resource contract semantics
**Artifacts reviewed**: 10 test files in `vb_compile/tests/` and `vb_core/tests/` + inline unit tests in `vb_core/src/contract_encoding.rs`

## Findings (ordered by severity)

---

### CRITICAL: C1 — `is_ok()`/`is_err()`-only assertions in `entry_point_contract_parameter.rs` (lines 41, 57, 295)

**File**: `crates/vb_compile/tests/entry_point_contract_parameter.rs`

Three tests assert only the `Result` discriminant without verifying the value:

- **Line 41**: `compile_source_accepts_contract_parameter()` — asserts `result.is_ok()` only. If `compile_source` succeeds but silently ignores the contract parameter (reverting to hardcoded DEFAULT), this test still passes.
- **Line 57**: `compile_source_accepts_extreme_contract_values()` — asserts `result.is_ok()` only. Same vulnerability.
- **Line 295**: `compile_source_rejects_invalid_source_with_contract_parameter()` — asserts `result.is_err()` only. Does not verify WHICH error variant. If the error changes from `CompileErrors::EmptySteps` to a different error type, this test still passes, missing a regression.

**Rubric**: Gate 3 — "`is_ok()`, `is_err()` ... are lethal unless the contract is explicitly boolean." The contract for these behaviors requires proof of contract preservation (C1, C3), not mere success. These tests survive deletion of the entire contract parameter plumbing.

**Fix required**: The first two tests (lines 32–60) are redundant with `compile_source_preserves_non_default_contract_after_compilation()` (line 85) which does assert `resource_contract() == contract`. Replace the two `is_ok()`-only tests with the stronger assertions already present at lines 66–100. The third test (line 287) should assert the exact `WorkflowError` variant for empty steps (or verify a specific `CompileErrors` discriminant).

---

### CRITICAL: C2 — KAT test does not assert a golden hash value

**File**: `crates/vb_compile/tests/contract_digest_binding.rs`, lines 350–372
**Test**: `canonical_digest_known_answer_for_default_contract()`

The test is named as a Known Answer Test and its doc-comment says "Any change to `ResourceContract::DEFAULT` or `canonical_digest` must update this golden value." However, the test body only asserts:
1. The digest is 32 bytes (line 362)
2. Determinism (line 364–372)

There is **no assertion of a specific expected hex value**. The test will pass silently if `ResourceContract::DEFAULT.max_steps` is changed from 10,000 to 9,999. The mutation "change a DEFAULT constant value" is an acknowledged survivor risk in the test plan (§7, line 1059).

**Rubric**: Gate 6 — "Mutation thought experiment: deleting branch/error/value logic must be caught by a named test." The KAT's own stated purpose (catching DEFAULT changes) is not fulfilled.

**Fix required**: Compute the actual 32-byte digest for the canonical `representative_source()` with `ResourceContract::DEFAULT` and add a hardcoded assertion:
```rust
assert_eq!(
    digest.as_bytes(),
    &[0x00, 0x01, /* ... */ 0x1F],
    "Golden hash for DEFAULT contract must match; update if contract changes"
);
```

---

### HIGH: H1 — Dual-path equivalence test is a determinism test, not a dual-path test (PF-BR-001)

**Files**:
- `crates/vb_compile/tests/contract_digest_binding.rs`, lines 323–344: `canonical_digest_is_deterministic_across_multiple_computations()`
- `crates/vb_compile/tests/proptest_dual_path_equivalence.rs`, lines 38–71: `proptest_dual_path_digest_equivalence()`

Both tests call `compile_source()` twice and compare outputs — they verify **determinism** (same function, repeated calls). Neither test calls `part_05::canonical_digest()` and `compile::mod::canonical_digest()` independently and compares them. The test-plan (§3.6, Behavior F1, line 687) explicitly requires:

```rust
let digest_part05 = vb_compile::mod_compile_lowering::part_05::canonical_digest(&source, contract);
let digest_compile_mod = vb_compile::compile::mod::canonical_digest(&source, contract);
assert_eq!(digest_part05, digest_compile_mod);
```

**Current state**: Both files document this limitation in comments ("Only one compilation path is active") and defer true dual-path testing until `compile/mod.rs` is activated.

**Mitigation**: The test-plan notes that `compile/mod.rs` is not currently activated in the module tree. If this is a genuine dead code path, the risk is lower. However, the test files are NAMED for dual-path equivalence and the proptest obligation PO-P04 is assigned to this gap. The test names are misleading — they claim coverage they don't provide.

**Fix required**: Either (a) activate `compile/mod.rs` and implement true dual-path calls, or (b) rename the tests to accurately reflect what they verify ("determinism"), and update the proptest obligation to reflect the actual coverage status.

---

### HIGH: H2 — `compile_source_with_default` API missing; equivalence test is vacuous (PF-BR-002)

**Files**:
- `crates/vb_compile/tests/proptest_with_default_equivalence.rs`, lines 34–54: `proptest_with_default_equivalence()`
- `crates/vb_compile/tests/entry_point_contract_parameter.rs`, lines 227–247: `compile_source_with_explicit_default_is_deterministic()`

The test-plan (§3.3, Behavior C5) requires testing that `compile_source_with_default(source)` produces the same digest as `compile_source(source, ResourceContract::DEFAULT)`. Since `compile_source_with_default` does not exist as a public API, neither test calls it. Instead, both tests call `compile_source(source, DEFAULT)` twice — another determinism test.

**Fix required**: Implement `compile_source_with_default(source)` and write the actual equivalence test. This is a known bridge finding (PF-BR-002) documented in STATE.md.

---

### MEDIUM: M1 — Proptest encoding injectivity only randomizes 3 of 17 fields (PF-BR-003)

**File**: `crates/vb_compile/tests/proptest_contract_field_sensitivity.rs`, lines 430–457
**Test**: `proptest_encoding_injectivity_for_distinct_contracts()`

This proptest generates random contracts and verifies injectivity, but only randomizes 3 fields: `max_steps`, `max_slots`, and `max_constants` (×2 for each of the two contracts). The test-plan (PI-02, line 840) calls for "full 17-field randomization." A mutation that causes two contracts to collide in the encoding ONLY when a non-randomized field (e.g., `max_blob_bytes`) differs would survive this proptest.

**Note**: The per-field sensitivity proptests (lines 48–403) cover all 17 fields individually, but the injectivity test (which catches multi-field collision bugs) has reduced coverage. The `proptest_multi_field_differs` test (line 462) randomizes 8 fields for contract_a but uses a fixed contract_b.

**Fix required**: Extend injectivity proptest to randomize more fields (ideally all 17) for both contracts. The test-plan PI-02 target is ≥5000 random pairs with full field randomization.

---

### MEDIUM: M2 — Fault injection gap: no behavior test for `compile_source` silently ignoring the contract parameter

**Files**: All integration tests pass contract to `compile_source`, but no test explicitly verifies that the old API signature `compile_source(source)` — without contract — fails to compile or is not callable.

**Context**: Contract Clause 3 requires "all 6 hardcoded DEFAULT locations are removed." The type system now requires a contract parameter (the two-argument `compile_source(source, contract)` is the only public API). Static analysis covers this. No additional behavior test is strictly needed if the single-argument version no longer exists.

**Verdict**: Acceptable — the static analysis layer (trophy allocation layer) covers this. No behavior test gap.

---

### LOW: L1 — Three determinism proptests with overlapping coverage (PF-BR-005)

**Files**:
- `proptest_dual_path_equivalence.rs` — PO-P04: determines determinism (calls compile_source twice)
- `proptest_digest_determinism.rs` — PO-P05: tests determinism (calls compile_source twice)
- `proptest_with_default_equivalence.rs` — PO-P06: tests determinism with DEFAULT (calls compile_source twice)

All three files test the same invariant: `compile_source(source, contract)` × 2 produces identical digests. The proptest inputs differ (different field sets randomized, DEFAULT vs non-DEFAULT), but the invariant is the same. Consolidation into a single proptest covering all input distributions is recommended but not blocking.

---

### LOW: L2 — Validation test does not verify `allows_secret_results` is validated for consistency (AC-5.2)

**File**: `crates/vb_core/tests/resource_contract_validation.rs`

Contract Clause C5, AC-5.2 states: "`allows_secret_results` is validated for consistency (valid bool)." Since `bool` is type-safe, a `bool` is always valid. The validation test (`resource_contract_preserves_allows_secret_results_field`) verifies preservation through `try_from_parts`, which is covered. No additional validation test is needed.

**Verdict**: Acceptable — type system guarantees validity.

---

### LOW: L3 — Multi-field proptest randomizes only 8 of 17 fields for contract_a

**File**: `crates/vb_compile/tests/proptest_contract_field_sensitivity.rs`, lines 462–487
**Test**: `proptest_multi_field_differs()`

Contract_a randomizes 8 fields (max_steps, max_slots, max_constants, max_accessors, max_expressions, max_expr_stack, max_step_budget_per_tick, max_transitions_per_tick) but leaves 9 fields at DEFAULT. This is a coverage reduction but the per-field tests cover all 17 fields individually.

---

## Positive Findings

### What passes review:

1. **No ignored tests or sleeps**: Zero `#[ignore]` or `thread::sleep` in bead-scope test files. Deterministic execution.

2. **No shared mutable state**: Zero `static Mutex`, `RefCell`, `lazy_static`, or `once_cell` in test files. Hermetic tests.

3. **Integration tests use public API**: All tests call `compile_source()`, `vb_yaml::parse_workflow_source()`, `encode_contract_bytes()`, or `CompiledWorkflow::try_from_parts()`. No `pub(crate)` internals accessed from integration tests.

4. **Exact error variant assertions** (where used): `resource_contract_validation.rs` asserts exact `WorkflowError::ResourceContractExceeded { resource: "..." }` variants with specific resource identifiers. Lines 111–288 exhaustively cover E1–E6 with boundary cases (exact-at-limit OK, exceed-limit ERROR).

5. **Complete encoding test suite**: `contract_encoding.rs` (lines 89–456) has thorough unit tests covering determinism (I1), ordered field tags (I2), little-endian encoding (I3), unique domain tags (I4), injectivity (I5), extreme values/no-panic (I6), plus tag prefix collision prevention. These are well-designed and mutation-resistant.

6. **Type integrity tests**: `resource_contract_type_integrity.rs` verifies all 17 fields via struct literal (compile-time assertion), roundtrip preservation, Copy trait, and DEFAULT reasonableness. Strong static + behavior coverage for Clause C2.

7. **Runtime enforcement tests**: `chunk_007.rs` (line 63–89) tests `SecretResultNotAllowed` enforcement with a real Shard and tainted answer. This covers D4 behavior.

8. **Per-field proptest coverage**: `proptest_contract_field_sensitivity.rs` has individual proptest blocks for all 17 fields plus `allows_secret_results` toggle, providing exhaustive randomized coverage for PI-01.

9. **Deterministic proptests**: All proptest blocks use seeded randomization via the `proptest!` macro. Regressions are tracked in `.proptest-regressions` files.

10. **No `unwrap`/`expect`/`panic`/`unsafe` in test bodies beyond test-only YAML parsing** (acceptable for test fixtures). The test fixtures use `.expect("valid YAML source")` which is idiomatic for test setup.

---

## Coverage Summary vs Contract

| Clause | Behaviors | Test Coverage | Gap |
|--------|-----------|:---:|---|
| C1: Digest-Contract Binding | A1–A8 | ✅ Covered (unit + proptest + integration) | KAT lacks golden hash (C2) |
| C2: Single Canonical Type | B1–B5 | ✅ Covered (static + integration) | None |
| C3: Entry Point Contract | C1–C6 | ⚠️ Partial | C1/C5 tests use `is_ok()` only (C1); C5 API missing (H2); C4 dual-path tests are determinism-only (H1) |
| C4: Taint Flag Sensitivity | D1–D5 | ✅ Covered (unit + proptest + runtime) | None |
| C5: Full Validation | E1–E11 | ✅ Covered (integration) | None (all 6 primary exceed dimensions + hard limits) |
| C6: Dual Path Consistency | F1–F3 | ❌ Not actually tested | Tests named "dual-path" verify determinism (H1) |
| C7: YAML Parsing | G1–G5 | N/A (P2 deferred) | Accepted via waiver WC-001 |
| C8: Backward Compatibility | H1–H3 | ✅ Documented | One-time migration noted |
| C9: Proof Obligation | Kani+Proptest | ⚠️ Kani pending CI; proptest coverage gap (M1) | Kani harnesses written, toolchain not available |
| C10: Non-Requirements | — | ✅ Confirmed out of scope | — |

---

## Mutation Resistance Check

The test-plan mutation matrix (§7) identifies 18 critical mutations. Quick audit against actual test coverage:

| Mutation | Caught by named test? |
|----------|:---:|
| Delete a field tag from encoding | ✅ `encode_contract_bytes_contains_all_17_field_tags_in_order` |
| Change endianness of one field | ✅ I3 LE tests |
| Remove `allows_secret_results` from encoding | ✅ I2 + D1 toggle + proptest |
| Swap order of two field tags | ✅ I2 (order check) |
| Use same tag for two fields | ✅ I4 (unique tag check) |
| Delete struct field (`allows_secret_results`) | ✅ B1 (compile-time: struct literal fails) |
| Hardcode DEFAULT in compile_source | ⚠️ C1 `is_ok()`-only tests survive (C1) |
| Skip `validate_budget` call | ✅ E9 (budget enforced through try_from_parts) |
| Skip `validate_resource_contract` call | ✅ E1–E6 (per-field exceeded tests) |
| Change DEFAULT constant value | ❌ KAT doesn't assert golden hash (C2) |

---

## Verdict

**STATUS: REJECTED**

**Rejection rationale**: 2 CRITICAL findings and 2 HIGH findings must be remediated before this test suite can be approved. The CRITICAL findings are:

1. **C1**: Three tests in `entry_point_contract_parameter.rs` use `is_ok()`/`is_err()`-only assertions, violating the lethal assertion rule of the test rubric. These tests would survive deletion of the contract parameter plumbing.
2. **C2**: The Known Answer Test `canonical_digest_known_answer_for_default_contract()` does not assert a specific golden hash value, making it unable to detect silent changes to `ResourceContract::DEFAULT`.

The HIGH findings (H1: dual-path test mislabeling, H2: missing `compile_source_with_default` API) are known bridge gaps documented in STATE.md. While they are deferred/blocked, they must not be silently accepted as covered by existing tests.

### Blocking remediation:

1. Replace `is_ok()`/`is_err()`-only assertions in `entry_point_contract_parameter.rs` with exact value/error variant assertions
2. Add a hardcoded 32-byte golden hash assertion to the KAT test in `contract_digest_binding.rs`
3. Either implement the true dual-path test (F1) and `compile_source_with_default` API, or rename the existing determinism tests to avoid mislabeling and update proptest obligation metadata

### Non-blocking (for follow-up bead):

- Extend proptest encoding injectivity to full 17-field randomization (M1)
- Consolidate determinism proptests (L1)
- Execute 14 Kani harnesses when Kani toolchain is available on CI
