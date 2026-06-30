# Proof Evidence: vb-core-lower-control-primitives

bead_id: vb-core-lower-control-primitives
phase: 5
updated_at: 2026-05-15T00:00:00Z

## Obligation Status Summary

| Obligation ID | Verifier | Artifact Path | Status | Notes |
|---|---|---|---|---|
| VERUS-INV-001 | verus | `verification/verus_invariants.vr` | STUB_PLANNED | Targets `lower_repeat` id+1 overflow |
| VERUS-INV-002 | verus | `verification/verus_invariants.vr` | STUB_PLANNED | Targets `lower_ask` id+1 overflow |
| VERUS-POST-001 | verus | `verification/verus_postconditions.vr` | STUB_PLANNED | `lower_for_each` returns exactly 2 nodes |
| VERUS-POST-002 | verus | `verification/verus_postconditions.vr` | STUB_PLANNED | `lower_together` returns exactly 2 nodes |
| VERUS-POST-003 | verus | `verification/verus_postconditions.vr` | STUB_PLANNED | `lower_collect` returns exactly 3 nodes |
| VERUS-POST-004 | verus | `verification/verus_postconditions.vr` | STUB_PLANNED | `lower_reduce` returns exactly 3 nodes |
| VERUS-POST-005 | verus | `verification/verus_postconditions.vr` | STUB_PLANNED | `lower_repeat` returns 3 nodes + attempt_slot=id+1 |
| VERUS-POST-007 | verus | `verification/verus_postconditions.vr` | STUB_PLANNED | `lower_ask` returns 2 nodes + resume.id=id+1 |
| VERUS-WAITKIND | verus | `verification/verus_waitkind.vr` | STUB_PLANNED | WaitKind enum dataless 2-variant exhaustiveness |
| KANI-OVERFLOW | kani | `verification/kani_lower_control.rs` | STUB_PLANNED | BMC for id ∈ [0, u16::MAX−1] on id+1 paths |
| TLA-WF-001 | tla-plus | `specs/ControlLowering.tla`, `specs/ControlLowering.cfg` | STUB_PLANNED | Step chain well-formedness invariants |
| CLIPPY-ERR | clippy | `crates/vb_compile/src/lib.rs` | INTEGRATED | Running `cargo clippy -p vb_compile` against existing lib.rs |

## Command Evidence (Planned)

### Verus
**Command**: `verus crates/vb_compile/src/lib.rs`
**Expected evidence**: 0 errors; all spec_fn/proof_fn verified
**Stub evidence**: STUB artifacts written to `verification/verus_*.vr`; actual verification requires annotations merged into lib.rs

### Kani
**Command**: `cargo kani --harness kani_lower_control --force-mc-flags`
**Expected evidence**: No counterexamples for id ∈ [0, u16::MAX−1]
**Stub evidence**: Harness written to `verification/kani_lower_control.rs`; actual verification requires harness added to vb_compile src tree

### TLA+
**Command**: `tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla`
**Expected evidence**: 0 invariant violations; deadlock check passes
**Stub evidence**: Spec written to `specs/ControlLowering.tla`; config written to `specs/ControlLowering.cfg`; actual verification requires these placed in repo specs/

### Clippy
**Command**: `cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings`
**Expected evidence**: 0 warnings/errors; all CompileError variants exhaustive
**Stub evidence**: N/A — this is a command-based gate that runs against existing lib.rs; the CompileError enum already has all variants covered

## Assumptions

1. Verus annotations in `verification/verus_*.vr` will be merged into `crates/vb_compile/src/lib.rs`
2. Kani harness `verification/kani_lower_control.rs` will be added to vb_compile under `#[cfg(kani)]`
3. TLA+ spec `specs/ControlLowering.tla` will be placed in the repo `specs/` directory
4. Rust toolchain with Verus, Kani, and TLA+ toolbox are available at runtime

## Waived Tools

| Tool | Reason |
|---|---|
| Flux | No refinement types in scope; type-state enforced by Rust ADTs |
| Loom | No concurrency in lowering (single-threaded, no spawn/Mutex/atomics) |
| Miri | No unsafe code in `vb_compile` (`#![forbid(unsafe_code)]`) |
| Fuzz | No parser/deserialization in lowering; input is validated StepIdx/SlotIdx |

## Integration Blockers

- **Verus**: lib.rs has no Verus annotations yet; STUB specs written but need inline `verus!{}` blocks
- **Kani**: Harness requires module to be added to vb_compile src tree
- **TLA+**: Spec/cfg must be placed in repo `specs/` directory
