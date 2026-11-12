# Red Queen Report — vb-2yb8

## Date: 2026-05-09
## Adversarial Agent: GoMasterOrchestrator

## Coevolutionary Pressure Applied

### Pressure 1: Matrix Drift
**Attack:** A future developer adds a new primitive to CompiledNodeKind but forgets to update DURABILITY_MATRIX.
**Defense:** `verify_matrix_completeness()` fails, gate test catches it.
**Result:** Defense survives. The REQUIRED_PRIMITIVES const is the source of truth.

### Pressure 2: Event Mapping Drift
**Attack:** A developer changes RecordKind variants in events.rs but forgets to update matrix rows.
**Defense:** Compile-time check — RecordKind is typed, so mismatches are compiler errors.
**Result:** Defense survives. Rust type system prevents drift.

### Pressure 3: Ack Point Erosion
**Attack:** A developer refactors a handler to return Ok before journal append.
**Defense:** Integration tests verify event presence before tick returns.
**Result:** Defense survives for tested handlers. Risk: untested handlers (resume, inspect, legacy action completion).

### Pressure 4: Test Evidence Rot
**Attack:** Tests referenced in test_evidence are deleted or renamed.
**Defense:** No automated check that test_evidence paths exist.
**Result:** Defense FAILS. The matrix could reference non-existent tests.

### Pressure 5: Storage Partition Mismatch
**Attack:** Events claim to go to ActionJournal but actually go to RuntimeJournal.
**Defense:** No automated verification of storage partition mapping.
**Result:** Defense FAILS. Partition mapping is declarative only.

## Mutations Survived

| Mutation | Caught By |
|----------|-----------|
| Remove a row from DURABILITY_MATRIX | matrix_has_row_for_every_primitive |
| Change ack_point to BeforeJournalAppend | no_row_claims_ack_before_persist |
| Empty test_evidence | every_row_has_replay_proof |
| Remove journal append from handle_cancel | cancel_persists_before_ack |

## Mutations NOT Caught

| Mutation | Why Not Caught |
|----------|----------------|
| Change storage_partition to wrong value | No integration test verifies actual keyspace |
| Change replay_assertion text | Not mechanically verified |
| Add primitive to REQUIRED_PRIMITIVES but not matrix | Caught, but error is at test time not compile time |

## Recommendations

1. Add compile-time verification that test_evidence paths exist (build script)
2. Add integration test that verifies actual Fjall keyspace for each event type
3. Add `static_assertions` for matrix size == primitives count

STATUS: REVIEWED — 2 gaps identified, not blocking
