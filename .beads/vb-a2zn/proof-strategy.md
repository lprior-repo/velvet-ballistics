# Proof Strategy — vb-a2zn (durability: normalize absent-run query outcomes)

## Scope Summary

One error-variant addition, one signature change, one mapping change, and seven CLI simplifications. The entire risk surface is **Rust-local invariant** (empty-vec → typed error) and **exit-code consistency** (all read commands agree on exit code 2 for absent runs).

## Risk Classification

| Risk | Trigger | Verifier |
|---|---|---|
| **Rust-local invariant** | `events_for_run` must never return `Ok([])` | Verus + Kani + static-scan |
| **Exit-code contract** | All 7 read commands must return exit code 2 for absent runs | proptest (From impl) + BDD (CLI) |
| **Dead-code removal** | `is_empty` checks after `?` are unreachable | static-scan only |
| **Error mapping** | `NoEvents` → `ValidationFailed`, others → `StorageError` | static-scan + proptest |

**Not in scope**: TLA+ (no temporal behavior), Loom (no concurrency), Miri (no unsafe/raw-pointer UB), fuzz (no adversarial input), Flux (no refinement types).

## Strategy

### Layer 1: Verus (ownership of the core invariant)

Prove that `events_for_run` and `events_for_run_from` satisfy INV-001 and INV-002 respectively:
- Spec functions model the postcondition: `result.is_err()` iff prefix scan yields zero items
- Proof functions verify that the `if replay.is_empty() { Err(NoEvents) }` guard covers all empty paths
- Shell out Fjall I/O, WAL, and durability (we trust Fjall's snapshot semantics)

### Layer 2: Kani (panic-freedom on bounded inputs)

Two harnesses proving that `events_for_run` and `events_for_run_from` never panic on bounded `RunId` and `EventSeq` inputs. The functions already use `?` and `map_err` — Kani confirms no unwraps, no index panics, no arithmetic overflows in the bounded model.

### Layer 3: Static-scan (error variant, exit-code mapping, dead-code removal)

Six grep-based checks:
1. `NoEvents { run: RunId }` exists in `JournalError` enum (POST-003)
2. `From<JournalError>` matches `NoEvents → ValidationFailed` (POST-004)
3. `cmd_events` has no `events.is_empty()` after `events_for_run` (POST-010)
4. `cmd_inspect` has no `events.is_empty()` after `events_for_run` (POST-011)
5. `recover_full_journal` has no `is_empty()` after `events_for_run` (POST-009)
6. `cmd_retry` / `cmd_resume` have no `is_empty()` after `read_journal_events` (POST-012)

**Additions** (code-level correction from contract analysis):
7. `read_journal_events` checks `NoEvents` specifically and returns `ValidationFailed` (corrects POST-007)
8. `cmd_diff` Err branch checks `NoEvents` and returns `ValidationFailed` (corrects POST-008)

### Layer 4: proptest (From impl consistency)

One property test verifying that `From<JournalError> for CliExitCode` always maps to a valid discriminant and that `NoEvents` specifically maps to `ValidationFailed`. This is cheaper than testing all seven CLI commands end-to-end and covers the mapping logic the CLI commands delegate to.

### Layer 5: BDD (end-to-end exit-code consistency)

Nine tests: one per read command (inspect, events, replay, trace, retry, resume, diff_a, diff_b) plus one aggregate test verifying all seven return exit code 2 for the same absent run.

## Contract Correction Notes

The original proof-obligations.jsonl reflects POST-007 (`cmd_trace` → `StorageError`) and POST-008 (`cmd_diff` → `StorageError`). However, INV-003 ("All read-only CLI commands return the SAME exit code (2 = ValidationFailed)") and POST-005 are the higher-level invariants. **INV-003 takes precedence.**

This means:
- `read_journal_events` must distinguish `NoEvents` → `ValidationFailed` from other errors → `StorageError`
- `cmd_diff` Err branch must distinguish `NoEvents` → `ValidationFailed` from other errors → `StorageError`

These corrections are captured as new obligations STATIC-INV-008 and STATIC-INV-009.

## Obligation Summary

| Layer | Count | IDs |
|---|---|---|
| Verus | 2 | VERUS-INV-001, VERUS-INV-002 |
| Kani | 2 | KANI-INV-001, KANI-INV-002 |
| Static-scan | 8 | STATIC-JOURNAL-ERROR, STATIC-INV-003 through STATIC-INV-009 |
| proptest | 1 | PROPT-FROM-001 |
| BDD | 9 | BDD-EXIT-001 through BDD-EXIT-009 |
| **Total** | **22** | |

## Execution Order

1. Verus proofs (core invariant — gates all downstream)
2. Kani harnesses (panic-freedom — independent of Verus)
3. Static-scan checks (verify code matches plan — independent)
4. proptest (mapping property — independent)
5. BDD tests (end-to-end — depends on all above passing)
