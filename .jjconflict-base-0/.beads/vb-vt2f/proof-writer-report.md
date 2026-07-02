# vb-vt2f State 5 Proof Writer Report

## Scope

- Bead: `vb-vt2f`
- State: 5
- Sublane: `black-hat-stale-ask-kani-projection-repair`
- Attempt: 1
- Role: proof-writer delegate
- Isolated workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

## Defect Addressed

**LETHAL-001** (`defects.md:3-7`): `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs:140-163` and `:250-254` modeled stale ask tickets as successful, but contract and BDD oracle require stale ask answers to fail with `RuntimeError::RunNotFound` (`contract.md:64-65`, `vb_vt2f_direct_runtime_api_acceptance.rs:658-698`).

## Fix Applied

Three changes to `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs`:

### 1. `answer_ask` (lines 140-154)

**Before (buggy):** `TicketShape::Stale` shared the success arm with `Matching` when `target_active && target_asking` was true.

**After (fixed):** `TicketShape::Stale` is removed from the success arm. Only `Matching` with active+asking returns `Ok`. `Stale`, `WrongRun`, `AbsentRun`, and non-matching `Matching` all return `Err(KernelRuntimeError::RunNotFound)`.

```rust
// ERR-004 / LETHAL-001 fix: Stale ask must fail with RunNotFound per contract
TicketShape::Matching if self.target_active && self.target_asking => {
    self.answer_value = Some(value);
    self.answer_taint = taint;
    self.target_asking = false;
    Ok(())
}
TicketShape::Stale | TicketShape::WrongRun | TicketShape::AbsentRun | TicketShape::Matching
    => Err(KernelRuntimeError::RunNotFound),
```

### 2. `tick_after_answer` (lines 157-171)

**Before (buggy):** `TicketShape::Stale` shared the success arm with `Matching` when `answer_value.is_some()` was true.

**After (fixed):** `Stale` always returns `RunNotFound`. `Matching` with an answer succeeds.

```rust
// ERR-004 / LETHAL-001 fix: Stale ask must fail with RunNotFound per contract.
TicketShape::Matching if self.answer_value.is_some() => {
    self.target_active = false;
    Ok(())
}
TicketShape::Stale | TicketShape::WrongRun | TicketShape::AbsentRun => {
    Err(KernelRuntimeError::RunNotFound)
}
// Matching with no answer value is also an error
TicketShape::Matching => Err(KernelRuntimeError::RunNotFound),
```

### 3. Test assertion branching (lines 257-270)

**Before (reflected buggy behavior):** `if matches!(shape, TicketShape::Matching | TicketShape::Stale)` grouped Stale with Matching as success case.

**After (contract-aligned):** `if matches!(shape, TicketShape::Matching)` — only Matching goes to success branch. Stale/WrongRun/AbsentRun fall to else branch with `RunNotFound` assertions.

## Obligation Results

| Obligation | Status | Evidence |
|---|---|---|
| `KANI-VT2F-RUNTIME-FACADE-001` (LETHAL-001 repair) | PASS | `0 of 489 failed`, `7 of 7 cover properties satisfied`, `VERIFICATION:- SUCCESSFUL`. |

## Kani Run Evidence

Command: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics`

Result: PASS. Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e3b5c68ff001X2hRbAWzZILtpN`

```
SUMMARY:
 ** 0 of 489 failed
 ** 7 of 7 cover properties satisfied
 VERIFICATION:- SUCCESSFUL
 Manual Harness Summary:
 Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

## Cover Points Satisfied

All 7 cover points remain satisfied after the fix:
- missing accepted artifact store covered
- accepted artifact store covered
- strict policy covered
- matching ticket covered
- stale ticket covered
- wrong-run ticket covered
- absent-run ticket covered

## Residual Risk

- **PROJ-EQ-VT2F-001**: The projection-equivalence review maps `KernelRuntimeError`, `FacadeKernelState`, `TicketShape`, etc. to concrete behavior. The stale-ask semantics fix aligns the projection kernel with the concrete BDD oracle. PROJ-EQ-VT2F-001 must be re-reviewed to confirm the corrected mapping is still acceptable.

## Next Routing

This sublane completes the LETHAL-001 repair. Next steps:
1. Route to proof-reviewer to verify the corrected projection semantics
2. PROJ-EQ-VT2F-001 re-review after this state lands
3. State advancement to unblock State 11/12
