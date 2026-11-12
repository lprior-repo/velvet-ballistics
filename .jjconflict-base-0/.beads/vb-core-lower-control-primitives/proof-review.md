# Proof Review: vb-core-lower-control-primitives

**Bead ID**: vb-core-lower-control-primitives
**Workspace**: /tmp/vb-ws/vb-core-lower-control-primitives
**Review Phase**: 6 (Proof Review)
**Reviewer**: proof-reviewer specialist
**Date**: 2026-05-15

---

## STATUS: REJECTED

**Summary**: All 12 proof obligations are either STUB-only (no verification executed), VACUOUS (spec fn returns true), or BLOCKED (tooling cannot run). TLA+ spec fails to parse. No non-STUB evidence exists.

---

## Verifier Tooling Availability

| Tool | Status | Notes |
|------|--------|-------|
| Verus | UNVERIFIED_TOOLING | Dependencies missing (saphyr, vb_core, vb_validate) |
| Kani | UNVERIFIED_TOOLING | Harness requires integration into vb_compile src tree |
| TLA+/TLC | SYNTAX_ERROR | Spec cannot be parsed; `Nat`, `>`, `<`, `..`, `Null` unresolved |
| Clippy | PASS | lib.rs passes with 0 warnings |

---

## Obligation Status

| Obligation ID | Verifier | Artifact | Status | Evidence |
|---|---|---|---|---|
| VERUS-INV-001 | verus | `verification/verus_invariants.vr` | STUB - BOUND_MISMATCH | Precondition `id < u16::MAX - 1` in proof does not match contract PRE-001 "u16::MAX - 1 range" |
| VERUS-INV-002 | verus | `verification/verus_invariants.vr` | STUB - BOUND_MISMATCH | Same as INV-001 |
| VERUS-POST-001 | verus | `verification/verus_postconditions.vr` | VACUOUS | spec fn returns `true // Placeholder` |
| VERUS-POST-002 | verus | `verification/verus_postconditions.vr` | VACUOUS | spec fn returns `true // Placeholder` |
| VERUS-POST-003 | verus | `verification/verus_postconditions.vr` | VACUOUS | spec fn returns `true // Placeholder` |
| VERUS-POST-004 | verus | `verification/verus_postconditions.vr` | VACUOUS | spec fn returns `true // Placeholder` |
| VERUS-POST-005 | verus | `verification/verus_postconditions.vr` | VACUOUS | spec fn returns `true // Placeholder` |
| VERUS-POST-007 | verus | `verification/verus_postconditions.vr` | VACUOUS | spec fn returns `true // Placeholder` |
| VERUS-WAITKIND | verus | `verification/verus_waitkind.vr` | STUB - TRUSTED_BOUNDARY | Proof trusts Rust compiler exhaustiveness, not Verus |
| KANI-OVERFLOW | kani | `verification/kani_lower_control.rs` | STUB - UNVERIFIED | Harness structurally flawed, cannot run in isolation |
| TLA-WF-001 | tla-plus | `specs/ControlLowering.tla` | SYNTAX_ERROR | 59 semantic errors; TLC cannot parse spec |
| CLIPPY-ERR | clippy | `crates/vb_compile/src/lib.rs` | PASS | 0 warnings, 0 errors |

---

## Critical Findings

### Finding 1: VACUOUS PROOFS (LETHAL)
**Severity**: LETHAL
**Obligation IDs**: VERUS-POST-001, VERUS-POST-002, VERUS-POST-003, VERUS-POST-004, VERUS-POST-005, VERUS-POST-007
**Location**: `verification/verus_postconditions.vr` lines 37, 73, 92, 112, 131, 151
**Problem**: All 6 postcondition spec functions return `true // Placeholder` with no actual proof content. These are not proofs — they are comments.
**Evidence**:
```rust
pub spec fn lower_for_each_post(...) -> bool { true // Placeholder }
pub spec fn lower_together_post(...) -> bool { true // Placeholder }
pub spec fn lower_collect_post(...) -> bool { true // Placeholder }
// ... all 6 are identical vacuous stubs
```
**Required Fix**: Replace each `true // Placeholder` with a real Verus spec that captures the postcondition (node count, field values, slot indices). Write proof_fns that verify the postconditions against actual function implementations.

### Finding 2: BOUND MISMATCH IN INVARIANT PROOFS (MAJOR)
**Severity**: MAJOR
**Obligation IDs**: VERUS-INV-001, VERUS-INV-002
**Location**: `verification/verus_invariants.vr` lines 47-74, 98-116
**Problem**: The invariant proofs use `id_val < max_val - 1` which means `id <= u16::MAX - 2`. Contract PRE-001 says "id is a valid StepIdx within u16::MAX - 1 range when id-plus-one is required". The phrase "within u16::MAX - 1 range" is ambiguous — if it means `id <= u16::MAX - 1` then the id+1 would overflow for `id = u16::MAX - 1`. The proof's stricter bound (`id <= u16::MAX - 2`) means the proof does NOT cover all valid inputs per the contract.
**Evidence**:
```rust
pub spec fn lower_repeat_invariant(id: StepIdx) -> bool {
    let id_val = id.as_usize();
    let max_val = u16::MAX as usize;
    id_val < max_val - 1  // Proves id <= u16::MAX - 2
    // Contract PRE-001: id within u16::MAX - 1 range
    // Ambiguity: does "within" mean <= u16::MAX - 1 or < u16::MAX - 1?
}
```
**Required Fix**: Clarify PRE-001 bound and ensure proof matches. If PRE-001 means `id <= u16::MAX - 1`, then `checked_add(1)` would overflow for that boundary case and the function must handle it with error. The proof should show the error path is correctly taken, not that the add succeeds.

### Finding 3: TLA+ SPEC SYNTAX ERROR (LETHAL)
**Severity**: LETHAL
**Obligation ID**: TLA-WF-001
**Location**: `specs/ControlLowering.tla` lines 29-30, 53-54, 63, 75, 88, 101, 114, 126-127
**Problem**: TLC reports 59 semantic errors. `Nat`, `>`, `<`, `..`, `Null` are all unresolved. The spec cannot be model-checked in its current form.
**Evidence**:
```
Unknown operator: `Nat'.
Unknown operator: `>'.
Unknown operator: `<...
Could not find declaration or definition of symbol '..'.
Unknown operator: `Null'.
```
**Required Fix**: The spec needs to either:
1. Import standard TLA+ modules (`EXTENDS Naturals, FiniteSets`) to resolve `Nat`, `..`, `>`, `<`
2. Define `Null` as a model value
3. Use `..` range operator correctly (TLA+ standard is `0..MaxSteps-1` syntax)

### Finding 4: KANI HARNESS STRUCTURAL DEFECTS (MAJOR)
**Severity**: MAJOR
**Obligation ID**: KANI-OVERFLOW
**Location**: `verification/kani_lower_control.rs` lines 41-87, 93-125
**Problem**:
1. Line 51-52: Dead code — `let id = StepIdx::new(kani::any());` is immediately overwritten by `let _id = StepIdx::new(id_val as usize);`
2. Line 59-60: `((id_val as usize) + 100).max(0)` — `.max(0)` is unnecessary and suggests uncertainty about unsigned arithmetic
3. The harness does NOT verify `attempt_slot == id + 1` — it only verifies the function returns `Ok`. The actual value relationship is unverified.
4. `#[kani::unwind(5)]` may be insufficient if the actual execution path is deeper (checked_add → ok_or → SlotIdx::new → building nodes)
**Evidence**:
```rust
let id = StepIdx::new(kani::any());  // Line 51 - DEAD CODE
let _id = StepIdx::new(id_val as usize);  // Line 52 - actual use
// ...
assert!(true, "overflow check passed");  // Line 77 - NOT ACTUALLY CHECKING attempt_slot VALUE
```
**Required Fix**: Remove dead code, remove unnecessary `.max(0)`, add concrete assertion that `attempt_slot.raw_value() == id_val + 1` for lower_repeat.

### Finding 5: VERUS-WAITKIND TRUSTED BOUNDARY EXPANSION (MAJOR)
**Severity**: MAJOR
**Obligation ID**: VERUS-WAITKIND
**Location**: `verification/verus_waitkind.vr` lines 34-40, 66-79
**Problem**: The spec function `waitkind_two_variants()` just returns `true` and the proof relies on "the Rust compiler would catch a third variant". This is not a Verus proof — it's trusting the Rust compiler as the proof authority.
**Evidence**:
```rust
pub spec fn waitkind_two_variants() -> bool {
    // This spec function is a proxy for enum exhaustiveness.
    // In Verus, the match expression is exhaustive by construction.
    // The proof obligation is that any WaitKind value matches one of the two arms.
    true  // NOT A REAL SPEC
}
proof fn waitkind_exhaustiveness_proof()
    ensures waitkind_two_variants() == true  // trivial
{ assert(waitkind_two_variants()); }  // trusts Rust compiler
```
**Required Fix**: Write a real Verus spec that names the two variants and proves exhaustiveness via match exhaustiveness, not via trusting the Rust compiler.

---

## Vacuity Hunt Findings

### Finding 6: ALL VERUS POSTCONDITION SPECS ARE TRIVIAL TAUTOLOGIES
**Obligation IDs**: VERUS-POST-001 through VERUS-POST-007
**Problem**: Each spec fn returns `true` unconditionally — they prove nothing. A proof that always returns true is vacuously correct but provides zero assurance.
**Required Fix**: Each spec fn must encode the actual postcondition being proven.

### Finding 7: VERUS INVARIANTS PROVE TRIVIAL PRECONDITIONS
**Obligation IDs**: VERUS-INV-001, VERUS-INV-002
**Problem**: The spec fns only state `id < u16::MAX - 1` and the proof just shows `checked_add` returns `Some` under that precondition. The proof does not verify the implementation correctly uses `checked_add` — it assumes the implementation.
**Required Fix**: Spec fn should verify the actual implementation path, not just the arithmetic.

---

## Verification Attempts

### Clippy (PASS)
```
Command: cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings
Result: No issues found
```

### TLC (FAILED)
```
Command: tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla
Result: 59 semantic errors; spec cannot be parsed
Errors: Unknown operators Nat, >, <, .., Null
```

### Verus (BLOCKED)
```
Command: verus crates/vb_compile/src/lib.rs
Result: Cannot verify — dependencies missing (saphyr, vb_core, vb_validate)
```

### Kani (BLOCKED)
```
Command: cargo kani --harness kani_lower_control --force-mc-flags
Result: Cannot verify — harness requires integration into vb_compile src tree
```

---

## Required Actions

1. **Write real Verus postcondition proofs** replacing all `true // Placeholder` bodies
2. **Fix TLA+ spec syntax** — add `EXTENDS Naturals, FiniteSets` and define `Null`
3. **Fix Kani harness** — remove dead code, verify `attempt_slot == id + 1` concretely
4. **Clarify PRE-001 bound** and align invariant proofs with contract semantics
5. **Replace VERUS-WAITKIND "proof"** with a real Verus exhaustiveness proof
6. **Integrate Verus annotations** into lib.rs inline (not companion .vr files) for actual verification
7. **Add Kani harness** to vb_compile src tree for actual verification

---

## Proof-Repair-Guide Location

See: `.beads/vb-core-lower-control-primitives/proof-repair-guide.md`
