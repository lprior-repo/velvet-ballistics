# Contract Verification Review Request: vb-qi37.3.2

## Bead Information

- **Bead ID**: vb-qi37.3.2
- **Title**: runtime/storage: Verify collect cursor persistence
- **Workspace**: `/home/lewis/src/Velvet-ballistics`
- **State**: 1.5 (Contract artifacts written)
- **Previous Sibling**: vb-qi37.3.1 (collect state isolation verified)
- **Reviewer Role**: Independent contract synthesizer (rust-contract skill)

## Reviewer Assignment

This review request is directed to the **contract-verification-reviewer** skill agent.

## Artifact Summary

| Artifact | Location | Status |
|----------|----------|--------|
| `contract.md` | `.beads/vb-qi37.3.2/contract.md` | Written |
| `lean-contract.md` | `.beads/vb-qi37.3.2/lean-contract.md` | Written |
| `verification-layers.md` | `.beads/vb-qi37.3.2/verification-layers.md` | Written |
| `proof-obligations.jsonl` | `.beads/vb-qi37.3.2/proof-obligations.jsonl` | Written |
| `traceability-matrix.jsonl` | `.beads/vb-qi37.3.2/traceability-matrix.jsonl` | Written |
| `martin-fowler-tests.md` | `.beads/vb-qi37.3.2/martin-fowler-tests.md` | Written |
| `test-plan.md` | `.beads/vb-qi37.3.2/test-plan.md` | Written |

## Contract Focus

vb-qi37.3.2 extends vb-qi37.3.1's collect state isolation proof to cover the Fjall journal persistence and recovery path:

1. **Capture**: `drive_deterministic_full` calls `collect_states.capture_state(run.run_id(), slot)` at `drive.rs:98`
2. **Embedding**: `evidence.push_slot_written_with_extra` receives the cursor extra at `drive.rs:100`
3. **Persistence**: `SlotWrittenEvent { extra: Some(bytes) }` written to Fjall journal
4. **Recovery**: `hydrate_collect_states_from_recovered_journal` rebuilds `CollectStates` from journal extras

## Key Contract Claims

1. **PP1-PP4**: Capture and encoding preconditions are structurally proven
2. **PQ1-PQ6**: Persistence path postconditions covered by existing tests
3. **RP1-RP5**: Recovery preconditions structurally proven
4. **RQ1-RQ6**: Recovery postconditions covered by existing tests
5. **PI1-PI4**: Invariants preserved through full persistence cycle

## Verification Layer Coverage

- **unit tests**: 15 clauses covered (collect_tests.rs:2112-2307)
- **code-review**: 9 clauses structurally proven
- **waiver**: 2 clauses (Postcard round-trip, identity validation) — compensated by unit tests
- **Total coverage**: 100% of 25 contract clauses have at least one verification layer

## Reviewer Tasks

1. **Read all artifacts** in `.beads/vb-qi37.3.2/`
2. **Verify completeness**: Every contract clause has a test or proof
3. **Verify correctness**: Lean waivers are justified; code review proofs are valid
4. **Verify JSONL validity**: `proof-obligations.jsonl` and `traceability-matrix.jsonl` are valid JSONL
5. **Write** `contract-verification-review.md` with `STATUS: APPROVED` or `STATUS: REJECTED`

## Next State

Upon `STATUS: APPROVED`, advance to State 2 (codebase mapping via `explore`).
