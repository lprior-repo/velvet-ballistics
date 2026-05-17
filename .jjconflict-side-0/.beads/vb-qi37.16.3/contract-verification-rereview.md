# Contract Verification Rereview: vb-qi37.16.3 State 4

**Bead**: vb-qi37.16.3
**Review Phase**: State 4 - Contract/Verification Rereview (Post-State-12-Rejection Repair)
**Date**: 2026-05-11
**STATUS**: REJECTED

---

## Files Reviewed

- `.beads/vb-qi37.16.3/contract.md`
- `.beads/vb-qi37.16.3/tla-spec.md`
- `.beads/vb-qi37.16.3/lean-contract.md`
- `.beads/vb-qi37.16.3/verification-layers.md`
- `.beads/vb-qi37.16.3/proof-obligations.jsonl`
- `.beads/vb-qi37.16.3/traceability-matrix.jsonl`
- `.beads/vb-qi37.16.3/formal-waivers.jsonl`
- `.beads/vb-qi37.16.3/state-3-formal-repair.md`
- `.beads/vb-qi37.16.3/formal-verification-report.md`
- `specs/RetryFSM.tla` (verified: EXISTS)
- `specs/RetryJournal.tla` (verified: EXISTS)
- `specs/RetryFSM.cfg` (verified: EXISTS)
- `specs/RetryJournal.cfg` (verified: EXISTS)

---

## Command Evidence

```bash
# TLA+ TLC execution on RetryFSM.tla
tlc -config specs/RetryFSM.cfg specs/RetryFSM.tla
-> Error: Successor state is not completely specified by action ActionFailed of the next-state relation. The following variables are not assigned: maxAttempts, retryPolicy, runs, stepHasRetryCheck.

# TLA+ TLC execution on RetryJournal.tla
tlc -config specs/RetryJournal.cfg specs/RetryJournal.tla
-> Error: TLC encountered a non-enumerable quantifier bound Nat. line 78, col 50 to line 78, col 52 of module RetryJournal

# JSONL validation
jq -c . .beads/vb-qi37.16.3/proof-obligations.jsonl -> valid (15 entries)
jq -c . .beads/vb-qi37.16.3/traceability-matrix.jsonl -> valid (16 entries)
jq -c . .beads/vb-qi37.16.3/formal-waivers.jsonl -> valid (6 entries)
```

---

## Findings

### Severity: LETHAL

**Clause**: TLA-RETRY-001 (INV-002)
**Problem**: `specs/RetryFSM.tla` ActionFailed action does not completely specify the next-state relation. TLC reports: "Successor state is not completely specified by action ActionFailed of the next-state relation. The following variables are not assigned: maxAttempts, retryPolicy, runs, stepHasRetryCheck."

All three branches of ActionFailed (NonRetryable, RetryAllowed, Exhausted) assign only `stepState'`, `actionAttempts'`, and `framePC'`. The remaining variables (`maxAttempts`, `retryPolicy`, `runs`, `stepHasRetryCheck`) are implicitly left unchanged but not explicitly UNCHANGED, which TLC requires.

**Required fix**: Add `UNCHANGED <<maxAttempts, retryPolicy, runs, stepHasRetryCheck>>` to all three branches of ActionFailed.

**rerun_from**: 3

---

### Severity: LETHAL

**Clause**: TLA-RETRY-002 (INV-003) and TLA-RETRY-003 (POST-004)
**Problem**: `specs/RetryJournal.tla` contains a non-enumerable quantifier bound. The JournalIdempotency invariant uses `attempt \in Nat`:

```
\A run \in Runs, step \in Steps, attempt \in Nat :
```

TLC cannot enumerate `Nat` (infinite set). TLC reports: "TLC encountered a non-enumerable quantifier bound Nat."

**Required fix**: Bound `attempt` with a finite range such as `0..10` or `0..MaxAttempts`.

**rerun_from**: 3

---

### Severity: MAJOR

**Clause**: formal-waivers.jsonl
**Problem**: All 6 waiver entries are missing the required `rerun_from` field specified by the executable_obligation_schema rule. Each proof-obligation entry must include `rerun_from`. While the waivers correctly track waiver metadata, they cannot serve as proof-obligation trackers without this field.

**Required fix**: Add `"rerun_from": 3` to all 6 waiver entries in formal-waivers.jsonl.

---

### Severity: MAJOR

**Clause**: WAIVER-VERUS-001
**Problem**: Factual contradiction between waiver reason and formal-verification-report.md:

- **Waiver claims**: "helpers.rs contains no Verus spec/proof annotations - it uses plain Rust"
- **formal-verification-report.md line 119 states**: "helpers.rs contains Verus spec/proof functions (spec_validate_ticket_attempt, proof_validate_ticket_attempt_bounds)"

These statements are mutually exclusive. One must be false.

**Required fix**: Correct the waiver reason to match the actual state of helpers.rs. If helpers.rs DOES contain Verus annotations, the waiver reason must acknowledge this and explain why Verus still cannot execute (toolchain missing).

---

### Severity: MINOR

**Clause**: verification-layers.md, formal-waivers.jsonl
**Problem**: Waiver compensating evidence cites "1364 passing tests (1337 lib + 18 integration + 9 durable retry red-phase)" as evidence that "verifies" the same properties as formal proof. This overclaims test coverage:

- Tests verify **implementation behavior** against a reference implementation
- Tests do NOT verify formal properties such as monotonicity, invariant preservation, or refinement correctness
- The red-queen adversarial execution does NOT prove absence of corner cases in the formal specification

**Required fix**: The compensating evidence should state tests "confirm implementation correctness via adversarial execution" rather than "verify the same properties as formal proof."

---

## Coverage Decision

### Contract clauses traced

All 15 contract clauses (PRE-001 through PRE-004, POST-001 through POST-007, INV-001 through INV-005) are present in `traceability-matrix.jsonl`.

### TLA+-owned clauses covered

| Clause | TLA+ Obligation | Status |
|--------|----------------|--------|
| INV-002 | TLA-RETRY-001 | **FAIL** - RetryFSM.tla non-executable |
| INV-003 | TLA-RETRY-002 | **FAIL** - RetryJournal.tla non-executable |
| POST-004 | TLA-RETRY-003 | **FAIL** - RetryJournal.tla non-executable |

### Verus-owned clauses covered

| Clause | Verus Obligation | Status |
|--------|-----------------|--------|
| PRE-002 | VERUS-PRE-002 | WAIVED (WAIVER-VERUS-001) |
| INV-001 | VERUS-INV-001 | WAIVED (WAIVER-VERUS-002) |
| POST-006 | VERUS-POST-006 | WAIVED (WAIVER-VERUS-003) |
| POST-001 | VERUS-POST-001 | WAIVED (WAIVER-VERUS-004) |
| PRE-004 | VERUS-PRE-004 | WAIVED (WAIVER-VERUS-005) |

### Lean/Aeneas/Hax scope

lean-contract.md correctly states no Lean obligations apply. Scope is valid.

### Proof obligations traced

15 entries in proof-obligations.jsonl. Schema is complete. 6 have waivers. 3 TLA+ obligations are non-executable. 6 remaining unit/integration obligations pass.

### TLA+ scope validity

**INVALID** - Both TLA+ specs have fatal errors preventing model checking:
- RetryFSM.tla: missing UNCHANGED in ActionFailed
- RetryJournal.tla: non-enumerable Nat quantifier

### Verus scope validity

Waivers are valid in principle (toolchain missing) but WAIVER-VERUS-001 contains a factual contradiction that must be resolved.

### Lean/Aeneas/Hax scope validity

Valid - no Lean obligations apply, correctly documented.

### Waivers validity

| Waiver | Valid? | Blocker |
|--------|--------|---------|
| WAIVER-VERUS-001 | **NO** | Factual contradiction about helpers.rs annotations |
| WAIVER-VERUS-002 | YES | Missing rerun_from field |
| WAIVER-VERUS-003 | YES | Missing rerun_from field |
| WAIVER-VERUS-004 | YES | Missing rerun_from field |
| WAIVER-VERUS-005 | YES | Missing rerun_from field |
| WAIVER-KANI-001 | YES | Missing rerun_from field |

---

## Summary

**STATUS: REJECTED**

The repaired formal obligations cannot be approved because:

1. **LETHAL**: RetryFSM.tla is non-executable - TLC cannot complete model checking due to incomplete next-state specification in ActionFailed.
2. **LETHAL**: RetryJournal.tla is non-executable - TLC cannot evaluate JournalIdempotency invariant due to non-enumerable Nat quantifier.
3. **MAJOR**: formal-waivers.jsonl missing rerun_from field on all 6 waivers.
4. **MAJOR**: WAIVER-VERUS-001 contains contradictory claims about helpers.rs annotations.
5. **MINOR**: Waiver compensating evidence overclaims test coverage as "formal verification."

**rerun_from: 3**

Required repairs before next review:
1. Fix RetryFSM.tla ActionFailed: add explicit UNCHANGED for maxAttempts, retryPolicy, runs, stepHasRetryCheck in all branches
2. Fix RetryJournal.tla JournalIdempotency: bound attempt with finite range (e.g., `0..10`)
3. Add rerun_from field to all 6 waiver entries in formal-waivers.jsonl
4. Correct WAIVER-VERUS-001 reason to match actual helpers.rs state (with Verus annotations present but toolchain missing)
5. Clarify compensating evidence: tests verify implementation, not formal properties

---

*Independent contract verification rereview by contract-verification-reviewer agent for vb-qi37.16.3 State 4.*
