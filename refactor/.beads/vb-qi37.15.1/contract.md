bead_id: vb-qi37.15.1
bead_title: cli: Add simulate command
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# Contract Specification

## Preconditions
- PRE-001: Input workflow path exists and is readable.
- PRE-002: Workflow source parses and compiles before simulation.

## Postconditions
- POST-001: `simulate` exits 0 for valid workflows and emits deterministic dry-run steps plus summary counts.
- POST-002: `simulate --json`/`--jsonl` emits a bounded structured trace with `success`, totals, and per-step metadata.
- POST-003: Simulation never opens or writes a durable DB or external action sink.

## Invariants
- INV-001: Dry-run output is derived only from compiled workflow structure.
- INV-002: Action nodes are described as would-execute, never actually executed.

## Error Taxonomy
- ERR-001: Unreadable workflow -> validation/read failure.
- ERR-002: Invalid workflow -> compile failure diagnostic.

## Contract Signatures
- simulate_workflow(workflow: &CompiledWorkflow) -> SimulationResult

## Verus-Owned Clauses
- None required for this I/O shell bead; pure enumeration is covered by tests and static gates.

## TLA+-Owned Clauses
- None required; no runtime lifecycle transition occurs in dry-run.
