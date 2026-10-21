# Bead vb-jei3u Contract

## Subject
Production compile_source digest determinism and Finish result/identifier
sensitivity over valid workflows; invalid slot inputs must return exact
typed ResourceContractExceeded max_slots errors, never silently filtered.

## Production Behavior (What Must Be True)

1. **Digest Determinism (C4, C9)**: Compiling the same WorkflowSource twice
   produces identical CompiledWorkflow digests. The digest is computed on
   the canonical AST before lowering, so it is deterministic and
   layout-independent.

2. **Finish Result Value Sensitivity (C1)**: Changing the Integer value of
   a finish step's result slot (within valid resource bounds) produces a
   different digest. The slot index is part of the canonical source
   representation that is hashed.

3. **Finish Step Identifier Sensitivity (C3)**: Changing step IDs
   produces different digests because step IDs
   are encoded in the canonical source representation.

4. **Invalid Slot Input Handling**: Slot indices from max_slots (1024 in
   DEFAULT ResourceContract) through 65534 must produce exactly one:
   - `CompileError::Workflow(WorkflowError::ResourceContractExceeded {
     resource: "max_slots" })`
   - Index 65535 must produce exactly one
     `CompileError::SlotIndexOutOfRange { value: 65535 }`, because the
     required slot count cannot fit in u16. This preserves the existing
     checked-overflow behavior; compiler production code is unchanged.
   - NOT silently filtered
   - NOT causing panics, unwraps, or expect failures
   - NOT causing the test to return Ok(()) early

## Constants and Types

- `ResourceContract::DEFAULT.max_slots = 1024` (u16)
- Valid slot range for digest properties: 0..1024
- Invalid slot range for boundary tests: >= 1024

## Test Requirements

- All 4 proptest properties execute under default `cargo test` (no `#[ignore]`)
- Generators use bounded strategies: `0..1024` for valid slot values
- Wide varied inputs preserved (many different slot values and step IDs)
- No `prop_assume` to filter invalid inputs; use separate negative tests
- Separate negative boundary tests for max-1 (1023), max (1024), max+1 (1025),
  and oversized u16 values (e.g., 65535)
- Negative tests assert exact error variant and resource field
- No `expect`, `panic`, or `unwrap` in test helpers
- No indexing operations on slices
- No deletion or weakening of existing properties

## Proof Seeds

- PS-FINISH-DIGEST-003: Determinism with varied inputs
- PS-FINISH-DIGEST-007: Determinism with many compilations
- PS-FINISH-DIGEST-001: Different finish values produce different digests
- PS-FINISH-DIGEST-010: Different step IDs produce different digests
