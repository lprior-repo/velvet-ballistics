# Verifier Lane Matrix: vb-xi2f.36

## Lane Coverage Map

| Proof Obligation | Kani | Verus | Proptest | Miri | TLA+ | Flux | Loom |
|-----------------|------|-------|----------|------|------|------|------|
| PO-01: `is_primitive("together")` | ✅ | ✅ | ✅ | — | — | — | — |
| PO-02: `parse_step_primitive()` matches `together` | ✅ | ✅ | — | — | — | — | — |
| PO-03: `parse_parallel()` returns `Together` | ✅ | ✅ | ✅ | — | — | — | — |
| PO-04: `together: {}` error | ✅ | — | ✅ | — | — | — | — |
| PO-05: `together: { branches: [] }` error | ✅ | — | ✅ | — | — | — | — |
| PO-06: `validate_workflow_schema()` accepts | ✅ | — | ✅ | — | — | — | — |
| PO-07: `STEP_PRIMITIVES` arrays complete | ✅ | — | ✅ | — | — | — | — |
| PO-08: `from_field("together")` | ✅ | ✅ | — | — | — | — | — |
| PO-09: `as_str(Parallel) == "together"` | — | ✅ | — | — | — | — | — |
| PO-10: `lower_together()` → `TogetherStart` | ✅ | — | — | — | — | — | — |
| PO-11: `is_primitive("parallel")` alias | ✅ | ✅ | ✅ | — | — | — | — |
| PO-12: `Together` invariant `branches.len() >= 1` | ✅ | ✅ | — | — | — | — | — |

---

## Tool Versions (Required)

| Tool | Minimum Version | Check Command |
|------|-----------------|--------------|
| `cargo-kani` | 2.0.0 | `cargo kani --version` |
| `cargo-verus` | 1.0.0 | `cargo verus --version` |
| `cargo-miri` | nightly-2024-01-01 | `cargo miri --version` |
| `proptest` | 1.4 | Cargo.lock |

---

## Evidence Commands

### Kani

```bash
# PO-01, PO-02, PO-03: Parse layer
cargo kani --package vb_yaml --harness is_primitive_together --output-format=json > kani-po-01.json
cargo kani --package vb_yaml --harness parse_step_together --output-format=json > kani-po-02.json
cargo kani --package vb_yaml --harness parse_parallel_together --output-format=json > kani-po-03.json

# PO-04, PO-05: Error contract
cargo kani --package vb_yaml --harness together_empty_error --output-format=json > kani-po-04.json
cargo kani --package vb_yaml --harness together_empty_branches_error --output-format=json > kani-po-05.json

# PO-06, PO-07: Validation layer
cargo kani --package vb_validate --harness validate_together_schema --output-format=json > kani-po-06.json
cargo kani --package vb_validate --harness step_primitives_contains_together --output-format=json > kani-po-07.json

# PO-08, PO-10: Compile layer
cargo kani --package vb_compile --harness from_field_together --output-format=json > kani-po-08.json
cargo kani --package vb_compile --harness lower_together_produces_together_start --output-format=json > kani-po-10.json

# PO-11: Backward compat
cargo kani --package vb_yaml --harness is_primitive_parallel_alias --output-format=json > kani-po-11.json

# PO-12: Invariant
cargo kani --package vb_yaml --harness together_invariant_branches_nonempty --output-format=json > kani-po-12.json
```

### Verus

```bash
# PO-01, PO-02: Pure function properties
cargo verus --package vb_yaml verus/is_primitive_proof.rs
cargo verus --package vb_yaml verus/parse_step_proof.rs

# PO-03: Parse property
cargo verus --package vb_yaml verus/parse_parallel_proof.rs

# PO-08: from_field mapping
cargo verus --package vb_compile verus/from_field_proof.rs

# PO-09: as_str property
cargo verus --package vb_compile verus/as_str_proof.rs

# PO-11: Backward compat
cargo verus --package vb_yaml verus/parallel_alias_proof.rs

# PO-12: Type invariant
cargo verus --package vb_yaml verus/together_invariant.rs
```

### Proptest

```bash
# PO-03, PO-04, PO-05: Grammar-based error testing
cargo test --package vb_yaml --test parse_together_errors

# PO-06, PO-07: Schema validation
cargo test --package vb_validate --test together_schema_validation

# PO-11: Backward compat
cargo test --package vb_yaml --test is_primitive_parallel_still_works
```

---

## Lane Decision Rationale

| Lane | Selected | Evidence Requirement |
|------|----------|---------------------|
| Kani | ✅ REQUIRED | All parse/compile paths; harness with arbitrary string and YAML inputs |
| Verus | ✅ REQUIRED | Pure fn proofs for `is_primitive`, `from_field`, `as_str`, invariant |
| Proptest | ✅ REQUIRED | Grammar-based error injection for malformed `together` inputs |
| Miri | ⚠️ DEFENSIVE | No unsafe expected; run standard test suite under miri to catch raw pointer issues |
| TLA+ | ❌ NOT NEEDED | Single-step parse is not a state machine; bounded checkers sufficient |
| Flux | ❌ NOT NEEDED | No dependent types required; Verus covers refinement needs |
| Loom | ❌ NOT NEEDED | No concurrency in parse/compile path |

---

## Waivers

| Lane | Obligation | Waiver Reason | Expiry |
|------|-----------|---------------|--------|
| Miri PO-09 | `as_str` proof | Pure fn, no unsafe, no pointers | N/A |
| TLA+ all | All | Single primitive parse; Kani exhausts the state space | N/A |
