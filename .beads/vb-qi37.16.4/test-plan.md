# Test Plan — vb-qi37.16.4

**Bead ID:** vb-qi37.16.4
**Title:** cli/runtime: Implement durable answer command
**Phase:** State 3 → State 4 (test planning)
**Contracts:** `contract.md` (98 lines), `proof-obligations.jsonl` (18 obligations), `verification-layers.md`, `traceability-matrix.jsonl`, `contract-verification-review.md` (APPROVED)
**TLA+ Model:** `specs/AskAnswerLifecycle.tla` + `AskAnswerLifecycle.cfg`

---

## 1. Scope and Targets

### 1.1 Crates Under Test
| Crate | Role |
|-------|------|
| `velvet_ballistics` | CLI surface — `Command::Answer { run_id, step, value_file, db, output }` |
| `vb_runtime` | Runtime core — `Shard::handle_ask_answer`, `AskTicket`, `AskAnswer`, `RuntimeJournalEvent::AskAnswered` |
| `vb_storage` | Journal persistence — Fjall-backed `SlotWritten` + `AskAnswered` journal events |

### 1.2 Public API Surface
```
Command::Answer { run_id, step, value_file, db, output }
AskTicket { run, step, seq, action, attempt, idempotency_key }
AskAnswer { value: SlotValue, taint: Taint }
ShardCommand::AskAnswered { run, step, ticket, answer }
Shard::handle_ask_answer(...)
RuntimeJournalEvent::AskAnswered { run, step, seq, value, taint, encoded_len }
type AnswerResult<T> = Result<T, AnswerError>
```

### 1.3 TLA+ Model Variables and Actions
| Variable | Type | Role |
|----------|------|------|
| `AskState` | `[RunId → {"idle","awaiting","answered","failed"}]` | Per-run lifecycle state |
| `PendingAnswers` | `SUBSET (RunId × StepIdx × SeqNo)` | In-memory ask-ticket set |
| `AnsweredLog` | `SEQ(EventKind × RunId × StepIdx × SeqNo)` | Journal history ("sw" = SlotWritten, "aa" = AskAnswered) |
| `SeqNoCounter` | `[RunId → Nat]` | Per-run monotonic sequence counter |

**Actions:** `Init`, `SubmitAsk`, `AnswerAsk`, `ReplayAnswer`, `AdvanceToNextStep`

---

## 2. Contract Clause Coverage Matrix

| Clause | Description | Primary Verifier | Secondary Verifier |
|--------|-------------|-----------------|-------------------|
| PRE-001 | Run in `AwaitingAsk` state | TLA-PRE-001 | INTEGRATION |
| PRE-002 | Step index matches suspended ask | TLA-PRE-001 | INTEGRATION |
| PRE-003 | Payload size ≤ `max_ipc_payload_bytes` | VERUS-PRE-003 + KANI-PRE-003 | PROPTEST-PRE-003 |
| PRE-004 | Ticket fields match (deterministic equality) | VERUS-PRE-004 | UNIT |
| PRE-005 | No duplicate (run_id, step, seq) ticket | VERUS-PRE-005 | TLA-INV-001 |
| PRE-006 | Caller validated no secret enters diagnostics unredacted | STATIC-SCAN-SECRET | INTEGRATION-PRE-006 |
| POST-001 | `SlotWritten` before `AskAnswered` journal | TLA-POST-ORDER | INTEGRATION |
| POST-002 | `AskAnswered` journal event emitted | TLA-POST-ORDER | INTEGRATION |
| POST-003 | Run transitions to next step after answer | TLA-POST-003 | UNIT |
| POST-004 | Answer survives process restart | INTEGRATION-POST-004 | MANUAL-QA |
| POST-005 | Diagnostics redact secret-tainted values | INTEGRATION-POST-005 | STATIC-SCAN-SECRET |
| INV-001 | No duplicate `AskAnswered` in journal | TLA-INV-001 | VERUS-INV-002 |
| INV-002 | `Secret`-tainted values rejected unless allowed | VERUS-INV-002 | KANI |
| INV-003 | SeqNo monotonic per run | TLA-INV-003 | UNIT |
| INV-004 | Idempotent replay (already-answered skip) | TLA-INV-004 | INTEGRATION |
| ERR-001 | `Error::RunNotFound` | INTEGRATION-ERR-VALIDATION | UNIT-ERR-ALL |
| ERR-002 | `Error::StepNotAwaitingAsk` | INTEGRATION-ERR-VALIDATION | UNIT-ERR-ALL |
| ERR-003 | `Error::TicketMismatch` | INTEGRATION-ERR-VALIDATION | UNIT-ERR-ALL |
| ERR-004 | `Error::DuplicateAnswer` | INTEGRATION-ERR-VALIDATION | UNIT-ERR-ALL |
| ERR-005 | `Error::PayloadTooLarge` | VERUS-PRE-003 + KANI-PRE-003 | INTEGRATION-ERR-VALIDATION |
| ERR-006 | `Error::ValueFileUnreadable` | INTEGRATION-ERR-VALIDATION | — |
| ERR-007 | `Error::SlotOutOfBounds` | VERUS-PRE-003 | INTEGRATION-ERR-VALIDATION |
| ERR-008 | `Error::SecretLeak` | INTEGRATION-ERR-VALIDATION | STATIC-SCAN-SECRET |

---

## 3. Unit Tests

### 3.1 Error Variant Construction (`UNIT-ERR-ALL`)

**Target file:** `crates/vb_runtime/src/shard/lifecycle.rs`
**Command:** `cargo test --lib answer_error_`
**Required for:** ERR-001 through ERR-008 (all 8 variants)

| Test Name | Clause | Input | Expected Error |
|-----------|--------|-------|---------------|
| `test_answer_error_run_not_found` | ERR-001 | Invalid `run_id` | `AnswerError::RunNotFound` |
| `test_answer_error_step_not_awaiting_ask` | ERR-002 | Run not in `AwaitingAsk` | `AnswerError::StepNotAwaitingAsk` |
| `test_answer_error_ticket_mismatch` | ERR-003 | Ticket fields don't match | `AnswerError::TicketMismatch` |
| `test_answer_error_duplicate_answer` | ERR-004 | Already-answered ticket | `AnswerError::DuplicateAnswer` |
| `test_answer_error_payload_too_large` | ERR-005 | `value_file` size > `max_ipc_payload_bytes` | `AnswerError::PayloadTooLarge` |
| `test_answer_error_value_file_unreadable` | ERR-006 | File missing or permission denied | `AnswerError::ValueFileUnreadable` |
| `test_answer_error_slot_out_of_bounds` | ERR-007 | Slot index invalid for run frame | `AnswerError::SlotOutOfBounds` |
| `test_answer_error_secret_leak` | ERR-008 | Secret-tainted value in diagnostics path | `AnswerError::SecretLeak` |

**Assertion rigor:** Each test MUST assert the exact `AnswerError` variant returned — no `is_err()` shortcuts.

### 3.2 AskTicket Equality (`VERUS-PRE-004` companion)

**Target file:** `crates/vb_runtime/src/shard/types.rs`
**Clause:** PRE-004

| Test Name | Description |
|-----------|-------------|
| `test_ticket_equality_all_fields_match` | All 6 fields (run, step, seq, action, attempt, idempotency_key) identical → equal |
| `test_ticket_equality_run_differs` | Only `run` differs → not equal |
| `test_ticket_equality_step_differs` | Only `step` differs → not equal |
| `test_ticket_equality_seq_differs` | Only `seq` differs → not equal |
| `test_ticket_equality_action_differs` | Only `action` differs → not equal |
| `test_ticket_equality_attempt_differs` | Only `attempt` differs → not equal |
| `test_ticket_equality_idempotency_key_differs` | Only `idempotency_key` differs → not equal |
| `test_ticket_equality_all_fields_zero` | All-zero fields → valid and equal to identical all-zero |
| `test_ticket_equality_max_values` | Max u64/u16/u128 values → equal to identical |

### 3.3 Duplicate Detection (`VERUS-PRE-005` companion)

**Target file:** `crates/vb_runtime/src/shard/lifecycle.rs::Shard::handle_ask_answer`
**Clause:** PRE-005, INV-001

| Test Name | Description |
|-----------|-------------|
| `test_no_duplicate_ticket_in_answered_set` | Same (run, step, seq) submitted twice → second is rejected |
| `test_duplicate_different_runs_same_step_seq` | Different runs, same step+seq → both allowed |
| `test_duplicate_different_step_same_run_seq` | Same run, different step, same seq → both allowed |
| `test_answered_set_empty_initially` | Fresh Shard → answered set empty |

### 3.4 Payload Size Check (`VERUS-PRE-003` + `KANI-PRE-003` companion)

**Target file:** `crates/vb_runtime/src/shard/lifecycle.rs::check_payload_size`
**Clause:** PRE-003, ERR-005

| Test Name | Description |
|-----------|-------------|
| `test_payload_size_exactly_at_limit` | Size == `max_ipc_payload_bytes` → Ok |
| `test_payload_size_one_byte_over` | Size == `max_ipc_payload_bytes + 1` → Err(`PayloadTooLarge`) |
| `test_payload_size_zero_bytes` | Empty value file → Ok |
| `test_payload_size_one_megabyte` | 1 MB value file against typical limit → Ok |
| `test_payload_size_two_megabytes_over_limit` | 2 MB against 1 MB limit → Err |

### 3.5 Taint Enforcement (`VERUS-INV-002` companion)

**Target file:** `crates/vb_runtime/src/shard/lifecycle.rs::Shard::handle_ask_answer`
**Clause:** INV-002, ERR-008

| Test Name | Taint Input | ResourceContract Allows Secrets | Expected |
|-----------|-------------|-------------------------------|----------|
| `test_taint_clean_accepted` | `Taint::Clean` | false | Ok |
| `test_taint_derived_accepted` | `Taint::DerivedFromSecret` | false | Ok |
| `test_taint_secret_rejected_without_permission` | `Taint::Secret` | false | Err(`SecretLeak`) |
| `test_taint_secret_accepted_with_permission` | `Taint::Secret` | true | Ok |

### 3.6 State Transition (`TLA-POST-003` companion)

**Target file:** `crates/vb_runtime/src/shard/lifecycle.rs`
**Clause:** POST-003

| Test Name | Description |
|-----------|-------------|
| `test_run_transitions_from_awaiting_to_answered` | After answer, `AskState[run] = answered` |
| `test_run_advances_to_next_step_after_answer` | After `AdvanceToNextStep`, run is `idle` again at next step |
| `test_seqno_increments_on_answer` | `SeqNoCounter[run]` increments by exactly 1 per answer |

### 3.7 Slot Value Write Ordering (`TLA-POST-ORDER` companion)

**Target file:** `crates/vb_runtime/src/shard/lifecycle.rs`
**Clause:** POST-001, POST-002

| Test Name | Description |
|-----------|-------------|
| `test_slot_written_emitted_before_ask_answered` | Journal append order: SlotWritten record appears at index N, AskAnswered at N+1 |
| `test_ask_answered_contains_correct_value_and_taint` | Journal event fields match `AskAnswer` input |

---

## 4. Integration Tests

**Target file:** `crates/velvet_ballistics/tests/cli_integration.rs`
**Command prefix:** `cargo test --test cli_integration`

### 4.1 Durable Answer — End-to-End (`INTEGRATION-POST-004`)

**Clause:** POST-004
**Test name:** `ask_answer_durable`

**Scenario (Given/When/Then):**
```
Given a run suspended at an Ask step in AwaitingAsk state
  And the run's AskTicket (run_id, step, seq) is recorded
When the operator invokes:
  velvet_ballistics answer --run-id <run_id> --step <step> --value-file <file> --db <db_path> --output <out>
Then the process exits 0
  And the run resumes from the next step index after process restart
```

**Evidence requirement:** Journal replay log shows `AskAnswered` entry for the ticket, and run state is `answered` after restart.

### 4.2 Secret Redaction in Diagnostics (`INTEGRATION-POST-005`)

**Clause:** POST-005
**Test name:** `ask_answer_secret_redaction`

**Scenario:**
```
Given a run in AwaitingAsk state with an AskTicket
  And the answer value has Taint::Secret
When the answer command is processed
Then the diagnostics trace output contains NO raw Secret-tainted SlotValue
  And the trace contains a redacted placeholder (e.g., "[REDACTED]", "***", or SlotValue::Redacted)
```

**Verification method:** Capture `trace!` macro output (via `tracing::Span` instrumentation or test subscriber) and assert the redacted form appears with no plaintext secret bytes.

### 4.3 Diagnostics Safety Validation (`INTEGRATION-PRE-006`)

**Clause:** PRE-006
**Test name:** `ask_answer_diagnostics_safe`

**Scenario:**
```
Given a caller invokes handle_ask_answer with a Taint::Secret value
When the diagnostics path is exercised
Then the function returns Err(SecretLeak) BEFORE emitting any trace event
  And no trace record with secret payload is emitted
```

### 4.4 Error Variant Exhaustive Validation (`INTEGRATION-ERR-VALIDATION`)

**Clause:** ERR-001 through ERR-008
**Test prefix:** `answer_error_`

| Test Name | Setup | Expected Error |
|-----------|-------|---------------|
| `answer_error_run_not_found` | Submit answer for non-existent `run_id` | `RunNotFound` |
| `answer_error_step_not_awaiting_ask` | Submit answer for run in `idle` state | `StepNotAwaitingAsk` |
| `answer_error_ticket_mismatch` | Submit answer with wrong `seq` | `TicketMismatch` |
| `answer_error_duplicate_answer` | Submit same answer twice | `DuplicateAnswer` |
| `answer_error_payload_too_large` | Write value file > 1 GiB | `PayloadTooLarge` |
| `answer_error_value_file_unreadable` | Point to non-existent file | `ValueFileUnreadable` |
| `answer_error_slot_out_of_bounds` | Target invalid slot index | `SlotOutOfBounds` |
| `answer_error_secret_leak` | Answer with secret value when contract disallows | `SecretLeak` |

**Assertion:** Each test MUST inspect the error variant returned (not just exit code).

### 4.5 Idempotent Journal Replay (`INTEGRATION-POST-004` + `TLA-INV-004`)

**Clause:** INV-004, POST-004
**Test name:** `ask_answer_idempotent_replay`

**Scenario:**
```
Given a run that was answered and the journal contains AskAnswered for (run_id, step, seq)
When journal replay is triggered for the same (run_id, step, seq)
Then the replay is a no-op (no duplicate AskAnswered appended)
  And the run remains in answered state
  And no SlotWritten is re-emitted
```

### 4.6 SlotWritten Precedes AskAnswered (`INTEGRATION-POST-001`)

**Clause:** POST-001, POST-002
**Test name:** `ask_answer_journal_ordering`

**Verification:** After answering, iterate the Fjall journal entries for the run and assert the first matching `SlotWritten` entry for the ticket appears at a lower sequence number than the corresponding `AskAnswered` entry.

### 4.7 State Transition on Answer (`INTEGRATION-POST-003`)

**Clause:** POST-003
**Test name:** `ask_answer_state_transition`

**Scenario:**
```
Given a run at step N in AwaitingAsk
When a valid answer is submitted
Then AskState transitions to answered
  And after AdvanceToNextStep, AskState becomes idle
  And the run's current step is N+1
```

---

## 5. Property-Based Tests (Proptest)

**Command:** `cargo test --lib proptest_payload_size`
**Target:** `crates/vb_runtime/src/shard/lifecycle.rs::check_payload_size`
**Clause:** PRE-003 (ERR-005)
**Waiver basis:** `KANI-PRE-003` provides formal bounded model checking; proptest provides empirical fuzz over the same function.

### 5.1 Payload Size Boundary Proptest

**Strategy:** `proptest::propUCT` or `proptest::vec` generating byte vectors of arbitrary length

| Property | Input Space | Assertion |
|----------|-------------|-----------|
| `proptest_payload_size_exact_limit` | `size == max_ipc_payload_bytes` | `Ok(())` |
| `proptest_payload_size_under_limit` | `size in 0..max_ipc_payload_bytes` | `Ok(())` |
| `proptest_payload_size_over_limit` | `size in max_ipc_payload_bytes+1..2*max_ipc_payload_bytes` | `Err(PayloadTooLarge)` |
| `proptest_payload_size_max_values` | Arbitrary u32 up to `u32::MAX` | No panic, deterministic result |

**Iterations:** 10,000 minimum per strategy.
**Fuzzing directives:** No `unwrap()`, no `expect()` in the function under test.

---

## 6. TLA+ Model-Checking Verification

**Command:** `tlc -config specs/AskAnswerLifecycle.cfg specs/AskAnswerLifecycle.tla`
**Expected:** TLC reports no invariant violation; no deadlock; temporal properties `EventuallyAnswered` and `EventuallyAdvanced` satisfied.

### 6.1 Invariant Checks

| Invariant | TLA+ Definition | What It Proves |
|-----------|----------------|----------------|
| `NoDuplicateAskAnswered` | No "(aa, run, step, seq)" appears twice in AnsweredLog | INV-001: No duplicate `AskAnswered` events |
| `ValidAskState` | `AskState[run]` ∈ {"idle","awaiting","answered","failed"} | Protocol well-formedness |
| `PendingSubset` | `PendingAnswers ⊆ RunId × StepIdx × SeqNo` | Protocol well-formedness |
| `MonotonicSeqNo` | `SeqNoCounter[run] >= 0` | INV-003: SeqNo non-negative |
| `AnswerPersistenceOrder` | Every "aa" entry has a preceding "sw" entry with same (run, step, seq) at lower index | POST-001+POST-002 ordering |

### 6.2 Temporal Property Checks

| Property | TLA+ Definition | What It Proves |
|----------|----------------|----------------|
| `EventuallyAnswered` | `awaiting ~> (answered ∨ failed)` | Every ask is eventually answered or failed |
| `EventuallyAdvanced` | `answered ~> idle` | Every answered run eventually advances |

### 6.3 Enabling Condition Checks (PRE-001, PRE-002)

- `AnswerAsk` is enabled ONLY when `AskState[run] = "awaiting"` — directly encodes PRE-001
- `PendingAnswers` contains the ticket — directly encodes PRE-002

---

## 7. Verus Formal Proof Obligations

**Command:** `verus crates/vb_runtime/src/shard/lifecycle.rs`

### 7.1 VERUS-INV-002 — Taint Enforcement on Slot Write

**Spec function:** `spec_taint_ok(value: SlotValue, taint: Taint, contract: &ResourceContract) -> bool`
**Proof:** `proof_answer_preserves_invariants`
**Invariants verified:**
- `taint_ok_write`: If `handle_ask_answer` returns `Ok`, then either `taint != Secret` or `contract.allows_secret_results = true`
- `no_secret_leak`: `Secret`-tainted values are never written to slot without explicit contract permission

### 7.2 VERUS-PRE-004 — Ticket Equality Determinism

**Spec function:** `spec_ticket_matches(ticket: &AskTicket, run: RunId, step: StepIdx, seq: SeqNo) -> bool`
**Proof:** `proof_ticket_equality_deterministic`
**Invariants verified:**
- `field_equality_deterministic`: Each of the 6 `AskTicket` fields is compared with `==`; no panics

### 7.3 VERUS-PRE-005 — Duplicate Detection

**Spec function:** `spec_not_duplicate(ticket: &(RunId, StepIdx, SeqNo), answered: &Set) -> bool`
**Proof:** `proof_no_duplicate_in_answered_set`
**Invariants verified:**
- `no_duplicate_ticket`: If `handle_ask_answer` returns `Ok`, the ticket was not in the answered set

### 7.4 VERUS-PRE-003 — Payload Size Bound

**Spec function:** `spec_payload_size_ok(size: u32, max: u32) -> bool`
**Proof:** `proof_payload_size_bound`
**Invariants verified:**
- `size_bound_no_overflow`: `check_payload_size` returns `Ok` iff `size <= max`

---

## 8. Kani Bounded Model Checking

**Command:** `cargo kani --harness check_payload_size --contract`
**Target:** `crates/vb_runtime/src/shard/lifecycle.rs::check_payload_size`
**Clause:** PRE-003 (ERR-005)

**Claim:** For all arbitrary `u32` values of `value_file_size` and `max_ipc_payload_bytes`, the comparison `size <= max` is safe with no overflow.

**Expected result:** Kani reports all paths safe, no overflow, no out-of-bounds.

---

## 9. Static Analysis — Secret Leak Gate

**Command:** `cargo clippy --workspace --lib --bins -- -D warnings`
**Target:** `crates/vb_runtime/src/trace.rs`
**Clauses:** PRE-006, POST-005, ERR-008

### 9.1 Clippy Lint Gate

| Rule | Target | Enforced Property |
|------|--------|-------------------|
| `suspicious` | Raw `SlotValue::Secret` construction in `trace!` call path | No secret reaches trace macro without redaction |
| `perf` | `.unwrap()`/`.expect()` in hot diagnostics path | No panic in trace emission |

### 9.2 No-raw-Secret Path Assertion

Evidence that the following call chain is taint-gated:
```
handle_ask_answer
  → emit_trace_event(..., value, taint)
    → if taint == Secret { emit_redacted(...) }
      else { emit_plain(value, ...) }
```

Verify: No code path exists where `SlotValue::Secret` reaches `trace!` without passing through the taint-check gate in `trace.rs`.

---

## 10. Journal Replay Tests

**Target:** Full replay determinism for already-answered tickets

### 10.1 Replay Skips Already-Answered Ticket (`TLA-INV-004` companion)

**Test:** `test_journal_replay_skips_already_answered_ticket`

```
Given a Fjall journal containing:
  1. SlotWritten(run=1, step=2, seq=5, value, taint)
  2. AskAnswered(run=1, step=2, seq=5, value, taint)
When replay processes the journal
Then no additional entries are appended
  And AskState[1] remains "answered"
```

### 10.2 Replay Restores Answered State (`INTEGRATION-POST-004` companion)

**Test:** `test_journal_replay_restores_answered_state`

```
Given the process was killed after AskAnswered was journaled but before AdvanceToNextStep
When the runtime restarts and replays the journal
Then the run is restored to AskState = "answered"
  And the slot value is present in the run's frame
```

### 10.3 SeqNo Restored Correctly on Replay

**Test:** `test_journal_seqno_always_increments`

```
Given SeqNoCounter[run] = N after an answer
When replay runs
Then SeqNoCounter[run] = N (not reset to 0)
  And subsequent answer increments to N+1
```

---

## 11. Redaction Verification Tests

### 11.1 Secret Value Never Appears Plaintext in Trace

**Test:** `test_trace_output_contains_no_secret_taint`
**Clause:** POST-005, PRE-006

```
Given answer with SlotValue::Secret("super-secret-key") and Taint::Secret
When diagnostics are emitted
Then the trace subscriber receives:
  - Either: nothing (trace suppressed for secret)
  - Or: redacted form "[REDACTED]" / "***" / SlotValue::Redacted
And the trace NEVER contains the bytes "super-secret-key"
```

### 11.2 DerivedFromSecret Passes Through Unredacted

**Test:** `test_derived_from_secret_not_redacted`
**Clause:** POST-005

```
Given answer with SlotValue::DerivedFromSecret and Taint::DerivedFromSecret
When diagnostics are emitted
Then the trace contains the actual slot value bytes (not redacted)
```

### 11.3 Clean Value Passes Through Unredacted

**Test:** `test_clean_value_not_redacted`
**Clause:** POST-005

```
Given answer with SlotValue::Clean and Taint::Clean
When diagnostics are emitted
Then the trace contains the actual slot value bytes
```

---

## 12. BDD Scenario Specifications

### 12.1 Scenario: Successful Durable Answer

**Feature:** Durable Answer Command

```
Scenario: Operator answers a suspended ask and run resumes after restart
  Given a run "run-42" is suspended at step 3 in AwaitingAsk state
    And the AskTicket is (run=42, step=3, seq=7, action=Ask, attempt=0, idempotency_key=0xABCD)
    And the value file "/tmp/answer-val.bin" contains 1024 bytes
    And the database is at "/var/lib/vb/run-42.db"
    And ResourceContract allows secret results = false
  When the operator runs:
    velvet_ballistics answer --run-id 42 --step 3 --value-file /tmp/answer-val.bin --db /var/lib/vb/run-42.db --output /tmp/run-42.out
  Then the command exits with status 0
    And the file "/tmp/run-42.out" contains the answer value
    And the journal contains in order:
      - SlotWritten(run=42, step=3, seq=7, value, taint=Clean)
      - AskAnswered(run=42, step=3, seq=7, value, taint=Clean, encoded_len=1024)
    And the run's AskState is "answered"
  Given the process restarts
  When the runtime replays the journal
  Then run "run-42" resumes at step 4
    And the slot value at step 3 is "answered"
```

### 12.2 Scenario: Duplicate Answer Rejected

**Feature:** Answer Idempotence

```
Scenario: Operator attempts to answer the same ticket twice
  Given a run "run-42" was already answered at (run=42, step=3, seq=7)
    And the journal already contains AskAnswered for that ticket
  When the operator attempts to answer the same ticket again
  Then the command exits with non-zero status
    And the error is "DuplicateAnswer"
    And the journal is unchanged (no new entries appended)
```

### 12.3 Scenario: Secret Value Rejected from Diagnostics

**Feature:** Secret Redaction

```
Scenario: Operator submits secret-tainted answer but diagnostics would leak it
  Given a run "run-42" is in AwaitingAsk state
    And the answer value has Taint::Secret
    And the workflow's ResourceContract does NOT allow secret results
  When handle_ask_answer is called
  Then the function returns Err(SecretLeak)
    And no trace event containing the secret value is emitted
    And the journal is not updated
```

### 12.4 Scenario: Invalid Ticket Returns Error

**Feature:** Answer Validation

```
Scenario: Operator presents wrong ticket seqno
  Given a run "run-42" is in AwaitingAsk at step 3 with seq=7
  When the operator submits an answer with seq=99 (does not match)
  Then the command returns TicketMismatch error
    And the run remains in AwaitingAsk state
```

### 12.5 Scenario: Payload Exceeds Limit

**Feature:** Payload Size Enforcement

```
Scenario: Operator submits value file exceeding max_ipc_payload_bytes
  Given max_ipc_payload_bytes = 1048576 (1 MiB)
    And the value file contains 1048577 bytes
  When the answer command is invoked
  Then the function returns PayloadTooLarge error
    And no journal entry is written
```

---

## 13. Manual QA Expectations

### 13.1 Hands-On Durability Check

**Step 1:** Start a long-running workflow that hits an ask step.
**Step 2:** Observe the run enters `AwaitingAsk` state.
**Step 3:** Submit an answer via `velvet_ballistics answer ...`.
**Step 4:** Verify answer is written to slot and acknowledged.
**Step 5:** Kill the `vb_runtime` process with `kill -9`.
**Step 6:** Restart `vb_runtime`.
**Step 7:** Verify the run resumes from the step AFTER the ask (not from the ask itself).
**Step 8:** Verify the answer value is present in the resumed run's frame.

**Pass criteria:** Run resumes correctly; no duplicate `AskAnswered` in journal; answer value intact.

### 13.2 Manual Secret Leak Check

**Step 1:** Configure a workflow that allows `Taint::Secret` answers.
**Step 2:** Submit an answer with `Taint::Secret` value.
**Step 3:** Inspect all trace output (logs, OTLP export, console).
**Step 4:** Verify no trace line contains raw secret bytes.
**Step 5:** Verify `[REDACTED]` or equivalent placeholder appears.

**Pass criteria:** Secret value never appears in plaintext in any diagnostic output.

### 13.3 Error Surface Manual Check

For each error variant (ERR-001 through ERR-008), trigger the condition manually and verify the exact error variant is returned with actionable context (run_id, step, seq in error message where applicable).

---

## 14. Test Execution Order and Dependencies

### 14.1 Phased Execution

**Phase 1 — Fastest feedback first:**
1. `cargo clippy --workspace` (static analysis — PRE-006, POST-005, ERR-008)
2. `cargo test --lib` (unit tests — all UNIT-* tests)
3. `verus crates/vb_runtime/src/shard/lifecycle.rs` (VERUS-* proofs)
4. `cargo kani --harness check_payload_size --contract` (KANI-PRE-003)

**Phase 2 — Integration:**
5. `cargo test --test cli_integration` (all INTEGRATION-* tests)

**Phase 3 — Model checking:**
6. `tlc -config specs/AskAnswerLifecycle.cfg specs/AskAnswerLifecycle.tla` (TLA-* obligations)

**Phase 4 — Property fuzzing:**
7. `cargo test --lib proptest_payload_size` (PROPTEST-PRE-003)

**Phase 5 — Manual QA:**
8. Hands-on durability and secret redaction verification

### 14.2 Prerequisite Gates

- Phase 2 integration tests MUST NOT run until Phase 1 passes (clippy clean, unit tests green, Verus verified, Kani clean).
- TLA+ model checker MUST be run in an environment with Java (TLC requirement).
- Manual QA MUST NOT proceed until Phase 2 integration tests pass.

---

## 15. Traceability Checklist

Each test listed above traces to exactly one `traceability-matrix.jsonl` entry and one `proof-obligations.jsonl` entry:

| Test | proof_obligation_id | contract_clause |
|------|--------------------|-----------------|
| `test_answer_error_run_not_found` | INTEGRATION-ERR-VALIDATION | ERR-001 |
| `test_answer_error_step_not_awaiting_ask` | INTEGRATION-ERR-VALIDATION | ERR-002 |
| `test_answer_error_ticket_mismatch` | INTEGRATION-ERR-VALIDATION | ERR-003 |
| `test_answer_error_duplicate_answer` | INTEGRATION-ERR-VALIDATION | ERR-004 |
| `test_answer_error_payload_too_large` | KANI-PRE-003, INTEGRATION-ERR-VALIDATION | ERR-005 |
| `test_answer_error_value_file_unreadable` | INTEGRATION-ERR-VALIDATION | ERR-006 |
| `test_answer_error_slot_out_of_bounds` | INTEGRATION-ERR-VALIDATION | ERR-007 |
| `test_answer_error_secret_leak` | INTEGRATION-ERR-VALIDATION, STATIC-SCAN-SECRET | ERR-008 |
| `test_ticket_equality_deterministic` | VERUS-PRE-004 | PRE-004 |
| `test_no_duplicate_ticket_in_answered_set` | VERUS-PRE-005, TLA-INV-001 | PRE-005, INV-001 |
| `test_payload_size_*` | KANI-PRE-003, VERUS-PRE-003 | PRE-003 |
| `proptest_payload_size_*` | PROPTEST-PRE-003 | PRE-003 |
| `test_taint_*` | VERUS-INV-002 | INV-002 |
| `test_slot_written_emitted_before_ask_answered` | TLA-POST-ORDER | POST-001 |
| `test_answer_emits_ask_answered_journal_event` | TLA-POST-ORDER | POST-002 |
| `test_answer_transitions_run_to_next_step` | TLA-POST-003 | POST-003 |
| `ask_answer_durable` | INTEGRATION-POST-004 | POST-004 |
| `ask_answer_secret_redaction` | INTEGRATION-POST-005 | POST-005 |
| `ask_answer_diagnostics_safe` | INTEGRATION-PRE-006 | PRE-006 |
| `test_no_duplicate_ask_answered_in_journal` | TLA-INV-001 | INV-001 |
| `test_seqno_monotonic_per_run` | TLA-INV-003 | INV-003 |
| `ask_answer_idempotent_replay` | TLA-INV-004 | INV-004 |
| `test_journal_replay_skips_already_answered_ticket` | TLA-INV-004, INTEGRATION-POST-004 | INV-004 |
| `test_trace_output_contains_no_secret_taint` | INTEGRATION-POST-005, STATIC-SCAN-SECRET | POST-005 |
| `test_run_transitions_from_awaiting_to_answered` | TLA-POST-003 | POST-003 |
| `test_seqno_increments_on_answer` | TLA-INV-003 | INV-003 |

---

## 16. Non-Goals (Explicitly Out of Scope)

- Workflow compilation or IR validation (separate bead)
- `ActionCompleted` action path (separate bead)
- Multi-step taint propagation across workflows (separate bead)
- Algebraic semantics of the full `Taint` lattice beyond the three-level lattice (separate bead)
- Fjall storage correctness in isolation (covered by integration test + `fjall` skill review)
- Performance benchmarks (answer command is not on a hot path)
- IPC transport reliability (third-party; covered by integration test)

---

**Plan authored:** 2026-05-11
**Bead:** vb-qi37.16.4
**Verification reviewer:** contract-verification-reviewer (approved State 4)
**Test plan version:** 1.0
