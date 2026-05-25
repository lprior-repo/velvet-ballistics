# Hazard Analysis: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## H1: Unchecked Emission in Production Code
- **Severity**: Critical
- **Description**: `compile_source` calls `CompiledWorkflow::from_parts_unchecked(parts)`, bypassing all structural validation.
- **Trigger**: Any compiler bug producing invalid `WorkflowParts` (e.g., out-of-bounds slot index, unreachable node, backward edge).
- **Impact**: Runtime executes corrupt IR; potential logic errors, data corruption, or deterministic panics in safe Rust.
- **Mitigation**: Replace with `try_from_parts`. Map `WorkflowError` to `CompileError::Workflow`. Remove `test-util` from `vb_compile` production deps.
- **Residual Risk**: Compiler bug + `try_from_parts` bug that fails to catch the invalidity. Mitigated by existing Kani/Verus proofs for validation functions.

## H2: test-util Feature Leaked to Production Dependencies
- **Severity**: High
- **Description**: `vb_compile/Cargo.toml` enables `vb_core` feature `test-util` in `[dependencies]`, making `from_parts_unchecked` available in release builds.
- **Trigger**: Future developer accidentally uses `from_parts_unchecked` in new production code.
- **Impact**: Same as H1 — unchecked construction becomes trivially accessible.
- **Mitigation**: Remove `features = ["test-util"]` from `vb_compile/Cargo.toml` `[dependencies]` entry. Move to `[dev-dependencies]` if tests need it.
- **Residual Risk**: Zero if Cargo.toml is fixed; CI lint should enforce no `test-util` in production dep graphs.

## H3: Validation Failure Not Mapped to CompileError
- **Severity**: Medium
- **Description**: After switching to `try_from_parts`, `WorkflowError` must be converted to `CompileError::Workflow`. If the conversion is omitted, the compile function returns a different error type, breaking the API contract.
- **Trigger**: Developer replaces `from_parts_unchecked` with `try_from_parts` but forgets `.map_err` or `?` with `From` impl.
- **Impact**: Type mismatch or unexpected error variant propagation.
- **Mitigation**: `CompileError` already has `Workflow(#[from] WorkflowError)`. Use `?` or `.map_err(CompileError::from)`.
- **Residual Risk**: Compilation failure if type mismatch; caught by Rust compiler.

## H4: Compiler Bug Produces Valid-But-Wrong IR
- **Severity**: Medium
- **Description**: `try_from_parts` checks structural invariants, not semantic correctness. A compiler bug could emit IR that passes validation but executes the wrong logic (e.g., wrong branch target).
- **Trigger**: Logic error in lowering (e.g., `canonical_layout` miscalculation).
- **Impact**: Workflow behaves differently from source YAML intent.
- **Mitigation**: BDD tests, Kani idempotency/parity proofs, property-based round-trip tests. Out of scope for this bead.
- **Residual Risk**: Accepted — structural validation does not prove semantic equivalence.

## H5: Performance Regression from Validation
- **Severity**: Low
- **Description**: `try_from_parts` runs `validate_parts` and `validate_budget`, which are O(nodes + edges). For large workflows, this adds compile-time overhead.
- **Trigger**: Compilation of 10,000+ node workflows.
- **Impact**: Slower compile times.
- **Mitigation**: Validation is already bounded by `ResourceContract::max_steps`. Budget computation is incremental. Benchmark compile latency before/after.
- **Residual Risk**: Negligible; validation cost is linear and dwarfed by YAML parsing.

## H6: Future Regression Re-introducing Unchecked Path
- **Severity**: Medium
- **Description**: A future refactor could re-introduce `from_parts_unchecked` in the compiler or add a new emission site.
- **Trigger**: Refactor of `mod_compile_lowering` or addition of new backend.
- **Impact**: H1 recurs.
- **Mitigation**: 
  - CI lint: grep for `from_parts_unchecked` in `crates/vb_compile/src/` (excluding tests).
  - Code review checklist: all `CompiledWorkflow` construction must use `try_from_parts`.
- **Residual Risk**: Low if lint + review gates are maintained.

## H7: Deserialized Artifact Bypasses Validation
- **Severity**: High
- **Description**: `WorkflowParts` deserialized from storage could be corrupted (disk error, tampering, version mismatch). If deserialized parts are passed directly to runtime without `try_from_parts`, corrupt IR executes.
- **Trigger**: Storage corruption, downgrade attack, serialization format change.
- **Impact**: Runtime executes invalid IR.
- **Mitigation**: Storage admission already calls `try_from_parts` (`vb_storage::admission`). This bead enforces the same policy at compile emission.
- **Residual Risk**: Zero if storage admission gate is preserved.

## Temporal Hazards

### T1: Compilation-to-Runtime Time-of-Check/Time-of-Use
- `ResourceContract` is validated at compile time against actual usage.
- Runtime admission re-checks against deployment policy.
- No TOCTOU if contract is carried immutably with the artifact.

### T2: Feature-Flag Time-of-Use
- `test-util` is compile-time gated. Switching feature flags changes available constructors.
- Hazard: conditional compilation makes `from_parts_unchecked` disappear in some builds but present in others.
- Mitigation: never enable `test-util` in production dependency graphs.
