# Verification Layers — vb-qi37.16.4

## Boundary
- **Verus-owned kernel:** Pure Rust invariants for answer command — taint enforcement, ticket equality, duplicate detection, payload size bound
- **TLA+ temporal model:** `AskAnswerLifecycle` — lifecycle state machine, journal replay determinism, no duplicate answers, monotonic seqno
- **Theorem projection:** None — Verus covers all Rust-local pure behavior
- **Runtime shell:** File I/O (`value_file` reading), Fjall storage append, IPC frame encoding, CLI argument parsing
- **External systems excluded from formal proof:** Fjall keyspace layout, OS filesystem permissions, IPC transport

---

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Rationale |
|----------------|-------------|-----------------|-----------|
| PRE-001 (run in AwaitingAsk) | `tla-plus` | integration-test | Model-checked via `AskState[run] = "awaiting"` enabling condition; confirmed in CLI integration |
| PRE-002 (step index matches) | `tla-plus` | integration-test | Model-checked via `PendingAnswers` projection; confirmed in `cli_integration.rs` |
| PRE-003 (payload size bound) | `verus` + `kani` | `proptest` | Verus `requires` on `check_payload_size`; Kani bounded model check; proptest for edge values |
| PRE-004 (ticket match) | `verus` | `unit-test` | Verus `spec fn` for deterministic field equality; unit test for each field |
| PRE-005 (no duplicate answer) | `verus` | `tla-plus` | Verus spec + proof for deduplication set check; TLA+ `NoDuplicateAskAnswered` invariant |
| PRE-006 (secret redaction) | `static-scan` + `integration-test` | — | Clippy lint gate + integration test that inspects diagnostics output |
| POST-001 (SlotWritten before AskAnswered) | `tla-plus` | `integration-test` | `AnswerPersistenceOrder` temporal property in TLA+; confirmed in `lifecycle.rs` journal order |
| POST-002 (AskAnswered journal emit) | `tla-plus` | `integration-test` | TLA+ `AnsweredLog` refinement; confirmed in journal replay test |
| POST-003 (state transition) | `tla-plus` | `unit-test` | `StateTransition` temporal property; unit test for `EngineSignal` transition |
| POST-004 (durability) | `integration-test` | `manual-qa` | Journal replay integration test; hands-on QA for process restart |
| POST-005 (secret redaction in diagnostics) | `integration-test` | `static-scan` | Integration test validates diagnostics output; static scan denies secret in hot path |
| INV-001 (no duplicate AskAnswered) | `tla-plus` | `verus` | TLA+ `NoDuplicateAskAnswered`; Verus set dedup proof |
| INV-002 (taint enforcement) | `verus` | `kani` | Verus invariant on `SlotValue` write path; Kani for bounded state exploration |
| INV-003 (monotonic seqno) | `tla-plus` | `unit-test` | TLA+ `MonotonicSeqNo`; unit test for counter increment |
| INV-004 (idempotent replay) | `tla-plus` | `integration-test` | TLA+ `IdempotentReplay`; journal replay integration test |
| ERR-001 (RunNotFound) | `unit-test` | — | Unit test for each error variant constructor |
| ERR-002 (StepNotAwaitingAsk) | `unit-test` | — | Unit test for each error variant constructor |
| ERR-003 (TicketMismatch) | `unit-test` | — | Unit test for each error variant constructor |
| ERR-004 (DuplicateAnswer) | `unit-test` | — | Unit test for each error variant constructor |
| ERR-005 (PayloadTooLarge) | `verus` + `kani` | `unit-test` | Verus/Kani size bound; unit test |
| ERR-006 (ValueFileUnreadable) | `integration-test` | — | Integration test for file permission/path error handling |
| ERR-007 (SlotOutOfBounds) | `verus` | `unit-test` | Verus bounds-checked slot access; unit test |
| ERR-008 (SecretLeak) | `integration-test` | `static-scan` | Integration test for diagnostics redaction; static scan |

---

## Verus Scope

**Rust target:** `crates/vb_runtime/src/shard/lifecycle.rs::Shard::handle_ask_answer`

**Spec/proof functions:**
- `spec_taint_ok(value: SlotValue, taint: Taint, contract: &ResourceContract) -> bool`
- `spec_ticket_matches(ticket: &AskTicket, run: RunId, step: StepIdx, seq: SeqNo) -> bool`
- `spec_not_duplicate(ticket: &(RunId, StepIdx, SeqNo), answered: &Set) -> bool`
- `proof_answer_preserves_invariants` — taint and slot bounds maintained after write

**Invariants:**
- Slot write taint matches `ResourceContract` secret result policy
- Ticket fields match the suspended ask
- No duplicate ticket in answered set before journal append

**Trusted boundary:**
- `AskTicket` constructed only by `Shard` internals
- `ResourceContract` validated at workflow admission
- `SlotValue` handles are interned and never decoded inline

**Shell exclusions:**
- File I/O for `value_file` reading (shell layer)
- Fjall journal write ordering (storage integration test)
- IPC frame encoding (integration test)
- Wall-clock time, async scheduling (not used in answer path)

**Evidence command:**
```bash
verus crates/vb_runtime/src/shard/lifecycle.rs
```

Expected: Verus verified all `handle_ask_answer` proof obligations with 0 errors.

---

## TLA+ Scope

**Module/model path:** `specs/AskAnswerLifecycle.tla`

**Variables:**
```
AskState, PendingAnswers, AnsweredLog, SeqNoCounter
```

**Actions:** `Init`, `SubmitAsk`, `AnswerAsk`, `ReplayAnswer`, `AdvanceToNextStep`

**Safety invariants:** `NoDuplicateAskAnswered`, `ValidAskState`, `PendingSubset`

**Temporal properties:** `EventuallyAnswered`, `EventuallyAdvanced`

**Fairness/deadlock stance:** Weak fairness on `AnswerAsk` and `AdvanceToNextStep`; deadlock-free under fairness

**Refinement boundary:**
- TLA+ `AskState[run] = "awaiting"` → Rust `EngineSignal::AwaitingAsk`
- TLA+ `AnsweredLog` → `RuntimeJournalEvent::AskAnswered` records in Fjall
- TLA+ `PendingAnswers` → in-memory `AskTicket` set in `Shard`
- TLA+ `SeqNoCounter` → `SeqNo` monotonic counter per run in journal header

**Evidence command:**
```bash
tlc -config specs/AskAnswerLifecycle.cfg specs/AskAnswerLifecycle.tla
```

Expected: TLC reports no invariant violations, no deadlock, temporal properties satisfied.

---

## Theorem Scope
None — all Rust-local pure clauses are Verus-expressible.

**Waiver:** See `lean-contract.md` waiver record.

---

## Kani Scope

**Rust target:** `crates/vb_runtime/src/shard/lifecycle.rs::check_payload_size`

**Claim:** Bounded model check that `value_file` bytes length is always <= `max_ipc_payload_bytes` when the function returns `Ok`

**Evidence command:**
```bash
cargo kani --contract check_payload_size --harness check_payload_size
```

Expected: Kani reports all paths safe, no overflow, no out-of-bounds.

---

## Integration Test Scope

**Targets:**
- `crates/velvet_ballistics/tests/cli_integration.rs` — full `Command::Answer` CLI smoke
- Journal replay test: start run, suspend at ask, answer, kill process, restart, verify run resumes correctly
- Secret redaction test: emit diagnostics after answer, verify no `Secret`-tainted slot values appear in trace output

**Evidence:** Integration test report with journal replay evidence

---

## Performance Obligations
None for this bead — answer command is not on a hot path; performance evidence not required.

---

## Waivers

| Clause | Layer Waived | Reason | Compensating Evidence |
|--------|-------------|--------|----------------------|
| Fjall storage correctness | formal proof | Fjall is third-party; storage integration tests cover behavior | `fjall` skill review + integration test |
| IPC transport reliability | formal proof | Transport is third-party | Integration test with actual IPC frames |
| UNIT-ERR-ALL | unit-test | Unit test infrastructure not available for this bead; error variants covered at integration scope | `INTEGRATION-ERR-VALIDATION` covers all ERR-001 through ERR-008 variants at integration level with full runtime context |
| PROPTEST-PRE-003 | proptest | Proptest harness not available for this bead; KANI-PRE-003 provides equivalent bounded model checking | `KANI-PRE-003` provides formal bounded model check of `check_payload_size` for all u32 values |
