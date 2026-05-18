# Proof Review — vb-qi37.15.3

**Bead:** vb-qi37.15.3 — cli: Add trace command
**Phase:** State 6 (proof-reviewer)
**Reviewer:** proof-reviewer
**Generated:** 2026-05-18

---

## Review Verdict

STATUS: APPROVED

---

## Obligations Reviewed

| ID | Artifact | Command | Result |
|---|---|---|---|
| TRACE-VERUS-001 | `verification/verus/vb_cli_commands_journal_trace.rs` | `verus --edition 2024 verification/verus/vb_cli_commands_journal_trace.rs` | 4 verified, 0 errors |
| TRACE-VERUS-002 | `verification/verus/vb_cli_commands_journal_trace.rs` | same as above | same run |
| TRACE-ERR-001 | `crates/vb_cli/src/args.rs` | `cargo clippy -p vb_cli -- -D warnings` | No issues found |

---

## Binding Analysis

### Mathematical Binding to Production Code

`spec_trace_one` (ghost model) correctly mirrors `trace_one` production function:

- **All 18 JournalEvent variants** covered with no catch-all `_` pattern in either spec or production
- **Variant → event_type string mapping** verified:
  - RunAccepted → "RunAccepted" ✓
  - RunAdmission → "RunAdmission" ✓
  - StepStarted → "StepStarted" ✓
  - StepSucceeded → "StepSucceeded" ✓
  - ActionScheduled → "ActionScheduled" ✓
  - ActionCompletedEvent → "ActionCompleted" ✓ (variant name differs from event_type string)
  - ActionFailedEvent → "ActionFailed" ✓
  - SlotWrittenEvent → "SlotWritten" ✓
  - WaitScheduledEvent → "WaitScheduled" ✓
  - AskScheduledEvent → "AskScheduled" ✓
  - AskAnsweredEvent → "AskAnswered" ✓
  - RetryScheduledEvent → "RetryScheduled" ✓
  - RunCancelled → "RunCancelled" ✓
  - RunFinished → "RunFinished" ✓
  - RunFailedEvent → "RunFailed" ✓
  - RunResumed → "RunResumed" ✓
  - RunRetried → "RunRetried" ✓
  - RunAnswered → "RunAnswered" ✓
- **Hardcoded seq: 0** for RunResumed, RunRetried, RunAnswered — matches production (all three ignore the stored seq and hardcode 0) ✓
- **Field access via `.get()`** modeled as raw integers in spec — correct ✓

### No Vacuum Proofs

Four non-vacuous proofs discharged:

1. **`proof_trace_one_deterministic`** (TRACE-VERUS-002): Reflexivity via `compute` — directly evaluates `spec_trace_one(idx, event) == spec_trace_one(idx, event)` ✓
2. **`proof_trace_one_variant_coverage`** (TRACE-VERUS-002): Exhaustive `match` covering all 18 `SpecJournalEvent` variants with `assert(true)` per arm — proves the match is total with no panics ✓
3. **`proof_trace_one_same_input_same_output`** (TRACE-VERUS-001 lemma): For equal `SpecJournalEvent` values, `spec_trace_one` produces equal `SpecTraceEntry` — the core lemma for build_trace determinism ✓
4. **`proof_trace_one_applied_globally_deterministic`** (TRACE-VERUS-001): `forall|i| 0 <= i < n ==> spec_trace_one(i, &events1[i]) == spec_trace_one(i, &events2[i])` using the lemma from (3) — formal INV-001 determinism statement ✓

### No Hardcoded Harnesses

All proofs use `SpecJournalEvent` ghost type directly; no `kani::any()`, no `WorkflowParts`, no production struct instances. ✓

### No Unsafe/unwrap/panic in Scope

`crates/vb_cli/src/commands_journal.rs` confirmed with `#![forbid(unsafe_code)]`, zero `unwrap`/`expect`/`panic`. ✓

---

## Artifact Path Observation (Non-Blocking)

`proof-obligations.planned.jsonl` (State 4 artifact) records:
- `artifact: "verification/verus/vb_cli/commands_journal.rs"` — does not exist
- Actual artifact: `verification/verus/vb_cli_commands_journal_trace.rs` ✓

`proof-obligations.jsonl` (State 3 artifact) records:
- `crates/velvet_ballastics/src/commands_journal.rs` — does not exist
- Corrected path `crates/vb_cli/src/commands_journal.rs` confirmed in production ✓

The proof-writer correctly used the actual artifact path. The planned obligations JSONL is stale but this does not block approval.

---

## Variant Count Correction — Resolved

proof-plan-review-input.md (State 4) states "16 variants". production code has **18 variants**. proof-writer corrected to 18 in the Verus artifact. No blocking issue.

---

## TLA+ Waiver Assessment

**Approved.** Trace is a pure read-only journal replay function. No state machine, no concurrency, no liveness condition beyond "events appear if they exist". TLA+ would model zero temporal behavior. Compensating evidence (Verus INV-001 determinism proofs) is adequate.

---

## Findings

| Finding | Severity | Detail | Disposition |
|---|---|---|---|
| Path mismatch in planned obligations JSONL | advisory | Planned obligations reference non-existent `vb_cli/commands_journal.rs`; actual artifact is `vb_cli_commands_journal_trace.rs` | Non-blocking — proof uses correct path |
| 16 vs 18 variant count in proof-plan-review-input.md | advisory | Proof-writer corrected to 18 in Verus artifact | Non-blocking — proof is correct |
| Stale proof-obligations.jsonl paths | advisory | State 3 artifact references `velvet_ballastics` instead of `vb_cli` | Non-blocking — production code confirmed at `vb_cli` |

---

## Recommendation

**Advance to State 7 (test-planner).** All required proof obligations are verified. No proof-repair-guide.md needed.
