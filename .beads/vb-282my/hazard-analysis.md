# Hazard Analysis — vb-282my

**Bead:** vb-282my (P1)
**Title:** Refinement-specific hazards for TLA bridge harnesses
**Date:** 2026-05-29

## Hazard Classification

| Hazard Class | Description | Severity |
|-------------|------------|----------|
| **TEMPORAL** | Liveness/fairness properties that Kani cannot verify. Gaps between TLA+ liveness and Rust safety. | HIGH |
| **RUST-CORE-INVARIANT** | State-machine invariants, type safety, overflow, panic-freedom in production Rust. | HIGH |
| **BOUNDED-STATE** | Model-checking bounds that are too small to reveal real bugs. State-space explosion masking counterexamples. | HIGH |
| **REFINEMENT** | Gap between TLA+ abstraction and Rust implementation. The model says X, the code does Y. | **CRITICAL** |
| **CONCURRENCY** | Race conditions in shard lifecycle not modeled by TLA+ or Kani. | HIGH |
| **HOSTILE-INPUT** | Malformed journal events, corrupt storage keys, oversized payloads. | MEDIUM |
| **PERFORMANCE** | Harness overhead masking time-of-check-to-time-of-use (TOCTOU) issues. | LOW |
| **RELEASE/API** | Harness compiled for test target but not for production; production code diverges. | MEDIUM |
| **UNSOUND-ASSUME** | `kani::assume` / `#[verus::trusted]` / `#[flux_rs::trusted]` that excludes reachable states. | **CRITICAL** |

---

## 1. Temporal Hazards

### HZ-TEMP-001: Weak Fairness Liveness Not Verifiable by Kani
- **Affected RROs:** RETRY-FSM-001
- **TLA+ Claim:** "Retryable failures eventually exhaust under weak fairness."
- **Hazard:** Kani verifies bounded safety properties, not liveness. The weak-fairness assumption ("every retryable action that is continuously enabled will eventually be taken") requires unbounded temporal reasoning. Kani cannot prove or disprove this.
- **Severity:** HIGH (but acceptable — liveness is TLA+'s responsibility)
- **Mitigation:** Kani verifies the safety subset: (a) no retry after max_attempts, (b) exhausted retries are typed and terminal. TLA+ handles the liveness part. This split must be documented in the RRO bridge.
- **Residual Risk:** If the scheduler starves a retryable action indefinitely, Kani won't catch it. Behavior tests with loom may detect starvation patterns.

### HZ-TEMP-002: TLC Bounds Mask Counterexamples
- **Affected RROs:** All 7
- **TLA+ Claim:** Various invariants checked under bounded TLC configs.
- **Hazard:** TLC bounds are small (e.g., RetryJournal: MaxSeq=4, MaxJournalEvents=4). A counterexample requiring 5 events would not be found. The model passes at bound N but fails at bound N+1.
- **Severity:** HIGH
- **Mitigation:** Each TLC config must be justified: "bound N is sufficient to exercise all state-machine transitions." The bounds must be adequate for the claim, not just convenient. For RetryJournal, MaxSeq=4 covers single-event, duplicate, idempotent-duplicate, and different-event-duplicate — all 4 interesting cases.
- **Residual Risk:** Bounds are acceptable per current review but should be periodically re-validated as production code evolves.

### HZ-TEMP-003: Adversarial Scheduling Between Journal Append and State Insert
- **Affected RROs:** ADMISSION-001, ASK-ANSWER-001, RESUME-001
- **Hazard:** In production, an async task could be scheduled between the journal append and the state insert. The append succeeds, but before the state is updated, another task reads stale state. Kani (single-threaded) cannot detect this.
- **Severity:** HIGH
- **Mitigation:** The TLA+ models for these RROs specify append-before-insert as atomic transitions at the TLA+ level. In Rust, the journal append and state insert are called sequentially within the same synchronous function body (`handle_submit`, `handle_resume`, `await_timer`). The Rust code does not await/yield between append and insert. Loom or shuttle could verify this.
- **Residual Risk:** If a future refactor adds an `.await` point between append and insert, the invariant breaks. A Loom model or shuttle test should guard this.

---

## 2. Rust-Core-Invariant Hazards

### HZ-RUST-001: Overflow in Branch Index Increment
- **Affected RROs:** CHOOSE-REPLAY-001
- **Rust Source:** `replay_choose_slot()` at `crates/vb_core/src/replay/choose.rs:45`
- **Hazard:** `index = index.checked_add(1).ok_or(ReplayError::Internal{...})?`. The checked_add is correct, but if the branch count is exactly `usize::MAX`, iteration would produce an Internal error rather than a valid result. In practice, branch counts are bounded by the compile-time fanout limit (≤ 64), so this is unreachable.
- **Severity:** LOW (guarded by compile-time limit)
- **Mitigation:** The compile-time `lower_canonical_choose` in `vb_compile` enforces `branches.len() <= 64`. Runtime `replay_choose_slot` is always called with lowered branches from that compile step. Kani harness should verify that index overflow cannot occur when branch count ≤ 64.
- **Residual Risk:** If the compile-time fanout limit is bypassed (e.g., hand-crafted `SlotBranch` arrays), runtime overflow could occur. Kani covers this for bounded branch counts.

### HZ-RUST-002: Recorded Attempt Counter Overflow
- **Affected RROs:** RETRY-FSM-001
- **Rust Source:** `record_retry_attempt()` at `crates/vb_runtime/src/shard/helpers.rs:288-292`
- **Hazard:** `attempt.checked_add(1).ok_or(RuntimeError::UnsupportedOperation{...})?`. If `max_attempts` is `u16::MAX` and the retry loop runs long enough, `attempt` could reach `u16::MAX` and overflow the `checked_add`. This is correctly fail-closed but produces a confusing error (`UnsupportedOperation` rather than a semantic retry error).
- **Severity:** MEDIUM
- **Mitigation:** The `validate_retry_attempt` guard checks `attempt >= max_attempts` before incrementing. If `max_attempts < u16::MAX`, `attempt` will hit the guard before overflowing. Only when `max_attempts == u16::MAX` could overflow occur, and then only after 65,535 retries. Existing Kani harness partly covers this (`kani_retry_attempt_overflow_fail_closed`).
- **Residual Risk:** The overflow path is already covered by the existing Kani harness. This hazard is well-mitigated.

### HZ-RUST-003: RuntimeState Inconsistency on Concurrent Resume + Submit
- **Affected RROs:** RESUME-001
- **Hazard:** Two concurrent calls: one calls `handle_resume(run)`, another calls `handle_submit(run)` (re-submit of same run). The `handle_submit` sees `RuntimeState::Running` in `get_runtime_state_or_running` (default value when key not found), then inserts a new `RunState`. The `handle_resume` races to read `runtime_states` before or after the insert.
- **Severity:** HIGH (but not in single-shard scope — shard operations are serialized by design)
- **Mitigation:** Shard lifecycle operations are called within the shard's event loop, which is single-threaded per shard. Cross-shard interactions are mediated by the runtime scheduler. The TLA+ models assume atomic state transitions.
- **Residual Risk:** If concurrency within a single shard is introduced, this hazard becomes critical. A Loom model should document the current serialization guarantee.

### HZ-RUST-004: Slot Uninitialized Reading in ChooseSlot Replay
- **Affected RROs:** CHOOSE-REPLAY-001
- **Rust Source:** `replay_choose_slot()` at `crates/vb_core/src/replay/choose.rs:22-28`
- **Hazard:** `run.read_slot(branch.condition)` returns `EngineError::SlotUninitialized` if the slot was never written. The replay maps this to `ReplayError::SlotNotAvailable`. If the compile-time lowering produces a branch whose condition slot is never written at runtime, the choose will fail. The TLA+ model assumes all condition slots are initialized.
- **Severity:** MEDIUM (compile-time lowering guarantees slot initialization before choose step)
- **Mitigation:** The compiler ensures that slot-writing steps precede choose steps in the lowered DAG. The TLA+ model validates this ordering under `SlotWritten` tracking.
- **Residual Risk:** Hand-crafted or corrupted `CompiledWorkflow` could have uninitialized condition slots. Kani harness should verify error mapping: uninitialized slot → `SlotNotAvailable`.

---

## 3. Refinement Hazards (CRITICAL)

### HZ-REF-001: TLA+ Model Abstraction Gap — AskAnswer Timers
- **Affected RROs:** ASK-ANSWER-001
- **Hazard:** The TLA+ model uses `pendingTimerStep[run]` and `pendingTimerKind[run]` as functions from RunId to timer state. The Rust code uses `self.pending_timers: HashMap<RunId, PendingTimer>` with generation counters. The TLA+ model does not model the generation counter, which detects stale timers in Rust. If the TLA+ model assumes a pending timer always implies `AskScheduled` was journaled, but the Rust code can have a stale pending timer (wrong generation), the refinement is incomplete.
- **Severity:** **CRITICAL**
- **Mitigation:** The TLA+ model's invariant `AskTimerImpliesAskScheduled` is unconditional: if `pendingTimerKind[run] = "Ask"`, then a journal event `AskScheduled` MUST exist. In Rust, this is enforced by: (a) `await_timer` appends `AskScheduled` BEFORE inserting into `pending_timers`, and (b) if append fails, `pending_timers` is NOT inserted. The generation counter is a defense-in-depth against stale timers, but the core refinement (timer → AskScheduled journal) holds without it. Kani should verify: if append fails, `pending_timers` is unchanged.
- **Residual Risk:** Future code changes could violate the append-before-insert ordering. Kani harness directly verifies the ordering.

### HZ-REF-002: TLA+ Model Abstraction Gap — Journal Event Payload Equality
- **Affected RROs:** RETRY-JOURNAL-001
- **Hazard:** The TLA+ model uses `(run, seq)` identity for `JournalRecord`. The Rust `append_queued_unpersisted` re-reads the existing journal event and compares byte-for-byte `existing == *event` as a Rust `PartialEq` comparison. If two `JournalEvent` values are semantically equal but have different internal representations (e.g., different enum variant tag due to encoding), the `==` comparison fails and the idempotent path incorrectly returns `DuplicateEvent`.
- **Severity:** HIGH
- **Mitigation:** `JournalEvent` derives `PartialEq` and the serialization round-trips through `postcard`. If encoding is deterministic, two equal events produce identical bytes. The Rust behavior tests verify this: duplicate journal tests pass (90 tests in `journal/tests.rs`).
- **Residual Risk:** A change to `JournalEvent`'s `PartialEq` implementation or to the postcard encoding could break idempotency. Kani should verify: `event == event` always true for `PartialEq` consistency.

### HZ-REF-003: TLA+ Resume Model Does Not Model `AlreadyRunning` Path
- **Affected RROs:** RESUME-001
- **Hazard:** The TLA+ ResumeStateMachine models `ResumeAlreadyRunning` as a no-op that returns `AlreadyRunning`. The Rust `handle_resume` checks `current_state == RuntimeState::Running` and returns `ResumeResult { status: ResumeStatus::AlreadyRunning }`. If the TLA+ model does not prove that `AlreadyRunning` cannot be followed by a mutation that contradicts the model, the refinement is incomplete.
- **Severity:** MEDIUM
- **Mitigation:** The `AlreadyRunning` path in Rust is a pure read-and-return — no state mutation. The TLA+ model's `ResumeAlreadyRunning(r)` leaves all variables unchanged. This is consistent.
- **Residual Risk:** If `AlreadyRunning` were changed to mutate state (e.g., increment a counter), the TLA+ model would need updating.

### HZ-REF-004: Admission Model Bounds Hide Race with Admission Check
- **Affected RROs:** ADMISSION-001
- **Hazard:** The admission model uses `ErrorCodes={HeaderPersistenceFailed, QueueFull}` — only two error codes. The real Rust `build_admission` has 13 error variants (CapabilityDenied, ResourceCapacityExceeded, BudgetPolicyExceeded, etc.). The TLA+ model does not cover these. If an admission error occurs BEFORE the header append (e.g., `AdmissionCapabilityDenied`), the Rust code returns an error without attempting any journal append. The TLA+ model's `AdmissionReject` covers this as a generic reject, but doesn't distinguish error kinds.
- **Severity:** MEDIUM
- **Mitigation:** The TLA+ model's claim is: "Admission never acknowledges or allocates live run state before durable header persistence." Pre-append admission errors (capability, budget) return before any journal append — they satisfy the claim trivially (no state allocated, no ack). The TLA+ model's `AdmissionReject` transitions cover this case. Post-append errors are the ones that matter for the "before durable persistence" claim.
- **Residual Risk:** If a pre-append error incorrectly allocates state before returning the error, the claim is violated. This would be a Rust logic bug, not a TLA+ modeling gap.

### HZ-REF-005: ChooseSlotLowering Model Uses Different Branch Semantics Than Lowering
- **Affected RROs:** CHOOSE-LOWERING-001
- **Hazard:** The TLA+ model `CanonicalLoweredBranches` maps every branch `condition[i]` to `nextTarget` (the fallthrough step). The Rust code does the same: `SlotBranch { condition, target }`. But the TLA+ model only models the lowering output, not the lowering algorithm itself. If the Rust algorithm produces branches that differ from the model's expected output, the refinement is invalid.
- **Severity:** HIGH
- **Mitigation:** The TLA+ model's lowering step `LowerSlot` produces `SlotLoweredBranches` for slot-input and `CanonicalLoweredBranches` for canonical input. The model asserts that for canonical input, all branch targets equal `nextTarget` and all branch conditions are recorded. Kani can verify: given canonical input (label-based), `lower_canonical_choose` produces branches where every `target == next` and every `condition` is a valid slot.
- **Residual Risk:** Kani verification must compare lowered output against the TLA+ model's expected shape. A mismatch indicates a bug in either the model or the code.

---

## 4. Concurrency Hazards

### HZ-CONC-001: Journal Append and Pending Timer Insertion Race
- **Affected RROs:** ASK-ANSWER-001
- **Hazard:** `await_timer` appends `AskScheduled` to the journal, then inserts into `pending_timers`. Between the append and insert, a concurrent `handle_ask_answer` could check `pending_timers` and not find the timer — not a correctness issue since the timer hasn't been registered yet. But if the order were reversed (insert then append), and append fails, the timer would be in `pending_timers` without the `AskScheduled` journal event — violating the TLA+ model.
- **Severity:** HIGH (currently mitigated by correct ordering)
- **Mitigation:** The current Rust code appends BEFORE inserting — correct ordering. The append-failure path does NOT insert. Kani harness must verify this ordering under both success and failure paths.
- **Residual Risk:** None in current code. A future refactor reversing the order would break the invariant; Kani harness would catch it.

### HZ-CONC-002: Admission Run Submission Duplicate Prevention
- **Affected RROs:** ADMISSION-001
- **Hazard:** Two concurrent `handle_submit` calls for the same `RunId`. The first call appends `RunSubmitted` to the journal and inserts `RunState`. The second call's append could race with the first's insert. If the second call checks `runs.contains_key(&run)` BEFORE the first inserts, both could proceed.
- **Severity:** HIGH (but same-shard operations are serialized)
- **Mitigation:** Shard operations are called within the shard's event loop (single-threaded). Cross-shard duplicate runs are prevented by admission checking. The TLA+ model's `duplicate_run` flag covers this at the model level.
- **Residual Risk:** Loom model to document single-shard serialization guarantee.

---

## 5. Hostile Input Hazards

### HZ-INPUT-001: Malformed Journal Event Payload During Idempotency Check
- **Affected RROs:** RETRY-JOURNAL-001
- **Hazard:** `append_queued_unpersisted` calls `decode_record::<JournalEvent>` on the existing value. If the stored value is corrupt (wrong magic, truncated), `decode_record` returns an error. The current code returns `Err(JournalError::DuplicateEvent { run, seq })` if the get succeeds but decode fails — which is semantically wrong (the event is corrupt, not a duplicate).
- **Severity:** MEDIUM
- **Mitigation:** The error mapping could be more precise: distinguish "decode failure" from "truly duplicate". Currently both return `DuplicateEvent`, which is conservative (fail-closed) but confusing. Not a correctness hazard for the TLA+ refinement.
- **Residual Risk:** If a corrupted journal has a valid key but invalid value, `append_queued_unpersisted` returns `DuplicateEvent` instead of a decode error. This could mask storage corruption.

### HZ-INPUT-002: Oversized Branch Table in Hand-Crafted Workflow
- **Affected RROs:** CHOOSE-LOWERING-001, CHOOSE-REPLAY-001
- **Hazard:** If a hand-crafted `CompiledWorkflow` has a choose node with > 64 branches (bypassing the compile-time check), `replay_choose_slot` would iterate over all of them. While `checked_add` prevents overflow, iteration over a large branch table could be slow (DoS).
- **Severity:** LOW (DoS only, not UB)
- **Mitigation:** The compile-time fanout limit of 64 is enforced by `lower_canonical_choose`. Hand-crafted workflows bypassing compilation would need to explicitly construct slot branches.
- **Residual Risk:** Acceptable. Kani can verify that for branch counts ≤ an upper bound (e.g., 64), the function completes without panic.

---

## 6. Unbound-Assumption Hazards

### HZ-UNSOUND-001: Kani `assume` Masking the Exhaustion Path
- **Affected RROs:** RETRY-FSM-001
- **Hazard:** If a Kani harness uses `kani::assume(attempt < max_attempts)` to avoid the exhaustion path, the proof is vacuous — it only covers the retry-allowed case, not the no-retry-after-max case. The full claim requires both paths. The existing harness `kani_retry_attempt_monotonicity` tests `attempt == max_attempts` explicitly at line 347, which is good, but does it also test the post-exhaustion path (attempts > max_attempts)?
- **Severity:** **CRITICAL**
- **Mitigation:** The harness already tests `attempt == policy.max_attempts` and asserts `result.is_err()`. The post-exhaustion path is covered because the monotonicity property `attempt >= max_attempts ⇒ error` holds at the boundary. The missing sub-claim is the terminal typing: after exhaustion, the error is `RetryExhausted` (or mapped to `Failed` state). The Kani harness must verify: after record_retry_attempt returns `Ok(false)` (no more retries), the next call to `record_retry_attempt` with an incremented attempt returns `Err` with the appropriate error kind.
- **Residual Risk:** The current harness does not verify terminal typing. This gap must be closed in the extended harness.

### HZ-UNSOUND-002: Stubbing Journal Append as Always-Success
- **Affected RROs:** ASK-ANSWER-001, ADMISSION-001, RESUME-001
- **Hazard:** If Kani harnesses stub `append_journal_event` to always return `Ok(())`, the append-failure paths are never exercised. The TLA+ model explicitly covers append-failure paths (e.g., `StorageFail` in admission model, `JournalAppendFailed` in resume model).
- **Severity:** **CRITICAL**
- **Mitigation:** Kani harnesses for ASK-ANSWER, ADMISSION, and RESUME MUST stub `append_journal_event` to return both `Ok(())` and `Err(JournalError::...)` nondeterministically. Both paths must be verified. The append-failure behavior test `runtime_ask_timer_append_failure_does_not_register_pending_timer` verifies the failure path for timers — Kani must do the same.
- **Residual Risk:** Harnesses that assume `append_journal_event` always succeeds provide no evidence for failure-path behavior, which is the primary claim of several RROs.

---

## 7. Performance / API Hazards

### HZ-PERF-001: Large Branch Table Performance
- **Affected RROs:** CHOOSE-REPLAY-001
- **Hazard:** `replay_choose_slot` performs `read_slot(branch.condition)` for each branch. Each `read_slot` is O(1) (array lookup), but with 64 branches, the function scans up to 64 slots. Acceptable for replay (not a hot path).
- **Severity:** LOW
- **Mitigation:** Performance is bounded by compile-time fanout limit of 64.

### HZ-API-001: Harness Only Compiled Under `cfg(kani)`
- **Affected RROs:** All 7
- **Hazard:** Kani harnesses are `#[cfg(kani)]` and are NOT compiled in production. If the production code path diverges from the harness code path (e.g., different feature flags enable different code), the verification may prove a code path that never runs in production.
- **Severity:** MEDIUM
- **Mitigation:** Harnesses must call the exact same production functions that run at runtime. No `#[cfg(kani)]`-only code paths in production functions. If a function has `#[cfg(not(kani))]` and `#[cfg(kani)]` versions, the harness MUST verify BOTH or the bridge is invalid.
- **Residual Risk:** Audit production functions for `cfg(kani)` divergence. Currently, no production functions in scope have Kani-specific paths.

---

## Hazard Severity Summary

| Hazard ID | Severity | Affected RROs | Mitigation Status |
|-----------|----------|---------------|-------------------|
| HZ-TEMP-001 | HIGH | RETRY-FSM-001 | Acceptable: liveness stays in TLA+ |
| HZ-TEMP-002 | HIGH | All | TLC bounds justified per model |
| HZ-TEMP-003 | HIGH | ADMISSION, ASK, RESUME | Shard serialization; Loom recommended |
| HZ-RUST-001 | LOW | CHOOSE-REPLAY-001 | Compile-time limit guards this |
| HZ-RUST-002 | MEDIUM | RETRY-FSM-001 | Existing Kani harness covers |
| HZ-RUST-003 | HIGH | RESUME-001 | Shard serialization |
| HZ-RUST-004 | MEDIUM | CHOOSE-REPLAY-001 | Kani to verify error mapping |
| HZ-REF-001 | **CRITICAL** | ASK-ANSWER-001 | Kani must verify append-before-insert ordering |
| HZ-REF-002 | HIGH | RETRY-JOURNAL-001 | Kani to verify PartialEq consistency |
| HZ-REF-003 | MEDIUM | RESUME-001 | Model and code are consistent |
| HZ-REF-004 | MEDIUM | ADMISSION-001 | Trivial satisfaction for pre-append errors |
| HZ-REF-005 | HIGH | CHOOSE-LOWERING-001 | Kani to verify lowered output shape |
| HZ-CONC-001 | HIGH | ASK-ANSWER-001 | Correct ordering; Kani to verify |
| HZ-CONC-002 | HIGH | ADMISSION-001 | Shard serialization |
| HZ-INPUT-001 | MEDIUM | RETRY-JOURNAL-001 | Fail-closed; not a refinement hazard |
| HZ-INPUT-002 | LOW | CHOOSE-* | DoS only; bounded by fanout limit |
| HZ-UNSOUND-001 | **CRITICAL** | RETRY-FSM-001 | Harness must cover full claim |
| HZ-UNSOUND-002 | **CRITICAL** | ASK, ADMISSION, RESUME | Harness must exercise both append paths |
| HZ-PERF-001 | LOW | CHOOSE-REPLAY-001 | Acceptable |
| HZ-API-001 | MEDIUM | All | Audit for cfg(kani) divergence |
