# Black-Hat Adversarial Review — vb-0sps State 12

## STATUS: REJECTED

## Verdict

Formal verification passed at State 11 with waivers and compensating evidence. However, a **critical implementation gap** survives the formal gates: `compare_observed_runs` never compares slot values, contradicting POST-001's explicit requirement that "all observed slot values" must match. This gap is masked by the test architecture (both sides use manually constructed expected values), not by genuine parity proof.

---

## Phase 1: Contract & Bead Parity

### CRITICAL FAILURE — Slot Values Not Compared

**File:** `crates/vb_codegen/src/codegen/parity.rs`  
**Function:** `compare_observed_runs` (lines 248–253)

**Contract POST-001 says:**
> "terminal result SlotValue, terminal result Taint, terminal status, final PC, executed-step count where observable, **all observed slot values**, all slot taints, all step states, and normalized terminal events match exactly."

**Actual implementation:**
```rust
pub fn compare_observed_runs(ir: &ObservedRun, gen_run: &ObservedRun) -> Result<(), ParityError> {
    compare_terminal_status(&ir.status, &gen_run.status)?;  // ✓ status, result value, result taint, pc, executed
    compare_journals(&ir.journal, &gen_run.journal)?;        // ✓ journal events
    compare_taints(&ir.taints, &gen_run.taints)?;           // ✓ slot taints only
    // MISSING: ir.slots vs gen_run.slots — slot VALUES never compared!
    Ok(())
}
```

**The `ObservedRun` struct** (line 127–140) has both:
- `slots: Vec<(SlotIdx, SlotValue)>` — slot values
- `taints: Vec<(SlotIdx, Taint)>` — slot taints

**`compare_taints`** (lines 415–448) compares taints only, never touches `slots`:
```rust
fn compare_taints(ir: &[(SlotIdx, Taint)], gen_run: &[(SlotIdx, Taint)]) -> Result<(), ParityError>
```

**Consequence:** If the generated runtime writes `SlotValue::I64(99)` to slot 0 while IR writes `SlotValue::I64(42)` to slot 0, but both have `Taint::Clean`, `compare_observed_runs` returns `Ok(())` — false positive parity.

### Slot Values Not Covered by Taint Comparison

Some might argue: "taints imply values" — NO. Taint is a security label (Clean/DerivedFromSecret/etc.). A slot can have `Taint::Clean` but hold `I64(99)` vs `I64(42)`. These are independent fields.

### Error Taxonomy Mismatch (Contract vs Implementation)

**Contract error taxonomy (contract.md lines 55–63):**
- `ParityError::TerminalMismatch { field }` ✓ (exists)
- `ParityError::SlotMismatch { slot, field }` — **NEVER PRODUCED** (missing in code)
- `ParityError::StepStateMismatch { step }` — **NEVER PRODUCED** (missing in code)
- `ParityError::SuspensionMismatch { field }` ✓ (exists)
- `ParityError::ResumeMismatch { field }` ✓ (exists)
- `ParityError::JournalMismatch { index, field }` ✓ (exists, called JournalMismatch not EventMismatch)
- `ParityError::TypedErrorMismatch { field }` — **NEVER PRODUCED** (missing in code)
- `ParityError::UnsupportedMismatch { feature }` — **IN CODE ONLY, NOT IN CONTRACT**

**Code parity.rs variants (lines 148–205):**
- `TerminalMismatch` ✓
- `JournalMismatch` ✓
- `TaintMismatch` ✓ (contract says `SlotMismatch` for slot taint/value issues)
- `SuspensionMismatch` ✓
- `ResumeMismatch` ✓
- `UnsupportedMismatch` ✓ (not in contract taxonomy)

**Gap:** `compare_observed_runs` cannot produce `SlotMismatch` because slot values are never compared. If a future bug causes slot value divergence, it silently passes.

### Test Architecture Masking the Gap

The BDD test `deterministic_workflow_terminal_parity_when_ir_and_generated_finish` (line 691):
1. Constructs `gen_run` manually: `slots: vec![(SlotIdx::new(0), SlotValue::I64(42))]`
2. Calls `run_ir_to_completion(&workflow)` which produces `ir_run.slots`
3. Calls `compare_observed_runs(&ir_run, &gen_run)`

**Problem:** Both `ir_run.slots` and `gen_run.slots` are constructed to have the same expected value. If `run_ir_to_completion` returned `slots: vec![(SlotIdx::new(0), SlotValue::I64(99))]`, the test would FAIL — but NOT because `compare_observed_runs` caught it. It would fail because the manually constructed `gen_run.slots` doesn't match `ir_run.slots`. The test passes because `gen_run.slots` was constructed to match the expected IR output — circular reasoning.

**Real parity proof** requires: running actual generated code and comparing its output to IR output. This bead does not yet have the compilation pipeline wired. The formal verification report acknowledges this (Residual Risk #1: "Generated code execution not available").

### Verification-Ledger POST-001 Evidence Is Incomplete

The ledger claims:
```
POST-001: PASS — "BDD `compare_observed_runs` fixture passes with structured assertions"
```

But `compare_observed_runs` doesn't compare slot values. The "structured assertions" pass because they compare IR output to manually constructed expected output that was built to match — not because the comparison function is complete.

---

## Phase 2: Farley Engineering Rigor

### Function Length — `compare_terminal_status` is 115 lines

**Hard constraint violation:** Any function over 25 lines should be flagged.

`compare_terminal_status` (lines 256–370) is 115 lines with 5+ parameters. It handles three `TerminalStatus` variants with exhaustive field comparisons.

**Assessment:** This is borderline acceptable because:
- The three arms are structurally identical (compare variant fields)
- A helper trait/object would be over-engineering for 3 variants
- But 115 lines violates the 25-line guideline

### No I/O Hiding

`compare_observed_runs` is pure computation — no I/O inside calculation logic. ✓

---

## Phase 3: Holzman Rust (The Big 6)

### Make Illegal States Unrepresentable ✓

`TerminalStatus` is a sum type with exactly 3 exhaustive variants:
- `Finished(FinishedRun)` — terminal success
- `Blocked(BlockedRun)` — suspension
- `Error(ErrorRun)` — typed error

No `Option`-based state machine. ✓

### Parse, Don't Validate ✓

Data is parsed into `FinishedRun`, `BlockedRun`, `ErrorRun` at boundary. The types are trusted after construction. ✓

### Types as Documentation ✓

No boolean parameters. `BlockKind` is an explicit enum. ✓

### No Raw Unwrapped Primitives in Domain Models ✓

`RunId`, `SlotIdx`, `StepIdx` are newtype wrappers. ✓

### `#[non_exhaustive]` on Public Enums

`BlockKind`, `ErrorClass`, `TerminalStatus`, `ParityError` are all `#[non_exhaustive]`.

**Assessment:** Acceptable for test/BDD-facing API where future variants may be needed. Not a safety issue. But the contract explicitly enumerates error variants — `#[non_exhaustive]` contradicts the explicit taxonomy. Minor concern.

---

## Phase 4: Ruthless Simplicity & DDD

### No Option-Based State Machine ✓

`TerminalStatus` is not `Option<TerminalStatus>`. ✓

### CUPID — Predictable, Idiomatic ✓

`compare_observed_runs` is a pure function: same inputs → same output. Predictable. ✓

### Panic Vector — No `unwrap/expect/panic/todo` in parity.rs ✓

The parity module is clean. No `unwrap()`, `expect()`, `panic!()`, `todo!()`. ✓

### `let _ = i;` No-Op in `compare_taints`

Line 445:
```rust
let _ = i; // <— unused variable, do not use
```

This is a linter suppression pattern but it's not actually needed — `i` is consumed by the enumerate iteration, the `let _ =` is superfluous. Minor style issue.

---

## Phase 5: The Bitter Truth

### YAGNI — Extra Error Variants

`UnsupportedMismatch` exists in code (line 201) but is NOT in the contract error taxonomy (contract.md lines 55–63). `SlotMismatch` and `TypedErrorMismatch` are in the contract but NEVER produced.

**This is a divergence between contract and implementation.** Either:
1. The contract is wrong and should remove `SlotMismatch`/`TypedErrorMismatch`
2. The implementation is wrong and should produce these variants

The formal verification gates passed with "compensating evidence" — but the compensating evidence doesn't address this specific taxonomy gap.

### Test Design = Circular Reasoning

The BDD tests pass but don't prove what they claim to prove:
- They prove: "manually constructed expected values match IR output"
- They do NOT prove: "generated code produces identical output to IR"

This is a **known residual risk** acknowledged in the implementation report (#1). But it means the 34 passing tests are not genuine parity evidence — they are fixture validation tests.

---

## Findings Summary

| Severity | Finding | Location | Contract Clause |
|----------|---------|----------|-----------------|
| **CRITICAL** | `compare_observed_runs` never compares `slots` — slot values unverified | parity.rs:248–253 | POST-001 |
| **HIGH** | `SlotMismatch` in contract taxonomy never produced | parity.rs:148–205 | Error Taxonomy |
| **HIGH** | `TypedErrorMismatch` in contract taxonomy never produced | parity.rs:148–205 | Error Taxonomy |
| **MEDIUM** | Test architecture provides circular parity evidence | vb_0sps_generated_ir_parity_bdd.rs:691 | POST-001 |
| **LOW** | `compare_terminal_status` is 115 lines ( guideline: 25) | parity.rs:256–370 | Farley |
| **LOW** | `UnsupportedMismatch` in code but not in contract taxonomy | parity.rs:201 | Error Taxonomy |
| **LOW** | `let _ = i;` no-op | parity.rs:445 | Style |
| **INFO** | `#[non_exhaustive]` on public enums contradicts explicit contract taxonomy | parity.rs:58,94,110,147 | Contract |

---

## Repair Guide

### MANDATORY FIX (CRITICAL)

**Add slot value comparison to `compare_observed_runs`:**

```rust
pub fn compare_observed_runs(ir: &ObservedRun, gen_run: &ObservedRun) -> Result<(), ParityError> {
    compare_terminal_status(&ir.status, &gen_run.status)?;
    compare_slots(&ir.slots, &gen_run.slots)?;  // ADD THIS
    compare_journals(&ir.journal, &gen_run.journal)?;
    compare_taints(&ir.taints, &gen_run.taints)?;
    Ok(())
}

fn compare_slots(ir: &[(SlotIdx, SlotValue)], gen: &[(SlotIdx, SlotValue)]) -> Result<(), ParityError> {
    if ir.len() != gen.len() {
        return Err(ParityError::SlotMismatch {
            slot: ir.first().map(|(s, _)| *s).unwrap_or_else(|| gen.first().map(|(s, _)| *s).unwrap_or(SlotIdx::new(0))),
            detail: format!("slot count mismatch: ir_len={}, gen_len={}", ir.len(), gen.len()),
        });
    }
    for (i, ((ir_idx, ir_val), (gen_idx, gen_val))) in ir.iter().zip(gen.iter()).enumerate() {
        if ir_idx != gen_idx {
            return Err(ParityError::SlotMismatch {
                slot: *ir_idx,
                detail: format!("slot index mismatch at position {}: ir={:?}, gen={:?}", i, ir_idx, gen_idx),
            });
        }
        if ir_val != gen_val {
            return Err(ParityError::SlotMismatch {
                slot: *ir_idx,
                detail: format!("slot value mismatch at {:?}: ir={:?}, gen={:?}", ir_idx, ir_val, gen_val),
            });
        }
    }
    Ok(())
}
```

### REQUIRED: Update Error Taxonomy Alignment

Either:
1. **Option A (preferred):** Add `SlotMismatch` and `TypedErrorMismatch` to `ParityError` enum in parity.rs, and implement them in `compare_observed_runs` and `compare_terminal_status`
2. **Option B:** Update contract.md to remove `SlotMismatch` and `TypedErrorMismatch` from the error taxonomy, since they are not produced

### REQUIRED: Add Real Slot Value Mismatch Test

The mutation test M8 (`mutation_detects_wrong_slot_index_in_suspension`) does NOT test slot value mismatch — it tests that identical blocked runs match. Add:

```rust
#[test]
fn mutation_detects_slot_value_mismatch() {
    let gen_run = ObservedRun {
        status: TerminalStatus::Finished(FinishedRun { /* ... */ }),
        slots: vec![(SlotIdx::new(0), SlotValue::I64(99))], // WRONG value
        // ...
    };
    let result = compare_observed_runs(&ir_run, &gen_run);
    assert!(result.is_err(), "slot value mismatch should be detected");
}
```

---

## Waivers Do Not Cover This Gap

The formal verification report shows:
- PRE-003, POST-002, INV-002, INV-003: WAIVED via `WAIVER-VERUS-ADAPTERS-001`
- These waivers cover: "concrete adapter exec functions do not exist in State 3"

But the slot value comparison gap is NOT about missing adapters — `compare_observed_runs` exists and is fully implemented. It simply doesn't compare slot values. This is an implementation omission, not a missing adapter.

**The waivers do not apply here.**

---

## Conclusion

The bead has passed formal verification at State 11 with compensating evidence. However, the compensating evidence does not address the slot value comparison gap. The `compare_observed_runs` function is the core parity comparison primitive — it must compare slot values per POST-001.

**This gap would cause a silent false positive parity result if generated code wrote wrong slot values.**

**REJECTED.** Fix the slot comparison gap and re-run State 12 black-hat review.
