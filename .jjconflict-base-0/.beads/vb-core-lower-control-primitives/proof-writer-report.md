# Proof Writer Report: vb-core-lower-control-primitives

bead_id: vb-core-lower-control-primitives
bead_title: "compiler: Lower v1 control primitives from YAML AST"
phase: 5
updated_at: 2026-05-15T00:00:00Z
attempt: 1

## Blocker Note

**vb-f04l status**: The lowering functions (`lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`, `lower_repeat`, `lower_wait`, `lower_ask`) already exist in `crates/vb_compile/src/lib.rs` at the source checkout. The proof-obligations.planned.jsonl (created during State 4) assumed vb-f04l had not yet landed. Since the implementation exists, proof artifacts target the SPEC/STUB interface already present in lib.rs and write STUB/skeleton annotations that will integrate with that implementation when the real vb-f04l lands or when Verus annotations are added to lib.rs.

## Artifacts Written

| Obligation ID | Artifact | Path | Status |
|---|---|---|---|
| VERUS-INV-001 | Verus spec snippet | `verification/verus_invariants.vr` | STUB — targets `lower_repeat` id+1 overflow |
| VERUS-INV-002 | Verus spec snippet | `verification/verus_invariants.vr` | STUB — targets `lower_ask` id+1 overflow |
| VERUS-POST-001 | Verus spec snippet | `verification/verus_postconditions.vr` | STUB — `lower_for_each` returns exactly 2 nodes |
| VERUS-POST-002 | Verus spec snippet | `verification/verus_postconditions.vr` | STUB — `lower_together` returns exactly 2 nodes |
| VERUS-POST-003 | Verus spec snippet | `verification/verus_postconditions.vr` | STUB — `lower_collect` returns exactly 3 nodes |
| VERUS-POST-004 | Verus spec snippet | `verification/verus_postconditions.vr` | STUB — `lower_reduce` returns exactly 3 nodes |
| VERUS-POST-005 | Verus spec snippet | `verification/verus_postconditions.vr` | STUB — `lower_repeat` returns exactly 3 nodes + attempt_slot=id+1 |
| VERUS-POST-007 | Verus spec snippet | `verification/verus_postconditions.vr` | STUB — `lower_ask` returns exactly 2 nodes + resume.id=id+1 |
| VERUS-WAITKIND | Verus spec snippet | `verification/verus_waitkind.vr` | STUB — WaitKind enum dataless 2-variant proof |
| KANI-OVERFLOW | Kani harness | `verification/kani_lower_control.rs` | STUB — targets `lower_repeat`/`lower_ask` id+1 paths |
| TLA-WF-001 | TLA+ spec | `specs/ControlLowering.tla` | STUB — step chain well-formedness model |
| TLA-WF-001 | TLC config | `specs/ControlLowering.cfg` | STUB — MaxSteps=10, MaxSlots=20 |
| CLIPPY-ERR | N/A (command-based) | N/A | INTEGRATED — runs `cargo clippy -p vb_compile` |

## Integration Notes

### Verus
- Verus annotations are written as a **separate `.vr` file** (`verification/verus_*.vr`) that can be merged into `lib.rs` or kept as a companion spec module.
- Each spec_fn targets the actual function signature from lib.rs (e.g., `lower_repeat(id: StepIdx, ...) -> Result<Vec<CompiledNode>, CompileError>`).
- The STUB annotations use `verus!{}` blocks with `spec_fn` and `proof_fn` that mirror the real implementation structure.
- **Integration command**: `verus crates/vb_compile/src/lib.rs` (Verus will pick up the .vr companion files).

### Kani
- The harness (`verification/kani_lower_control.rs`) is written as a **new module** that will be added to `crates/vb_compile/src/kani_idempotency_parity.rs` or created as a new file.
- Uses `#[kani::proof]` with `#[kani::unwind(5)]` for the id+1 paths.
- Targets `lower_repeat` and `lower_ask` directly.
- **Integration**: This file should be added to `lib.rs` under `#[cfg(kani)]` or as a new harness file.

### TLA+
- `specs/ControlLowering.tla` models the step chain structure with `steps` and `slots` variables.
- Invariants: NoDuplicateStepIds, ValidOffsets, AskResumeIdCorrect, SlotsRecorded.
- `specs/ControlLowering.cfg` sets `MaxSteps=10`, `MaxSlots=20` and enables deadlock checking.
- **Integration**: Place in repo `specs/` directory and run `tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla`.

### Clippy
- No new artifact; `CLIPPY-ERR` is satisfied by running `cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings` against the existing lib.rs.
- The CompileError enum in lib.rs already has all variants covered in `match` expressions in `code()` and `error_description()` methods.

## Commands That Will Be Run

```bash
# Verus (9 obligations)
verus crates/vb_compile/src/lib.rs

# Kani (1 obligation)
cargo kani --harness kani_lower_control --force-mc-flags

# TLA+ (1 obligation)
tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla

# Clippy (1 obligation)
cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings
```

## Verification Artifacts Not Written (Blocked)

| Obligation | Blocker |
|---|---|
| All Verus (VERUS-*) | Verus annotations require lib.rs to be annotated with `verus!{}` blocks; STUB specs written but need real annotations in lib.rs |
| All Kani | Harness written but needs to be added to vb_compile src tree |
| TLA-WF-001 | TLA+ spec written but needs to be placed in repo specs/ directory |

## Integration When vb-f04l Lands

When vb-f04l lands and the real implementation is stable:
1. Merge `verification/verus_*.vr` contents into `crates/vb_compile/src/lib.rs` (Verus annotations inline)
2. Add `kani_lower_control.rs` to vb_compile src under `#[cfg(kani)]`
3. Copy `specs/ControlLowering.tla` and `specs/ControlLowering.cfg` to the repo specs/ directory
4. Run the full verification command suite
