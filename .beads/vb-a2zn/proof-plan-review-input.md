# Proof Plan Review Input — vb-a2zn

## Bead

`vb-a2zn` — durability: normalize absent-run query outcomes

## Review Context

The proof plan covers 22 obligations across 5 verifier layers for a single-bead change: adding `JournalError::NoEvents`, changing `events_for_run` to return `Err` instead of `Ok([])`, updating exit-code mapping, and simplifying 7 CLI commands.

## Contract-Proof Traceability

| Contract Clause | Obligation IDs | Status |
|---|---|---|
| PRE-003 (PRE: `events_for_run` called by `recover_full_journal`) | VERUS-INV-001, KANI-INV-001 | Covered |
| PRE-005 (`NoEvents` semantically distinct from `ProcessLockHeld`) | STATIC-JOURNAL-ERROR | Covered |
| POST-001 (`events_for_run` returns `Err(NoEvents)` iff no events) | VERUS-INV-001 | Covered |
| POST-002 (`events_for_run_from` returns `Err(NoEvents)` iff no events) | VERUS-INV-002 | Covered |
| POST-003 (`NoEvents` variant exists with `run: RunId`) | STATIC-JOURNAL-ERROR | Covered |
| POST-004 (exit-code mapping `NoEvents → ValidationFailed`) | STATIC-INV-003, PROPT-FROM-001 | Covered |
| POST-005 (all 7 read commands → exit code 2 for absent runs) | BDD-EXIT-001..009 | Covered |
| POST-006 (`read_journal_events` propagates `NoEvents` Err) | STATIC-INV-008 (new) | Covered |
| POST-007 (`cmd_trace` via `read_journal_events` → correct exit) | BDD-EXIT-004, STATIC-INV-008 | Covered |
| POST-008 (`cmd_diff` → correct exit for absent runs) | BDD-EXIT-007,008, STATIC-INV-009 | Covered |
| POST-009 (`recover_full_journal` dead code removed) | STATIC-INV-006 | Covered |
| POST-010 (`cmd_events` empty check removed) | STATIC-INV-004 | Covered |
| POST-011 (`cmd_inspect` empty check removed) | STATIC-INV-005 | Covered |
| POST-012 (`cmd_retry`/`resume` empty check removed) | STATIC-INV-007 | Covered |
| INV-001 (`events_for_run` never returns `Ok([])`) | VERUS-INV-001, KANI-INV-001 | Covered |
| INV-002 (`events_for_run_from` never returns `Ok([])`) | VERUS-INV-002, KANI-INV-002 | Covered |
| INV-003 (all read commands return SAME exit code for absent) | BDD-EXIT-009, PROPT-FROM-001 | Covered |
| INV-004 (orthogonality: `NoEvents` vs `ProcessLockHeld`) | STATIC-JOURNAL-ERROR | Covered |
| INV-006 (`NoEvents` `#[non_exhaustive]` compatible) | STATIC-JOURNAL-ERROR | Covered |
| INV-007 (`From` maps `NoEvents → ValidationFailed`) | STATIC-INV-003, PROPT-FROM-001 | Covered |

**Traceability: 100% covered.**

## Plan Correctness Assessment

### Correct decisions
1. **Verus for INV-001/002** — The core risk is that `Ok(Vec::new())` exists as a return path. Verus is the right tool: prove that `if replay.is_empty()` guards all empty paths and the function cannot reach `Ok(replay)` with zero elements.
2. **Kani for panic-freedom** — Cheap verification that `events_for_run_from` doesn't panic on bounded inputs (no unwrap/index arithmetic in the for-loop body).
3. **Static-scan for CLI simplifications** — These are structural changes (removing dead code, fixing error mapping). Static checks are cheaper and more precise than tests.
4. **proptest for From impl** — One property test covers the mapping function exhaustively: no variant can map to an invalid `CliExitCode`.
5. **BDD for end-to-end** — The contract requires 7 commands to agree on exit code 2. Only end-to-end testing can verify the full pipeline (journal → error → CLI handler → exit code).

### Corrections from review
1. **Added STATIC-INV-008** — `read_journal_events` currently hardcodes `StorageError` for ALL errors. After the fix, it must check `NoEvents` specifically and return `ValidationFailed`. The original plan did not include this because POST-007 incorrectly said `StorageError`.
2. **Added STATIC-INV-009** — `cmd_diff` similarly hardcodes `StorageError` for ALL errors. Must check `NoEvents` specifically. Same correction as above.

### Waivers / Not-Applicable lanes

| Lane | Verifier | Reason |
|---|---|---|
| TLA+ | tla-plus | No temporal/state-machine behavior; single-threaded CLI command routing |
| Loom | loom | No concurrency; all read commands are sequential |
| Miri | miri | No `unsafe`, no raw pointers, no UB risk in changes |
| Flux | flux-rs | No refinement types or numeric predicates |
| Fuzz | fuzz | No adversarial input boundary; `RunId` parsing already tested |

## Reviewer Checklist

- [ ] Verus spec functions correctly model `events_for_run` and `events_for_run_from` postconditions
- [ ] Verus proof functions cover both `Ok([])` and `Err(NoEvents)` branches
- [ ] Kani harnesses use bounded `RunId` and `EventSeq` inputs (no hardcoded data)
- [ ] `From<JournalError>` impl has explicit `NoEvents → ValidationFailed` match arm
- [ ] `read_journal_events` checks `matches!(e, JournalError::NoEvents { .. })` for exit-code branch
- [ ] `cmd_diff` Err branch checks `NoEvents` specifically
- [ ] All `is_empty()` dead-code removals are in the correct functions
- [ ] No `Ok([])` remains in `events_for_run_from` code path
- [ ] BDD tests use a genuinely absent run (not an existing run with no events)

## Risk Assessment

**Overall risk: LOW.** Changes are localized to error types, one function, one impl, and seven CLI command wrappers. No new dependencies, no concurrency, no unsafe, no protocol changes. The primary risk is a missing Err branch in one CLI command, which BDD-EXIT-009 (exit-code consistency test) catches.
