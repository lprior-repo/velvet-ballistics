# Codebase Map: vb-xi2f.35 — Digest Coverage of Resource Contract Semantics

## Scope

This bead addresses the gap where the compiler's canonical digest is **not sensitive to `ResourceContract` properties**. When resource contract limits or taint flags change, the digest stays identical — violating the correctness contract that digests must change when semantic IR properties change.

## Key Finding: Digest Does Not Cover Resource Contract

**Both `canonical_digest()` implementations hash only:** version, name, trigger type, step IDs, and primitive type names/values. No resource contract fields are hashed. `ResourceContract::DEFAULT` is hardcoded everywhere.

## Relevant Source Files

### Digest Computation (TARGET FOR CHANGE)

| File | Symbol | Description |
|------|--------|-------------|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` | `canonical_digest()` | Primary canonical digest computation (for cold-path lowering). Hashes version, name, trigger, step IDs, primitives. **Does NOT hash resource contract.** |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-162` | `digest_step_primitive()` | Step-level hashing: hashes `set` output/value, `finish` result, and primitive names for other types. |
| `crates/vb_compile/src/compile/mod.rs:220-261` | `canonical_digest()` | Duplicate digest computation in the direct compile path. Identical behavior to part_05. |
| `crates/vb_compile/src/compile/mod.rs:243-261` | `digest_step_primitive()` | Duplicate in compile/mod.rs path. |
| `crates/vb_compile/src/mod_compile_core.rs:113-116` | `compute_compiled_digest()` | Public API: blake3 hash of raw source bytes. Not the canonical digest; used for verification. |
| `crates/vb_compile/src/lib.rs:56` | re-export | Re-exports `compute_compiled_digest` as public API. |

### ResourceContract / WorkflowParts (WHERE CONTRACT LIVES)

| File | Symbol | Description |
|------|--------|-------------|
| `crates/vb_core/src/workflow/mod.rs:191-228` | `ResourceContract` (17 fields) | **Canonical** ResourceContract: 17 fields including `max_transitions_per_tick` and `allows_secret_results`. Re-exported by `lib.rs`. |
| `crates/vb_core/src/compiled_workflow.rs:130-163` | `ResourceContract` (15 fields) | **Duplicate type**: MISSING `max_transitions_per_tick` and `allows_secret_results`. Used by `validation/resource.rs`. Divergence risk. |
| `crates/vb_core/src/workflow/mod.rs:274-297` | `WorkflowParts` | Holds `resource_contract: ResourceContract` field (the 17-field version). |
| `crates/vb_core/src/workflow/mod.rs:19-31` | `CompiledWorkflow` | Compiled IR struct with `resource_contract: ResourceContract`. |
| `crates/vb_core/src/workflow/mod.rs:230-252` | `ResourceContract::DEFAULT` | Default resource contract: `max_steps=10000`, `max_slots=1024`, `allows_secret_results=false`, etc. |
| `crates/vb_core/src/lib.rs:108-110` | re-export | Re-exports `ResourceContract`, `WorkflowParts`, `CompiledWorkflow` from `workflow` module. |

### Compilation Entry Points (ALL HARDCODE DEFAULT)

| File | Location | Description |
|------|----------|-------------|
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs:44-57` | `compile_source()` | Sets `resource_contract: ResourceContract::DEFAULT` on WorkflowParts. |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:169-194` | `lower_steps_to_ir()` | Sets `resource_contract: ResourceContract::DEFAULT`. |
| `crates/vb_compile/src/mod_compile_lowering/part_08.rs:91-104` | Lowering step | Sets `resource_contract: ResourceContract::DEFAULT`. |
| `crates/vb_compile/src/compile/mod.rs:95-103` | `compile_source()` (alt path) | Sets `resource_contract: ResourceContract::DEFAULT`. |
| `crates/vb_compile/src/compile/mod.rs:288-313` | `lower_steps_to_ir()` (alt path) | Sets `resource_contract: ResourceContract::DEFAULT`. |
| `crates/vb_compile/src/compile/mod.rs:854-872` | `SlotCompiler::build_parts()` | Sets `resource_contract: ResourceContract::DEFAULT`. |

### YAML Source AST (NO RESOURCE CONTRACT HERE)

| File | Symbol | Description |
|------|--------|-------------|
| `crates/vb_yaml/src/ast/types.rs:12-31` | `WorkflowSource` | Parsed YAML AST. Has version, name, trigger, inputs, vars, secrets, steps, result, examples. **No resource contract fields.** |
| `crates/vb_yaml/src/ast/parse.rs:161-196` | `parse_workflow_from_yaml()` | Parser that produces `WorkflowSource`. Rejects unknown fields against a fixed whitelist: `["version", "name", "when", "inputs", "vars", "secrets", "steps", "result", "examples"]`. |

### Taint-Related Code (RELEVANT TO `allows_secret_results`)

| File | Description |
|------|-------------|
| `crates/vb_compile/src/type_taint.rs` | Type-level taint tracking: validates secret taint propagation, rejects secret leaks in `Finish` results. |
| `crates/vb_compile/src/compile/type_taint.rs` | Duplicate taint validation module in compile path. |
| `crates/vb_compile/src/taint/mod.rs` | Test module for secret taint tests. |
| `crates/vb_compile/src/taint/tests/secret_finish_tests.rs` | Tests for Finish result secret taint rejection. |
| `crates/vb_core/src/value.rs` (likely) | `Taint` enum: `Clean`, `Secret`, `DerivedFromSecret`. |
| `crates/vb_core/src/kani_taint_propagation.rs` | Kani proofs for taint propagation. |
| `crates/vb_core/src/kani_taint.rs` | Additional Kani taint harnesses. |

### Module Structure (Compilation Pipeline)

| File | Description |
|------|-------------|
| `crates/vb_compile/src/lib.rs` | Module declarations; `mod_compile_lowering` as `lwr`, `mod_compile_core` as `core`. |
| `crates/vb_compile/src/mod_compile_core.rs` | Cold compiler facade: `YamlCompiler`, `compile_workflow`, re-exports. |
| `crates/vb_compile/src/mod_compile_lowering/` | 13-part lowering module. `part_01.rs` is entry, `part_05.rs` has digest. |
| `crates/vb_compile/src/compile/mod.rs` | Alternative direct compile module (894 lines). |
| `crates/vb_compile/src/mod_compile_errors/` | Error types. |

## Existing Test Coverage

### Digest Tests
| File | Test | What It Covers |
|------|------|----------------|
| `crates/vb_compile/src/tests/error_variant_tests.rs:681-686` | `workflow_digest_from_bytes_creates_digest` | Smoke test: digest creation. |
| `crates/vb_compile/src/tests/error_variant_tests.rs:764-777` | `compiled_digest_is_deterministic` | Same source → same `compute_compiled_digest` output. |
| `crates/vb_compile/src/tests/error_variant_tests.rs:780-803` | `different_sources_produce_different_digests` | Different source name → different digest. |
| `crates/vb_compile/tests/v1_primitive_lowering.rs:824-834` | `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | Proptest: same source → same digest (64 cases). |
| `crates/vb_core/src/ids/mod.rs:603-621` | `workflow_digest_roundtrip`, `workflow_digest_zero_array` | WorkflowDigest utility tests. |
| `crates/vb_core/src/ids/mod.rs:895-983` | `workflow_digest_equality`, `workflow_digest_inequality`, `workflow_digest_single_byte_difference`, `workflow_digest_hash_consistency` | WorkflowDigest property tests. |

### ResourceContract Tests
| File | Test | What It Covers |
|------|------|-------------|
| `crates/vb_core/src/engine/validate/tests/red_phase_behavior_tests.rs:886-975` | `resource_contract_exceeded` module | Tests ResourceContractExceeded errors for max_steps, max_slots, etc. |
| `crates/vb_core/src/engine/validate/tests/red_phase_behavior_tests.rs:1608-1642` | `validate_resource_contract_never_panics_default`, `validate_resource_contract_is_deterministic` | Kani-like contract validation. |
| `crates/vb_core/tests/section36_mandatory_coverage.rs:493,1187-1327,1851-1881,2142-2549` | Multiple resource contract tests | Validation, construction, boundary values. |
| `crates/vb_core/src/workflow/tests.rs:165-204,685-686,3989-4027,4500-4501` | Various tests using ResourceContract | Test helpers and contract-using tests. |
| `crates/vb_core/src/budget/tests.rs:218-219,1099,1297-1316,3765-3795` | Budget tests with contracts | Budget validation with resource contracts. |

### Key Gap: NO tests verify digest changes when ResourceContract fields change
- No test exists that changes `max_steps`, `max_slots`, `allows_secret_results`, etc., and verifies digest differs.
- All existing digest tests focus on source identity or name changes; none consider resource contract semantics.
- The `compute_policy_digest()` in `vb_storage/src/admission.rs` computes a SEPARATE policy digest for the admission side — it does not validate that the compilation-time digest is sensitive to contracts.

## Cross-Crate Dependencies

```
vb_yaml (AST parsing, re-exports vb_core)
    ↓
vb_compile (digest, lowering, compilation — depends on vb_yaml, vb_core, vb_validate)
    ↓
vb_core (IR types: WorkflowDigest, ResourceContract, CompiledWorkflow, WorkflowParts)
    ↓
vb_storage (admission: compute_policy_digest — SEPARATE from compile-time digest)
    ↓
vb_runtime (execution)
```

### Dependency Graph for This Bead
- `vb_core::workflow::ResourceContract` — the 17-field type (canonical)
- `vb_core::compiled_workflow::ResourceContract` — the 15-field type (duplicate, used by validation)
- `vb_core::ids::WorkflowDigest` — the 32-byte hash wrapper
- `vb_compile::mod_compile_lowering::part_05::canonical_digest()` — primary digest site
- `vb_compile::compile::mod::canonical_digest()` — duplicate digest site
- `vb_compile::mod_compile_lowering::part_01::compile_source()` — compilation entry, hardcodes DEFAULT
- `vb_yaml::ast::WorkflowSource` — YAML AST (no contract fields)
- `vb_yaml::ast::parse::parse_workflow_from_yaml()` — parser whitelist (would need updating if contracts come from YAML)

## Risks

| Risk | Severity | Detail |
|------|----------|--------|
| **Digest orphan** | HIGH | The compile-time digest does not cover ResourceContract semantics. Changing limits or taint flags produces identical digests, breaking the semantic contract. |
| **Duplicate ResourceContract types** | HIGH | `compiled_workflow.rs` has 15-field type; `workflow/mod.rs` has 17-field type (canonical). The 15-field one is used by `validation/resource.rs`. |
| **Two compilation paths** | MEDIUM | `mod_compile_lowering/part_05.rs` and `compile/mod.rs` have nearly identical `canonical_digest()` and `lower_steps_to_ir()` with different code. Changes must be applied to both. |
| **YAML source has no contract fields** | MEDIUM | `WorkflowSource` AST has no resource contract representation. Contracts are currently hardcoded as DEFAULT. If contracts come from YAML in the future, the parser whitelist must be updated. |
| **compute_policy_digest is separate** | MEDIUM | `vb_storage::admission::compute_policy_digest()` computes its own policy digest from the resource contract. This creates a split where compilation-time digest lacks contract sensitivity but runtime admission has its own. |
| **No test coverage for gap** | HIGH | Zero tests verify digest changes when resource contract properties change. |

## Open Questions

1. **Where does the resource contract come from?** Currently hardcoded as DEFAULT everywhere. If it comes from YAML, the parser (`vb_yaml/src/ast/parse.rs`) needs updating. If it comes from a separate config, the compiler API needs a contract parameter.
2. **Which ResourceContract type is canonical?** The 17-field one in `workflow/mod.rs` (re-exported by `lib.rs`) appears to be canonical. The 15-field one in `compiled_workflow.rs` is a divergence that should be resolved.
3. **Should both `canonical_digest()` implementations be unified?** The two paths (`mod_compile_lowering/part_05.rs` and `compile/mod.rs`) duplicate digest logic. Fixing one without the other leaves the gap.

## Recommended Downstream Owners

- **rust-contract**: Model the correct ResourceContract → digest binding
- **proof-planner**: Plan Kani/Verus proof that digest changes when contract fields change
- **holzman-rust**: Implement the fix in both canonical_digest paths
- **test-writer**: Add proptests for digest sensitivity to resource contract changes
