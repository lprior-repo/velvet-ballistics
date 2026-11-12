# Proof-to-Implementation Input: vb-xi2f.4

## Bridge Mapping

This document maps approved proof claims to Rust implementation obligations for the proof-to-implementation bridge (State 7).

### Claim → Source Ref Mapping

| Proof Claim | Source Ref | Obligation |
|-------------|-----------|------------|
| PO-001: compile_source uses try_from_parts | `crates/vb_compile/src/mod_compile_lowering/part_01.rs:57` | Replace `Ok(CompiledWorkflow::from_parts_unchecked(parts))` with `Ok(CompiledWorkflow::try_from_parts(parts).map_err(\|e\| CompileErrors(vec![CompileError::Workflow(e)]))?)` |
| PO-001: compile_source uses try_from_parts | `crates/vb_compile/Cargo.toml:13` | Remove `features = ["test-util"]` from `vb_core` dependency in `[dependencies]` |
| PO-002: compile_source panic-free | `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-58` | Ensure `try_from_parts` error path propagates via `?` without panic |
| PO-003: compile_source validated output | `crates/vb_compile/src/mod_compile_core.rs:30-36` | `YamlCompiler::compile` continues to return `Result<CompiledWorkflow, CompileErrors>` |
| PO-005: Error mapping preservation | `crates/vb_compile/src/mod_compile_errors.rs` | Verify `CompileError::Workflow(#[from] WorkflowError)` exists and is reachable |
| PO-006: Error variant correctness | `crates/vb_core/src/workflow/mod.rs:35` | `try_from_parts` validation logic unchanged |

### Behavior Test Refs

| Test Obligation | Location | Purpose |
|-----------------|----------|---------|
| BT-001: Valid workflow compiles successfully | `crates/vb_compile/tests/` | Regression test that valid YAML still compiles to Ok(CompiledWorkflow) |
| BT-002: Invalid workflow returns typed error | `crates/vb_compile/tests/vb_xi2f_error_variant_proptest.rs` | Verify try_from_parts errors surface as CompileError::Workflow |
| BT-003: No unchecked path in production | CI lint / xtask | grep for from_parts_unchecked in vb_compile/src/ (excluding tests) |

### Refinement Harness Refs

| Harness | Location | Verifier | Maps To |
|---------|----------|----------|---------|
| RH-001: compile_source postcondition | `verification/verus/vb_xi2f_compile_source.rs` | verus | `compile_source` postcondition |
| RH-002: compile_source panic-free | `verification/kani/vb_xi2f_compile_source.rs` | kani | `compile_source` bounded verification |
| RH-003: error mapping spec | `verification/verus/vb_xi2f_error_mapping.rs` | verus | `From<WorkflowError>` implementation |
| RH-004: error variant harness | `verification/kani/vb_xi2f_error_variants.rs` | kani | `try_from_parts` error paths |

### Evidence Commands

```bash
# Static analysis: verify no unchecked path in production code
grep -r "from_parts_unchecked" crates/vb_compile/src/ | grep -v "test" | grep -v "\.rs:.*//.*test"
# Expected: zero matches

# Compile check: ensure vb_compile builds without test-util in prod deps
cargo check --package vb_compile

# Unit test: existing compile tests still pass
cargo test --package vb_compile

# Kani: compile_source panic-free
cd /tmp/opencode/vb-xi2f.4-workspace/repo && TMPDIR="$PWD/target/kani-tmp" env -u RUSTC_WRAPPER rustup run nightly-2026-04-28 cargo kani --package vb_compile --harness kani_compile_source_try_from_parts --quiet

# Kani: error variant correctness
cd /tmp/opencode/vb-xi2f.4-workspace/repo && TMPDIR="$PWD/target/kani-tmp" env -u RUSTC_WRAPPER rustup run nightly-2026-04-28 cargo kani --package vb_core --harness kani_try_from_parts_error_variants --quiet

# Verus: compile_source postcondition
cd /tmp/opencode/vb-xi2f.4-workspace/repo && cargo verus verification/verus/vb_xi2f_compile_source.rs

# Verus: error mapping
cd /tmp/opencode/vb-xi2f.4-workspace/repo && cargo verus verification/verus/vb_xi2f_error_mapping.rs

# Flux: refinement check
cd /tmp/opencode/vb-xi2f.4-workspace/repo && cargo flux --package vb_compile
cd /tmp/opencode/vb-xi2f.4-workspace/repo && cargo flux --package vb_core

# proptest: compile_source validation
cd /tmp/opencode/vb-xi2f.4-workspace/repo && cargo test --package vb_compile --test vb_xi2f_compile_source_proptest

# proptest: error variant coverage
cd /tmp/opencode/vb-xi2f.4-workspace/repo && cargo test --package vb_compile --test vb_xi2f_error_variant_proptest
```

### Regression Prevention

- Add CI lint rule: `from_parts_unchecked` must not appear in `crates/vb_compile/src/` outside `#[cfg(test)]` blocks.
- Add `test-util` to production dependency denylist in `xtask` lint or `moon ci` gate.
- Document in `CONTRIBUTING.md` or compiler README: all `CompiledWorkflow` construction must use `try_from_parts`.
