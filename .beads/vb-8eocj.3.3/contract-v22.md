# Repeat Counter Write-Ownership Validation Contract (V22)

## Problem
Malformed IR can cause a Repeat's body to write to the Repeat's owning counter slot, which is accepted by the constructor and only manifests as StepBudgetExhausted at runtime. This is a counter-allocation bug, not canonical-source reachability.

## Solution
Add cold validation that rejects any node (other than the Repeat's own RepeatStart and RepeatCheck nodes) that writes to a Repeat's counter slot via its `output` or `error_slot` field.

## Design
1. Identify all Repeat counter slots from RepeatCheck nodes' `attempt_slot` fields
2. For each counter slot, build the set of legitimate writers:
   - The RepeatStart node (identified by its `output` field matching the counter slot)
   - The RepeatCheck node itself (identified by its `attempt_slot` field)
3. Reject any other node that has `output` or `error_slot` set to the counter slot
4. Typed error: `WorkflowError::RepeatCounterForeignWrite` with writer/owner/slot fields

## Scope
- vb_core only
- CompiledNodeKind audit: all node kinds that write to slots
- Preserve valid max1/max3/private body output/exhausted current==max patterns
- No decoder changes (> remains >)
- No runtime or compiler lowering edits
- No test edits

## Acceptance Criteria
1. Valid Repeat workflows (max1, max3, private body output, exhausted) compile and pass
2. Malformed Repeat with body writing to counter slot is rejected with typed error
3. Error message includes writer step, owner step, and counter slot
4. No regression in existing validator behavior
