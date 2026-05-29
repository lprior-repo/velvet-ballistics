# Implementation Contract — vb-282my

**Bead:** vb-282my (P1)
**Title:** Add refinement harnesses or waivers for repaired TLA bridge (7 RRO rows)
**Contract Version:** 1.0
**Date:** 2026-05-29
**Status:** DRAFT — pending proof-planner lane decisions

## Contract Scope

This contract defines the implementation obligations for closing the `TLA-BRIDGE-REFINEMENT-HARNESS-GAP` finding. It covers 7 Rust Refinement Obligation (RRO) rows in `verification/tla/rust-refinement-obligations.jsonl`. The contract is binding on all downstream agents: proof-planner, proof-writer, proof-reviewer, formal-verifier, and the implementation agent.

## Contract Principles

### CP-1: No TLA-Only Closure
A TLA+ model passing TLC under bounded configuration provides temporal-design evidence only. It is **never** sufficient as Rust implementation proof. Every RRO row must close with either a refinement harness or an approved proportional waiver.

### CP-2: Harness Over Waiver
Refinement harnesses are the default closure path. Proportional waivers are acceptable **only** when:
- The claim is `behavior_affecting: false`
- TLC + behavior-test evidence is demonstrably sufficient
- The waiver has independent reviewer approval with a future expiry date

### CP-3: Independent Verification
Every harness and waiver must be reviewed by an independent agent invocation. Self-approval is categorically forbidden. The `proof-reviewer` agent that writes the bridge verdict MUST NOT be the same agent invocation that wrote the harness or drafted the waiver.

### CP-4: Full Claim Coverage
A refinement harness must cover the **full** TLA+ claim, not a subset. Partial coverage (like the existing RetryFSM harness covering monotonicity only) is a blocking gap. If a claim has sub-claims, each sub-claim must be explicitly covered or explicitly waived.

### CP-5: Production Code Targeting
All refinement harnesses must target production Rust functions or extracted production helpers. A harness verifying a model-only copy of the code is vacuous (GOD RULE 2). Every `source_ref` in the RRO row must have corresponding harness coverage.

### CP-6: Append-Failure Path Coverage
For RRO rows that involve journal append ordering (ASK-ANSWER-001, ADMISSION-001, RESUME-001), harnesses MUST exercise both the success path (`append → Ok(())`) and the failure path (`append → Err(...)`). A harness that only verifies the success path is incomplete.

## Per-RRO Contract Clauses

### CC-1: CHOOSE-LOWERING-001 — Fanout and Empty-Branch Enforcement

**Claim:** Compile-time Choose lowering enforces fanout and empty-branch rejection, resolves canonical otherwise labels, records condition slots, and lowers canonical branch targets.

**Implementation Obligations:**

1. **OC-1.1 — Fanout Limit**
   - Write a Kani harness that verifies: for any input with `branches.len() > 64`, `lower_canonical_choose` returns `Err(PrimitiveLoweringLimitExceeded)`.
   - Unwind bound: 65 (to exercise the branch-count check).
   - Use `kani::any::<usize>()` for branch count; use `kani::assume(branches.len() > 64)`.

2. **OC-1.2 — Empty Branch Table Rejection**
   - Verify: for any input with `branches.is_empty()` and `otherwise.is_none()`, `lower_canonical_choose` returns `Err(EmptyBranchTable)`.
   - Verify: with `branches.len() == 0` and `otherwise.is_some()`, lowering succeeds (empty table with otherwise is valid).

3. **OC-1.3 — Non-empty Branch Bodies Rejection**
   - Verify: for any input where a branch has a non-empty `steps` field, `lower_canonical_choose` returns `Err(UnsupportedStepPrimitive)`.

4. **OC-1.4 — Label Resolution**
   - Verify: `otherwise` label that exists in `step_names` resolves to correct `StepIdx`.
   - Verify: unknown otherwise label returns `Err(UnknownStepLabel)`.

5. **OC-1.5 — Condensed Verification**
   - Target: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:216-293::lower_canonical_choose`
   - Harness file: `crates/vb_compile/src/verification/kani/kani_choose_lowering.rs` (new)
   - Evidence command: `cargo kani -p vb_compile --harness kani_choose_lowering_*`

### CC-2: CHOOSE-REPLAY-001 — Branch Selection and Otherwise Fallback

**Claim:** Runtime ChooseSlot replay selects the first true branch, falls back to otherwise when all branches are false, and errors when no branch and no otherwise target exist.

**Implementation Obligations:**

1. **OC-2.1 — True Branch Selection**
   - Write a Kani harness that verifies: with at least one branch whose slot value is `Bool(true)`, `replay_choose_slot` returns `Ok(Continue(target_of_first_true_branch))` and `run.pc()` equals that target.

2. **OC-2.2 — Otherwise Fallback**
   - Verify: with all branches having `Bool(false)` value and `otherwise.is_some()`, function returns `Ok(Continue(otherwise_target))`.

3. **OC-2.3 — No-Match Error**
   - Verify: with all branches `Bool(false)` and `otherwise.is_none()`, function returns `Err(Internal{reason: "choose_slot no branch matched and no otherwise"})`.

4. **OC-2.4 — Non-Boolean Condition Error**
   - Verify: with any branch having a non-boolean slot value (e.g., `I64(42)`), function returns `Err(Internal{reason: "choose_slot condition is not boolean"})`.

5. **OC-2.5 — Condensed Verification**
   - Target: `crates/vb_core/src/replay/choose.rs:12-58::replay_choose_slot`
   - Harness file: `crates/vb_core/src/replay/kani_choose_replay.rs` (new) or `crates/vb_core/src/verification/kani/kani_choose_replay.rs`
   - Evidence command: `cargo kani -p vb_core --harness kani_choose_replay_*`

### CC-3: ASK-ANSWER-001 — Journal-Monotonic Timer and Answer Lifecycle

**Claim:** Ask-answer states require a matching pending ask timer before answer, emit SlotWritten before AskAnswered, preserve per-run journal sequence monotonicity, and never expose a pending ask timer unless AskScheduled was journaled successfully.

**Implementation Obligations:**

1. **OC-3.1 — Append-Before-Insert Ordering (CRITICAL)**
   - Write a Kani harness that verifies: in `await_timer`, the `AskScheduled` journal append is attempted BEFORE `pending_timers.insert()`. If append succeeds, the timer is inserted. If append fails, `pending_timers` is NOT modified.
   - This corresponds to hazard HZ-REF-001 and HZ-UNSOUND-002.
   - Must stub `append_journal_event` to return both `Ok(())` and `Err(JournalError::...)`.

2. **OC-3.2 — Pending Timer Guard**
   - Verify: in `handle_ask_answer`, if `pending_timers.get(&run)` is `None` or has wrong `step`/`kind`, function returns `Err(InvalidActionCompletion)`.

3. **OC-3.3 — SlotWritten Before AskAnswered**
   - Verify: in `handle_ask_answer`, the `SlotWritten` journal append (line 38-44) precedes the `AskAnswered` journal append (line 50-54) in execution order. If `SlotWritten` append fails, `AskAnswered` is never attempted.

4. **OC-3.4 — Journal Sequence Monotonicity**
   - Verify: each successful journal append in `handle_ask_answer` increments the per-run sequence counter. No two journal events for the same run have the same sequence number.

5. **OC-3.5 — Condensed Verification**
   - Target: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:2-62::handle_ask_answer` AND `crates/vb_runtime/src/shard/transitions.rs:123-162::await_timer`
   - Harness file: `crates/vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs` (new)
   - Evidence command: `cargo kani -p vb_runtime --harness kani_ask_answer_*`

### CC-4: RETRY-FSM-001 — Full Retry Exhaustion and Terminal Typing

**Claim:** Retryable failures eventually exhaust under weak fairness; no retry occurs after max attempts; exhausted retries remain typed and terminal.

**Existing Partial Coverage:**
- `kani_retry_attempt_monotonicity` covers monotonicity only
- `kani_ticket_retry_capacity_bounds` covers capacity bounds
- `kani_retry_attempt_overflow_fail_closed` covers overflow fail-closed

**Remaining Gaps:**
- No proof that after `max_attempts`, `record_retry_attempt` returns `Ok(false)` (no more retries)
- No proof that exhausted retries result in typed terminal states
- No proof of the exhaustion loop: repeated calls converge to `Ok(false)` or `Err`

**Implementation Obligations:**

1. **OC-4.1 — Extend Kani Harness for Exhaustion**
   - Add a Kani proof that verifies: when `action_attempts[step] >= policy.max_attempts`, `record_retry_attempt` returns `Ok(false)` and does NOT increment the attempt counter.
   - This covers the "no retry after max" subclaim.

2. **OC-4.2 — Verify Terminal Typing**
   - Add a Kani proof that verifies: when `record_retry_attempt` returns `Ok(false)`, the attempt counter is at `max_attempts` and the error state (if any) is identifiable.
   - Or verify: after `Ok(false)`, the next call with incremented attempt returns `Err(AttemptBeyondMax)`.

3. **OC-4.3 — Weak Fairness Liveness (TLA+ Domain)**
   - The weak-fairness liveness claim ("eventually exhaust") is a TLA+ temporal property and CANNOT be proven by Kani. This subclaim is satisfied by TLC. Document the split in the harness.

4. **OC-4.4 — Condensed Verification**
   - Target: `crates/vb_runtime/src/shard/helpers.rs:273-294::record_retry_attempt`
   - Harness file: `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs` (extend existing)
   - Evidence command: `cargo kani -p vb_runtime --harness kani_retry_*`

### CC-5: RETRY-JOURNAL-001 — Storage Key Injectivity and Idempotency

**Claim:** Runtime journal duplicate identity is keyed by Rust storage key (run, seq): strict appends reject duplicate keys and queued unpersisted appends allow exact idempotent duplicates only.

**Implementation Obligations:**

1. **OC-5.1 — Key Encoding Injectivity**
   - Write a Kani harness that verifies: for any two distinct `(RunId, EventSeq)` pairs, `run_event_key(run1, seq1) != run_event_key(run2, seq2)`.
   - This proves that the storage key uniquely identifies each journal event.

2. **OC-5.2 — Strict Duplicate Rejection**
   - Verify: `append_unpersisted(event)` returns `Err(DuplicateEvent{run, seq})` when a journal event with the same key already exists.

3. **OC-5.3 — Idempotent Duplicate Path**
   - Verify: `append_queued_unpersisted(event)` returns `Ok(())` when the existing event is bitwise-identical to `event`.
   - Verify: returns `Err(DuplicateEvent{run, seq})` when the existing event differs from `event`.

4. **OC-5.4 — Condensed Verification**
   - Target: `crates/vb_storage/src/journal/internal.rs:27-74::append_unpersisted/append_queued_unpersisted` AND `crates/vb_storage/src/keys.rs:41::run_event_key`
   - Harness file: `crates/vb_storage/src/verification/kani/kani_journal_duplicate.rs` (new)
   - Evidence command: `cargo kani -p vb_storage --harness kani_journal_duplicate_*`

### CC-6: RESUME-001 — RuntimeState Discipline and Rollback

**Claim:** Resume transitions preserve RuntimeState discipline: Resumed is journaled before drive success, journal append failure rolls back to Resumable, and post-Resumed drive failure rolls runtime state back while preserving the durable Resumed event.

**Implementation Obligations:**

1. **OC-6.1 — Resume State Guard**
   - Write a Kani harness that verifies: `handle_resume` returns `Err(NotResumable)` when `RuntimeState` is not `Resumable`.
   - Verify: `Resumable` → `append_resumed_event` is called; `Running` → `Ok(AlreadyRunning)`.

2. **OC-6.2 — Append-Before-Drive Ordering**
   - Verify: in `handle_resume`, `self.append_resumed_event(run)` is called BEFORE `self.drive_run(run)`. The journal append must succeed before the drive is attempted.

3. **OC-6.3 — Append Failure Rollback**
   - Verify: when `append_journal_event(resumed_event)` returns `Err(...)`, `self.apply(run, ResumeRollback)` is called and `handle_resume` returns `Err(JournalAppendFailed)`.
   - RuntimeState after rollback must be `Resumable`.

4. **OC-6.4 — Drive Failure Rollback (Preserving Journal)**
   - Verify: when `append_resumed_event` succeeds (journal has `Resumed`) but `drive_run` returns `Err(...)`, `restore_resumable_after_drive_failure` is called. RuntimeState rolls back to `Resumable`, but the `Resumed` journal event is preserved (not removed).

5. **OC-6.5 — Condensed Verification**
   - Target: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:291-367::handle_resume/append_resumed_event/restore_resumable_after_drive_failure` AND `crates/vb_runtime/src/shard/transitions.rs:36-60::apply`
   - Harness file: `crates/vb_runtime/src/verification/kani/kani_resume_state_machine.rs` (new)
   - Evidence command: `cargo kani -p vb_runtime --harness kani_resume_*`

### CC-7: ADMISSION-001 — Durable Header Before Live State

**Claim:** Admission never acknowledges or allocates live run state before durable header persistence; RunSubmitted/RunAdmission append failures map to AdmissionHeaderPersistenceFailed.

**Implementation Obligations:**

1. **OC-7.1 — Append-Before-Insert Ordering**
   - Write a Kani harness that verifies: in `handle_submit`, `append_admission_header_journal_event(RunSubmitted)` and `append_admission_header_journal_event(RunAdmission)` are called BEFORE `self.runs.insert(run, state)`.
   - If either append fails, `self.runs.insert()` is NEVER called.

2. **OC-7.2 — Append Failure Error Mapping**
   - Verify: when `append_journal_event` returns `Err(...)`, `append_admission_header_journal_event` calls `self.discard_journal_sequence(run)` and returns `Err(RuntimeError::AdmissionHeaderPersistenceFailed{source})`.

3. **OC-7.3 — Error Conversion Consistency**
   - Verify: `RuntimeError::admission_header_persistence_failed(StorageJournalAppend{source})` returns `AdmissionHeaderPersistenceFailed{source}`.
   - Verify: `admission_header_persistence_failed(AdmissionHeaderPersistenceFailed{source})` returns `AdmissionHeaderPersistenceFailed{source}` (idempotent wrapping).

4. **OC-7.4 — No Live State on Failure**
   - Verify: after `handle_submit` returns `Err(AdmissionHeaderPersistenceFailed{...})`, `self.runs` does NOT contain the run. No live state was allocated.

5. **OC-7.5 — Condensed Verification**
   - Target: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:150-200::handle_submit` AND `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:203-215::append_admission_header_journal_event` AND `crates/vb_runtime/src/error/conversions.rs:22-31::admission_header_persistence_failed`
   - Harness file: `crates/vb_runtime/src/verification/kani/kani_admission_ordering.rs` (new)
   - Evidence command: `cargo kani -p vb_runtime --harness kani_admission_*`

## Contract Acceptance Criteria

### Acceptance Gate 1: Harness Existence
- [ ] All 7 RRO rows have non-empty `refinement_harness_refs` or an approved waiver
- [ ] Each harness file exists and compiles with `#[kani::proof]` (or equivalent)
- [ ] Each harness targets production Rust symbols (not model copies)

### Acceptance Gate 2: Verification Pass
- [ ] All Kani harnesses pass: `cargo kani` exit 0 for each
- [ ] All Flux refinements pass: `cargo flux` exit 0
- [ ] All Verus specs pass: `verus` exit 0 (if applicable)
- [ ] All proptest properties pass: `cargo test` exit 0

### Acceptance Gate 3: Independent Review
- [ ] `proof-reviewer` confirms each harness binding with `reviewer_disposition: accepted`
- [ ] No harness is self-reviewed by the same agent that wrote it
- [ ] Bridge verdict transitions from REJECTED to PASS

### Acceptance Gate 4: Evidence Recording
- [ ] Raw command evidence exists for every harness (TLC, behavior test, harness)
- [ ] `verification-ledger.jsonl` has PASS rows for all 7 RRO rows
- [ ] `traceability-matrix.jsonl` maps every claim to every evidence artifact

### Acceptance Gate 5: No Regression
- [ ] All existing behavior tests pass (387+ tests across all RRO rows)
- [ ] All existing TLC runs pass (7 models, exit 0)
- [ ] No production code changes that weaken existing guards

## Contract Exclusions

This contract does NOT cover:
- Performance benchmarking of harnesses
- Fuzzing the journal key encoding or branch table parsing
- Loom models for concurrency hazards (recommended but not required for this bead)
- Miri checks for unsafe code (no unsafe code exists in scope)
- TLS/network security properties
- Admission resource budget arithmetic correctness (separate RRO)

## Precedence

In case of conflict between this contract and the TLA+ models, the Rust production code is the **ground truth**. The TLA+ models are specifications; if they contradict the Rust code, the Rust code is authoritatively correct until proven otherwise, at which point the Rust code must be fixed (GOD RULE 4).

The `velvet-ballistics-MASTER.md` and `AGENTS.md` govern overall workflow and linting. This contract governs the specific bead scope.
