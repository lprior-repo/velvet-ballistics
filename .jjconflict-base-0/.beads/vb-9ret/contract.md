# Contract: vb-9ret validate/compile adapters

## Title
validate/compile: Preserve adapters while removing residual duplication

## Scope
- vb_validate and vb_compile adapter preservation during deduplication
- Workflow compilation pipeline integrity
- No adapter state corruption

## Preconditions
- PRE-001: Adapter trait signatures preserved after deduplication
- PRE-002: Compile workflow succeeds with preserved adapters

## Postconditions
- POST-001: Output artifacts match expected structure

## Invariants
- INV-001: No adapter state corruption during deduplication

## Error Taxonomy
- AdapterStateCorruption: adapter state corrupted during deduplication
- CompileWorkflowFailure: workflow compilation failed with preserved adapters

## Non-Goals
- vb_core budget computation internals
- Makepad rendering, OS filesystem semantics
- moon ci pre-contract gate (see verification-layers.md waiver)
