# Proof Plan Review Input — vb-qi37.15.3

**Bead:** vb-qi37.15.3 — cli: Add trace command
**Phase:** State 4 review input
**Reviewers:** contract-verification-reviewer, proof-reviewer

---

## What This Bead Does

Adds a `trace` CLI command that reads a Fjall journal directory and emits all `JournalEvent` records for a given `run_id` as ordered `TraceEntry` structs, formatted as text, JSON, or JSONL.

---

## Proof Surface

### Formal Core (Verus)
Two pure functions in `crates/vb_cli/src/commands_journal.rs`:

1. **`build_trace`** — `&[JournalEvent] -> Vec<TraceEntry>` via iterator + `trace_one`
2. **`trace_one`** — `usize × &JournalEvent -> TraceEntry` via exhaustive match over all 16 `JournalEvent` variants

Both functions are `pub(crate)`, have no `unsafe`, no side effects, and no I/O.

### JournalEvent Variants (16 total, confirmed from source)

```
RunAccepted, RunAdmission, StepStarted, StepSucceeded,
ActionScheduled, ActionCompletedEvent, ActionFailedEvent,
SlotWrittenEvent, WaitScheduledEvent, AskScheduledEvent,
AskAnsweredEvent, RetryScheduledEvent, RunCancelled,
RunFinished, RunFailedEvent, RunResumed, RunRetried, RunAnswered
```

All 16 variants are covered in `trace_one` (exhaustive match, no catch-all `_`).

### Shell Layer (excluded from formal proof)
- `cmd_trace`: CLI dispatch, argument parsing, output format routing
- `read_journal_events`: Fjall journal I/O (trusted immutable input source)
- `parse_run_id`: input validation (covered by static-scan/clippy)

---

## Risk Summary

| Risk | Level | Mitigation |
|---|---|---|
| Determinism violation (same events → different trace) | medium | Verus spec + proof |
| Missing event variant (completeness) | medium | Verus exhaustive match proof |
| Index misalignment | low | Verus index-correspondence invariant |
| Wrong output format | medium | gauntlet-standard integration tests |
| Error swallowed / wrong exit code | medium | CLI integration + clippy |
| JournalEvent corruption (trusted boundary) | out-of-scope | Fjall storage layer owns this |

---

## TLA+ Waiver Justification

Trace is a **pure read-only function** over an immutable event sequence. There is:
- No state machine
- No concurrency
- No retry/lease/cancel logic
- No liveness condition beyond "events returned if they exist"
- No deadlock potential

TLA+ would add modeling overhead with zero proof value. INV-001 (determinism) is covered by Verus pure-function proofs + proptest.

---

## Obligation Status at Plan Time

| ID | Status | Evidence Required |
|---|---|---|
| TRACE-VERUS-001 | PLANNED | verus-report.md |
| TRACE-VERUS-002 | PLANNED | verus-report.md |
| TRACE-CLI-001 | PLANNED | moon-report.md |
| TRACE-CLI-002 | PLANNED | moon-report.md |
| TRACE-CLI-003 | PLANNED | moon-report.md |
| TRACE-CLI-004 | PLANNED | moon-report.md |
| TRACE-CLI-005 | PLANNED | moon-report.md |
| TRACE-CLI-006 | PLANNED | moon-report.md |
| TRACE-CLI-007 | PLANNED | moon-report.md |
| TRACE-ERR-001 | PLANNED | clippy-report.txt |
| TRACE-ERR-002 | PLANNED | moon-report.md |
| TRACE-ERR-004 | PLANNED | moon-report.md |
| TRACE-PROP-001 | PLANNED (optional) | proptest-report.md |

---

## Corrected Artifact Paths (vs. proof-obligations.jsonl)

The obligations JSON contains `crates/velvet_ballastics/src/commands_journal.rs` which does not exist. Correct path is `crates/vb_cli/src/commands_journal.rs`. Proof-writer must use the corrected path.

---

## Reviewer Checklist

- [ ] Verus spec functions (`spec_build_trace`, `spec_trace_one`) mathematically bind to actual Rust implementations (no vacuum proofs)
- [ ] Verus proofs cover all 16 `JournalEvent` variants (exhaustive match proof)
- [ ] No hardcoded `WorkflowParts` or fixed dummy data in any harness
- [ ] TRACE-PROP-001 proptest generates `JournalEvent` sequences via `kani::any()` or equivalent — not fixed dummy data
- [ ] TLA+ waiver is documented with compensating evidence (Verus + proptest)
- [ ] All waived lanes have explicit waiver reason and owner
- [ ] No `unsafe`, `unwrap`, `expect`, `panic` in `commands_journal.rs` (confirmed by discovery)
- [ ] moon ci commands reference correct crate (`vb_cli` not `velvet_ballastics`)
- [ ] Every obligation maps to a requirement or contract clause (INV-001, POST-001–007, ERR-001/002/004)
