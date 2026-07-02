# Hazard Analysis: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Hazard Classification

Each hazard is classified by: category, behavior-affecting (Y/N), severity (LOW/MEDIUM/HIGH/CRITICAL), reproducibility, and required proof strategy.

---

## H-001: Canonical Digest Orphan — Contract Not Hashed

**Category**: Rust-core invariant / Digest integrity
**Severity**: CRITICAL
**Behavior-Affecting**: YES

**Description**: `canonical_digest()` (in both `part_05.rs:116-138` and `compile/mod.rs:220-241`) hashes only `version`, `name`, `trigger`, `step_ids`, and `step_primitive` values. No `ResourceContract` fields are hashed. Changing any of the 17 contract fields produces the **identical** digest.

**Reproduction**:
```rust
// These produce the SAME digest:
let source = parse("version: v1\nname: test\nsteps: []");
let digest_a = canonical_digest(&source); // contract A: max_steps=100
let digest_b = canonical_digest(&source); // contract B: max_steps=99999
assert_eq!(digest_a, digest_b); // TRUE — BUG!
```

**Impact**:
1. Two workflows with different resource limits have identical digests
2. A workflow with `allows_secret_results=false` and one with `allows_secret_results=true` have identical digests
3. Admission layer cannot distinguish contracts by digest
4. Silent contract substitution is possible

**Required Fix**: Hash all 17 contract fields into the canonical digest.

**Proof Seeds**: PS-001, PS-002, PS-003, PS-004

---

## H-002: Duplicate ResourceContract Types

**Category**: Type-safety / Divergent types
**Severity**: HIGH
**Behavior-Affecting**: YES

**Description**: Two `ResourceContract` types exist:
1. `vb_core::workflow::ResourceContract` — 17 fields (canonical, re-exported by `lib.rs`)
2. `vb_core::compiled_workflow::ResourceContract` — 15 fields (used by `validation/resource.rs`)

Missing from the 15-field duplicate: `max_transitions_per_tick`, `allows_secret_results`

**Impact**:
1. `validation/resource.rs` cannot validate `max_transitions_per_tick` or `allows_secret_results`
2. Code that imports the 15-field type cannot express these two contract dimensions
3. `CompiledWorkflow` in `compiled_workflow.rs` stores the 15-field contract; the 17-field canonical fields are lost at this boundary
4. Runtime code (`chunk_002.rs`) accesses `allows_secret_results` but uses the `CompiledWorkflow::resource_contract()` accessor which returns the **15-field type** (from `compiled_workflow.rs`)

**Wait — runtime impact check**: Looking at `CompiledWorkflow::resource_contract()` in `compiled_workflow.rs:106`, it returns `compiled_workflow::ResourceContract` (15-field, no `allows_secret_results`). But the proptest regression files show `CompiledWorkflow` debug output with `allows_secret_results`. This suggests there may be a type alias or the runtime uses the canonical type. Need to verify which `ResourceContract` the `CompiledWorkflow` actually uses at the storage/admission boundary.

**Required Fix**: Eliminate the 15-field duplicate. All code must use the 17-field canonical type.

**Proof Seeds**: PS-005, PS-006

---

## H-003: DEFAULT Hardcoded at All 6 Entry Points

**Category**: Architecture / Missing API parameter
**Severity**: HIGH
**Behavior-Affecting**: YES

**Description**: Every compilation entry point hardcodes `resource_contract: ResourceContract::DEFAULT`. There is no path for a user to specify a non-default resource contract. Even after fixing the digest to hash the contract, all workflows would still get DEFAULT contracts.

**Locations**:
1. `part_01.rs:54` — `compile_source()`
2. `part_05.rs:189` — `lower_steps_to_ir()`
3. `part_08.rs:103` — `SlotCompiler::build_parts()`
4. `compile/mod.rs:105` — `compile_source()` (alt path)
5. `compile/mod.rs:308` — `lower_steps_to_ir()` (alt path)
6. Additional `SlotCompiler::build_parts()` in `compile/mod.rs`

**Impact**: The whole contract system is inert. No user can set different limits for different workflows. The DEFAULT contract is universally applied.

**Required Fix**: Add a `contract: ResourceContract` parameter (or `Option<ResourceContract>` with DEFAULT fallback) to compilation entry points.

**Proof Seeds**: PS-007

---

## H-004: Taint Flag Silent Matching

**Category**: Security / Semantic gap
**Severity**: HIGH
**Behavior-Affecting**: YES

**Description**: `allows_secret_results` is a behavior-affecting field. When `false`, the runtime rejects secret-tainted answers with `RuntimeError::SecretResultNotAllowed` (`chunk_002.rs:6-8`). Since the canonical digest does not hash `allows_secret_results`, two workflows with different taint policies produce identical digests.

**Impact**: An attacker could compile a workflow with `allows_secret_results=true`, get its digest, then substitute a workflow with `allows_secret_results=false` — and the admission layer would not detect the substitution. Secret data could leak through an answer that was supposed to be allowed, or legitimate secret-tagged answers could be blocked unexpectedly.

**Required Fix**: `allows_secret_results` must be hashed into canonical digest. The 15-field duplicate must be resolved so validation can enforce this field.

**Proof Seeds**: PS-008, PS-009

---

## H-005: Dual Compilation Path Drift

**Category**: Maintainability / Duplicate logic
**Severity**: MEDIUM
**Behavior-Affecting**: YES (if one path is fixed and not the other)

**Description**: Two compilation paths exist with near-identical logic:
- `mod_compile_lowering/part_05.rs`: `canonical_digest()` + `lower_steps_to_ir()`
- `compile/mod.rs`: `canonical_digest()` + `lower_steps_to_ir()` (duplicate)

If the digest fix is applied to only one path, the other path remains broken.

**Impact**: Partial fix creates false confidence. One compilation path produces contract-sensitive digests while the other does not.

**Required Fix**: Either unify both compilation paths into one, or apply the fix to both paths with a shared implementation.

**Proof Seeds**: PS-010

---

## H-006: Missing YAML Contract Parsing

**Category**: Parser boundary / Input validation
**Severity**: MEDIUM
**Behavior-Affecting**: YES (future, once contracts are user-settable)

**Description**: The YAML parser whitelist (`parse.rs:173-175`) does not include `resource_contract`. `WorkflowSource` (`types.rs:12-31`) has no `resource_contract` field. Even if a user writes a resource contract in YAML, it is silently ignored (unknown field rejection).

**Impact**: After adding contract parameter to compilation, users need a way to specify contracts. The YAML layer must be updated to parse and validate contract fields.

**Proof Seeds**: PS-011

---

## H-007: Validation Gap — max_transitions_per_tick

**Category**: Runtime invariant / Budget
**Severity**: HIGH
**Behavior-Affecting**: YES

**Description**: `max_transitions_per_tick` is validated in `vb_core::budget::validate_budget_limit()` (checks `== 0` and `> HARD_MAX_TRANSITIONS_PER_TICK`). However, `validation/resource.rs` uses the 15-field `ResourceContract` which does **not** have `max_transitions_per_tick`. This means the compile-time validation in `validate_resource_contract()` cannot check this field.

**Impact**: A zero or excessively large `max_transitions_per_tick` could pass compile-time validation but fail at budget validation time. The error surface is shifted from compile-time to runtime.

**Proof Seeds**: PS-012

---

## H-008: Zero Digest Sensitivity Test Coverage

**Category**: Test coverage gap
**Severity**: HIGH
**Behavior-Affecting**: YES

**Description**: Zero tests verify that changing `ResourceContract` fields changes the digest. All existing digest tests focus on source identity and determinism. The gap is acknowledged but untested:

- `compiled_digest_is_deterministic` — same source, same digest ✓
- `different_sources_produce_different_digests` — different name, different digest ✓
- No test for: same source, different contract → different digest ✗

**Impact**: The bug can persist indefinitely without being caught by CI.

**Proof Seeds**: PS-013, PS-014

---

## H-009: Policy Digest / Canonical Digest Semantic Split

**Category**: Architectural / Dual identity
**Severity**: MEDIUM
**Behavior-Affecting**: YES

**Description**: `compute_policy_digest()` in admission correctly hashes the resource contract. But the canonical digest does not. This creates a split: admission knows the contract identity, but the compilation-time digest does not. The `AcceptedArtifact` struct carries both `digest` (canonical, contract-blind) and `policy_digest` (contract-aware).

**Impact**: The canonical digest is the primary identity. If it does not cover the contract, then all subsystems that rely on `CompiledWorkflow::digest()` (including admission verification) may operate on incomplete identity.

**Proof Seeds**: PS-015

---

## H-010: Field Name Stability in Hash

**Category**: Digest determinism / Future-proofing
**Severity**: MEDIUM
**Behavior-Affecting**: NO (determinism only)

**Description**: When adding contract fields to the hash, the encoding must be stable. If field names are hashed as strings (e.g., `hasher.update(b"max_steps")`), renaming a field would change the digest even if the semantics are identical. This is acceptable if intentional, but a decision must be made: hash by field order (positional) or field name (nominal).

**Recommendation**: Hash by field name + value (nominal). This prevents accidental reordering from changing semantics while ensuring that intentional field name changes are reflected.

**Proof Seeds**: PS-016

---

## Hazard Summary Table

| ID | Category | Severity | Behavior-Affecting | Fix Priority |
|----|----------|----------|-------------------|--------------|
| H-001 | Digest orphan | CRITICAL | YES | P0 |
| H-002 | Duplicate types | HIGH | YES | P0 |
| H-003 | Hardcoded DEFAULT | HIGH | YES | P1 |
| H-004 | Taint silent match | HIGH | YES | P0 |
| H-005 | Dual path drift | MEDIUM | YES | P1 |
| H-006 | Missing YAML parsing | MEDIUM | YES | P2 |
| H-007 | Validation gap | HIGH | YES | P1 |
| H-008 | No test coverage | HIGH | YES | P0 |
| H-009 | Digest split | MEDIUM | YES | P2 |
| H-010 | Field name stability | MEDIUM | NO | P2 |

## Risk Matrix

```
Likelihood
    ^
    │  H-008   H-001 ● (CRITICAL: active bug, no test coverage)
    │  H-002   H-004 ● (HIGH: duplicate types, taint silent match)
    │  H-003   H-007 ▲ (HIGH: hardcoded DEFAULT, validation gap)
    │  H-005        ■ (MEDIUM: dual path drift)
    │  H-006   H-009 ▼ (MEDIUM: YAML parsing, digest split)
    │  H-010        ○ (MEDIUM: naming stability)
    │
    └──────────────────────────→ Impact
```

**H-001 is the most urgent**: it is a CRITICAL bug that is currently active, has zero test coverage, and can cause silent contract substitution.
