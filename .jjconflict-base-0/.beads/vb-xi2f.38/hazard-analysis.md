# Hazard Analysis: Digest Covers Collect Semantics (vb-xi2f.38)

## H-1: Incomplete Collect Field Hashing (CRITICAL)

### ID: H-1
### Severity: CRITICAL
### Category: Digest Coverage / Refinement
### Boundary: Pure Core (Boundary 1)

**Description**: `digest_step_primitive` for `StepPrimitive::Collect` only hashes the static string `"collect"` via `canonical_primitive_name`, ignoring the semantically significant fields: `variable`, `source`, `pages`, `items`, and `body`.

**Code Location**:
- `vb_compile/src/mod_compile_lowering/part_05.rs` lines 158–160
- `vb_compile/src/compile/mod.rs` lines 257–259

```rust
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}
```

**Attack Vector**: Author two workflows with identical step IDs but different `Collect` parameters:

```yaml
# Workflow A
- id: step1
  collect:
    variable: x
    source: "list_a"
    pages: 10
    items: 5
    steps:
      - id: body1
        set:
          output: result
          value: "1"

# Workflow B
- id: step1
  collect:
    variable: y
    source: "list_b"
    pages: 999
    items: 100
    steps:
      - id: body2
        finish:
          result: "999"
```

**Expected**: `canonical_digest(workflow_a) != canonical_digest(workflow_b)`
**Actual**: Both produce **identical digests** because only `"step1"` and `"collect"` are hashed.

**Impact**:
1. **Wrong content-addressing**: Storage would store only one artifact for both workflows
2. **Wrong replay**: Replaying workflow B would load workflow A's artifact
3. **Execution divergence**: Runtime executes different collect parameters with same digest identity
4. **Contract breach**: `WorkflowDigest` no longer serves as content-addressed identity

**Proof Seeds**: `H-1-COLLECT-FIELD-HASH-001` through `H-1-COLLECT-FIELD-HASH-005`

**Fix Required**: Add explicit match arm for `Collect` that hashes all 5 fields recursively.

---

## H-2: Same-Risk Pattern for Other Primitives (HIGH)

### ID: H-2
### Severity: HIGH
### Category: Digest Coverage / Refinement
### Boundary: Pure Core (Boundary 1)

**Description**: All `StepPrimitive` variants except `Set` and `Finish` use the catch-all `canonical_primitive_name` path, meaning they only hash the primitive name string without their variant-specific fields.

**Affected Variants**:
| Variant | Fields NOT Hashed | Risk Level |
|---------|------------------|------------|
| `Collect` | variable, source, pages, items, body | **CRITICAL** (confirmed bug) |
| `ForEach` | variable, input, at_once, body | HIGH |
| `Aggregate` | variable, input, initial, body | HIGH |
| `Together` | branches (Vec) | HIGH |
| `Choose` | branches, otherwise | MEDIUM |
| `Repeat` | max_attempts, body | MEDIUM |
| `Do` | action, input | MEDIUM |
| `Save` | value | MEDIUM |
| `Wait` | event, timeout | LOW |
| `Ask` | prompt, timeout | LOW |

**Impact**: Same content-addressing failure for any two workflows differing only in these primitive fields.

**Proof Seeds**: `H-2-FOREACH-FIELD-HASH-001`, `H-2-AGGREGATE-FIELD-HASH-001`, `H-2-TOGETHER-FIELD-HASH-001`

---

## H-3: Digest Collision (LOW)

### ID: H-3
### Severity: LOW (theoretical)
### Category: Bounded State / Arithmetic
### Boundary: Pure Core (Boundary 1)

**Description**: BLAKE3-256 has 256-bit output space. A collision would mean two different workflow sources produce the same `WorkflowDigest`.

**Mathematical Bound**: Probability of collision with n digests ≈ n² / 2²⁵⁶ (birthday paradox)

**Attack Vector**: Adversary attempts to craft two workflows with different execution behavior but identical digests (chosen-prefix collision attack on BLAKE3).

**Feasibility**: Computationally infeasible for 256-bit BLAKE3. Not a practical risk.

**Mitigation**: BLAKE3-256 is a trusted hash function; no custom crypto.

---

## H-4: IR Drift from Lowering Non-Determinism (MEDIUM)

### ID: H-4
### Severity: MEDIUM
### Category: Refinement / Concurrency
### Boundary: Imperative Shell (Boundary 2)

**Description**: `canonical_digest` hashes the YAML AST. `compute_compiled_digest` hashes the serialized `WorkflowParts` (IR). If lowering is non-deterministic (same YAML → different IR on different runs), the two-stage digest system becomes inconsistent.

**Sources of Non-Determinism**:
1. `Vec<StepAst>` iteration order in body hashing
2. `Vec<CompiledNode>` emission order in lowering
3. `serde` serialization order for maps/sets

**Impact**:
- Same YAML source → different artifact digest → storage shows duplicate artifacts for semantically identical workflows
- Cross-run determinism tests (`vb_kyyf_cross_run_determinism`) would fail

**Proof Seeds**: `H-4-IR-DRIFT-001`, `H-4-IR-DRIFT-002`

---

## H-5: Serialization Non-Determinism (MEDIUM)

### ID: H-5
### Severity: MEDIUM
### Category: Refinement / Parser/Codec
### Boundary: Storage (Boundary 3)

**Description**: `compute_compiled_digest` calls `blake3::hash(artifact_bytes)` where `artifact_bytes = postcard::serialize(&workflow_parts)`. If `postcard` serialization is non-deterministic (e.g., due to internal pointer addresses, HashMap iteration, or `Rc`/`Arc` serialization), the same `WorkflowParts` could serialize to different bytes on different runs.

**Evidence**: `crates/vb_compile/src/tests/error_variant_tests.rs` tests `compute_compiled_digest` determinism. If this test passes consistently, serialization is deterministic.

**Impact**: Same IR → different artifact digest → content-addressing breaks

**Proof Seeds**: `H-5-SERIAL-DET-001`

---

## H-6: Collect Runtime Pagination State Corruption (MEDIUM)

### ID: H-6
### Severity: MEDIUM
### Category: Temporal / Concurrency
### Boundary: Runtime (Boundary 4)

**Description**: `CollectPaginationState` tracks cursor position across pages. If state is corrupted (e.g., cursor exceeds limit, page_size changes mid-iteration), the collect loop would behave incorrectly.

**State Fields at Risk**:
- `cursor: u32` — current page number
- `limit: u32` — max pages (from `pages` or default 1)
- `page_size: u32` — items per page (from `items` or default 1)

**Impact**:
- `cursor > limit` → infinite loop or premature termination
- `page_size` changed → wrong number of items per page
- Source list changed mid-iteration → wrong data collected

**Mitigation**: `CollectPaginationState` is stored in Fjall with ACID guarantees; runtime reads validated state.

**Proof Seeds**: `H-6-PAGINATION-STATE-001`

---

## H-7: Two Collect Primitives with Same Step ID (MEDIUM)

### ID: H-7
### Severity: MEDIUM
### Category: Refinement / Parser
### Boundary: Validation (Boundary 0 → 1)

**Description**: Two `Collect` primitives in the same workflow with the same step ID but different parameters. Step ID uniqueness is validated separately. However, if both have identical step IDs, the digest would reflect only one set of parameters (current bug) plus the shared step ID.

**Example**:
```yaml
- id: step1
  collect:
    variable: x
    source: "list_a"
    pages: 10

- id: step1   # DUPLICATE ID — should be rejected by validation
  collect:
    variable: y
    source: "list_b"
    pages: 999
```

**Current Behavior**: Validation rejects duplicate IDs before digest computation. The digest bug only manifests when step IDs are unique but collect params differ.

**Mitigation**: `vb_validate` checks for `DuplicateId` before digest computation.

---

## H-8: Body Steps with Same ID Across Collect Instances (LOW)

### ID: H-8
### Severity: LOW
### Category: Refinement
### Boundary: Pure Core (Boundary 1)

**Description**: Two `Collect` primitives with different parameters but body steps that have identical IDs and identical primitive content. With the bug fixed, the digests would still be different because the parent `Collect` parameters are different.

**Note**: This is NOT a bug; it's correct behavior. The body steps being identical doesn't mean the collects are identical — the parent scope parameters differ.

---

## H-9: Proof Harness Using Hardcoded Collect Data (CRITICAL - GOD RULE VIOLATION)

### ID: H-9
### Severity: CRITICAL (GOD RULE VIOLATION)
### Category: Verification / Proof
### Boundary: Verification Harness

**Description**: A Kani/Verus harness that hardcodes a single `Collect` structure with fixed dummy data to "prove" digest coverage. This proves nothing about the actual implementation's handling of different `Collect` parameters.

**Example of BAD harness**:
```rust
#[kani::proof]
fn prove_collect_digest() {
    let collect = StepPrimitive::Collect {
        variable: "x".to_string(),
        source: "list".to_string(),
        pages: Some(10),
        items: Some(5),
        body: vec![],
    };
    let mut hasher = blake3::Hasher::new();
    digest_step_primitive(&mut hasher, &collect);
    // Proves ONE specific Collect hashes without panic
    // Does NOT prove different Collects produce different digests!
}
```

**Required Approach**: Use `kani::any()` to generate arbitrary `Collect` primitives and prove:
1. `kani::cover!(digest_a != digest_b)` when parameters differ
2. `kani::cover!(digest_a == digest_b)` when parameters identical
3. Property: `digest(step_primitive_a) == digest(step_primitive_b)` implies `a == b`

**God Rule Violated**: "No Hardcoded Kani Shapes" — Kani harnesses MUST NOT hardcode structural inputs

---

## Summary Table

| ID | Severity | Category | Boundary | Description |
|----|----------|----------|----------|-------------|
| H-1 | CRITICAL | Digest Coverage | Pure Core | Collect fields not hashed |
| H-2 | HIGH | Digest Coverage | Pure Core | Other primitives also not hashed |
| H-3 | LOW | Bounded State | Pure Core | BLAKE3 collision (theoretical) |
| H-4 | MEDIUM | Refinement | Shell | Lowering non-determinism |
| H-5 | MEDIUM | Codec | Storage | Serialization non-determinism |
| H-6 | MEDIUM | Temporal | Runtime | Pagination state corruption |
| H-7 | MEDIUM | Refinement | Validation | Duplicate step IDs |
| H-8 | LOW | Refinement | Pure Core | Identical body steps, different parent |
| H-9 | CRITICAL | Proof | Verification | Hardcoded harness data (GOD RULE) |
