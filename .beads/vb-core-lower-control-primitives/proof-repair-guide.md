# Proof Repair Guide: vb-core-lower-control-primitives

**Bead ID**: vb-core-lower-control-primitives
**Workspace**: /tmp/vb-ws/vb-core-lower-control-primitives
**For State**: Return to State 5 (Proof Writing)
**Priority**: LETHAL — 6 vacuous proofs, 1 syntactically invalid TLA+ spec

---

## Critical Path (Fix in Order)

### 1. TLA+ Spec Syntax (LETHAL — blocks all TLA+ verification)

**File**: `specs/ControlLowering.tla`

**Problem**: TLC cannot parse — missing `EXTENDS`, unresolved operators.

**Required Fix**:
```tla
---- MODULE ControlLowering ----
EXTENDS Naturals, FiniteSets

CONSTANT
    MaxSteps,
    MaxSlots

ASSUME MaxSteps \in Nat /\ MaxSteps > 0
ASSUME MaxSlots \in Nat /\ MaxSlots > 0

\* Define Null as a model value
Null == CHOOSE x : x \notin Nat

\* Fix range syntax: use .. from Naturals extension
StepIds == 0..MaxSteps-1
SlotIds == 0..MaxSlots-1

\* Fix Null usage: output field should be \in SlotIds \union {Null}
```
Also fix all comparison operators — in TLA+ standard module, `>` and `<` are valid when using Naturals.

**Verification**: Run `tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla` — must report 0 errors.

---

### 2. Vacuous Verus Postcondition Proofs (LETHAL — 6 proofs do nothing)

**File**: `verification/verus_postconditions.vr`

**Problem**: All 6 `spec fn` bodies return `true // Placeholder`. These prove nothing.

**Required Fix for each obligation**:

#### VERUS-POST-001 (lower_for_each)
Replace:
```rust
pub spec fn lower_for_each_post(...) -> bool { true // Placeholder }
```
With:
```rust
pub spec fn lower_for_each_post(id: StepIdx, input: SlotIdx, item_slot: SlotIdx,
                                 limit: u32, body: StepIdx, done: StepIdx) -> bool
{
    true  // WRONG
}
```
Replace with:
```rust
pub spec fn lower_for_each_post(id: StepIdx, input: SlotIdx, item_slot: SlotIdx,
                                 limit: u32, body: StepIdx, done: StepIdx) -> bool
{
    // Postcondition: returns exactly 2 nodes
    // First node is ForEachStart with correct fields
    // Second node is ForEachNext with iterator_slot == item_slot
    true  // PLACEHOLDER - must be replaced with actual Verus spec
}

// Example real spec structure:
proof fn lower_for_each_post_proof(id: StepIdx, input: SlotIdx, item_slot: SlotIdx,
                                    limit: u32, body: StepIdx, done: StepIdx)
    requires id.as_usize() < 65535
    ensures {
        let result = lower_for_each(id, input, item_slot, limit, body, done);
        result.is_ok() ==> result.get_ok().len() == 2
    }
{
    // Verify implementation returns exactly 2 nodes
}
```

#### VERUS-POST-002 through VERUS-POST-007
Same pattern — replace `true // Placeholder` with actual spec encoding the postcondition.

**Key postconditions to encode**:
- POST-002: `len == 2`, `TogetherStart.output == Some(accumulator)`, `TogetherJoin.output == Some(accumulator)`
- POST-003: `len == 3`, correct CollectStart/CollectPage/CollectFinish
- POST-004: `len == 3`, `iterator_slot == accumulator` (accumulator reuse)
- POST-005: `len == 3`, `attempt_slot.raw_value() == id.as_usize() + 1`
- POST-007: `len == 2`, `resume.id.raw_value() == id.as_usize() + 1`, `AskResume.output == Some(answer)`

---

### 3. Bound Mismatch in INV-001/INV-002 (MAJOR)

**File**: `verification/verus_invariants.vr`

**Problem**: Proof uses `id < u16::MAX - 1` but contract PRE-001 is ambiguous.

**Required Fix**:
First, clarify with contract owner: does PRE-001 "within u16::MAX - 1 range" mean:
- `id <= u16::MAX - 1` (then id+1 overflows for id=u16::MAX-1, requiring error handling)?
- `id < u16::MAX - 1` (then id+1 always succeeds)?

If first interpretation: the proof should verify the `ok_or` error path is taken for `id = u16::MAX - 1`:
```rust
proof fn lower_repeat_invariant_proof(id: StepIdx)
    requires id.as_usize() <= u16::MAX as usize - 1
    ensures {
        match id.as_usize().checked_add(1) {
            Some(v) => v <= u16::MAX as usize,
            None => id.as_usize() == u16::MAX as usize - 1,  // Error path taken
        }
    }
```

---

### 4. VERUS-WAITKIND Trusted Boundary (MAJOR)

**File**: `verification/verus_waitkind.vr`

**Problem**: Proof trusts Rust compiler, not Verus.

**Required Fix**:
```rust
pub spec fn waitkind_variant_count() -> nat {
    2  // Exactly 2 variants: Until and Event
}

proof fn waitkind_exhaustiveness_proof(w: WaitKind)
    ensures match w {
        WaitKind::Until { .. } => true,
        WaitKind::Event { .. } => true,
    }
{
    match w {
        WaitKind::Until { .. } => { },
        WaitKind::Event { .. } => { },
    }
}
```

---

### 5. Kani Harness Defects (MAJOR)

**File**: `verification/kani_lower_control.rs`

**Required Fixes**:

1. Remove dead code (line 51):
```rust
// DELETE this line:
let id = StepIdx::new(kani::any());
// KEEP only:
let id = StepIdx::new(id_val as usize);
```

2. Remove unnecessary `.max(0)` (lines 59-60):
```rust
// WRONG:
let body = StepIdx::new(((id_val as usize) + 100).max(0));
// CORRECT (unsigned addition cannot go negative):
let body = StepIdx::new((id_val as usize) + 100);
```

3. Add concrete `attempt_slot` verification (line ~73-77):
```rust
// WRONG:
assert!(true, "overflow check passed");
// CORRECT - verify the actual slot value:
if let crate::CompiledNodeKind::RepeatAttempt { attempt_slot, .. } = &nodes[1].kind {
    assert!(attempt_slot.raw_value() == id_val as usize + 1,
            "attempt_slot must equal id + 1");
}
```

4. Consider increasing unwind bound if path depth requires it.

---

### 6. Integration for Actual Verification

**Verus**: The companion `.vr` files cannot be verified standalone. Options:
- A) Merge `.vr` contents into `lib.rs` as inline `verus!{}` blocks
- B) Keep `.vr` as companion but ensure `verus` command picks them up: `verus crates/vb_compile/src/lib.rs` (Verus auto-discovers companion `.vr` files)

**Kani**: Add to vb_compile src tree:
```rust
// In crates/vb_compile/src/lib.rs add:
#[cfg(kani)] pub mod kani_lower_control;
```

**TLA+**: Copy spec files to repo `specs/` directory and run from there.

---

## Verification After Repair

```bash
# Verus (after integration into lib.rs)
verus crates/vb_compile/src/lib.rs

# Kani (after adding to src tree)
cargo kani --harness kani_lower_control --force-mc-flags

# TLA+ (after syntax fix)
tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla

# Clippy (already passes)
cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings
```

---

## Return to State 5

After making all fixes above, update STATE.md to `owner_state: 5` and `rerun_from: 5` to re-enter proof writing.
