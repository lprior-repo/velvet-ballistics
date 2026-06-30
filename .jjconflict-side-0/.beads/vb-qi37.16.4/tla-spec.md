# TLA+ Temporal Model Plan — vb-qi37.16.4

## Boundary
- **Temporal/workflow behavior owned by TLA+:**
  - Lifecycle state transitions for `AwaitingAsk` → `AskAnswered` → next step
  - Journal append discipline (no duplicate `AskAnswered` for same `(run_id, step, seq)`)
  - Monotonic sequence number invariant across journal events per run
  - Idempotent replay: already-answered ask tickets are skipped without error on replay
  - Answer persistence ordering: `SlotWritten` precedes `AskAnswered` in journal
- **Rust/core behavior excluded from TLA+ (Verus/Kani/tests):**
  - `AskTicket` field equality (pure Rust, checked by Verus)
  - `SlotValue` taint enforcement on write (Verus invariant)
  - Payload size bound check against `max_ipc_payload_bytes` (Verus + Kani)
  - File I/O for `value_file` reading (shell/integration test)
  - Fjall storage write ordering (storage integration test)
- **External systems abstracted:**
  - Fjall persistence treated as atomic journal append
  - CLI argument parsing treated as prevalidated inputs

## Non-applicability Rationale
Not applicable — this bead has clear temporal/state-over-time behavior (lifecycle state machine, journal replay determinism, duplicate prevention) that TLA+ is designed to specify and model-check.

---

## TLA+-Owned Clauses

### INV-001 → `NoDuplicateAskAnswered`
**Contract clause:** INV-001 (no two `AskAnswered` events with same `(run_id, step, seq)`)
**TLA+ module:** `AskAnswerLifecycle`
**Model path:** `specs/AskAnswerLifecycle.tla`

### INV-003 → `MonotonicSeqNo`
**Contract clause:** INV-003 (journal seqno monotonic per run)
**TLA+ module:** `AskAnswerLifecycle`

### INV-004 → `IdempotentReplay`
**Contract clause:** INV-004 (already-answered ticket skipped on replay)
**TLA+ module:** `AskAnswerLifecycle`

### POST-003 → `StateTransition`
**Contract clause:** POST-003 (run transitions from AwaitingAsk to next step after answer)
**TLA+ module:** `AskAnswerLifecycle`

### POST-001 + POST-002 ordering → `AnswerPersistenceOrder`
**Contract clause:** POST-001 (SlotWritten before AskAnswered)
**TLA+ module:** `AskAnswerLifecycle`

---

## Model Shape

**Module/model path:** `specs/AskAnswerLifecycle.tla`

**Variables:**
```
AskState \in [RunId → {"idle", "awaiting", "answered", "failed"}]
PendingAnswers \in SUBSET (RunId × StepIdx × SeqNo)
AnsweredLog \in SEQ (RunId × StepIdx × SeqNo × SlotValue × Taint)
SeqNoCounter \in [RunId → Nat]
```

**Init action:**
```
AskState = [r \in RunId |-> "idle"]
PendingAnswers = {}
AnsweredLog = <<>>
SeqNoCounter = [r \in RunId |-> 0]
```

**Actions:**
- `SubmitAsk(run, step, seq)` — run enters `awaiting`, ticket added to `PendingAnswers`
- `AnswerAsk(run, step, seq, value, taint)` — only enabled when state = `awaiting` and ticket ∈ `PendingAnswers`, emits `SlotWritten` then `AskAnswered` event, advances `SeqNoCounter[run]`, removes from `PendingAnswers`, sets `AskState[run] = "answered"`
- `ReplayAnswer(run, step, seq, value, taint)` — only enabled when `AskState[run] = "answered"` and ticket already in `AnsweredLog`; idempotently skips (no-op)
- `AdvanceToNextStep(run, step)` — transitions run to next step after answer applied

**State constraints:**
```
Len(AnsweredLog) <= MaxJournalEvents
SeqNoCounter[run] <= MaxSeqNo
```

**Symmetry sets:** None (runs are distinguishable by RunId).

**Bounded model limits for TLC:**
```
RunId \in 1..3
StepIdx \in 1..5
SeqNo \in 1..10
MaxJournalEvents \in 50
MaxSeqNo \in 1000
```

---

## Properties

### Safety Invariants
- `NoDuplicateAskAnswered`: No `(run, step, seq)` triple appears twice in `AnsweredLog`
- `ValidAskState`: `AskState[run]` is only ever one of `idle`, `awaiting`, `answered`, `failed`
- `PendingSubset`: `PendingAnswers ⊆ RunId × StepIdx × SeqNo`

### Liveness/Eventuality
- `EventuallyAnswered`: Every `run` that enters `awaiting` eventually either reaches `answered` or is marked `failed` (no infinite pending)
- `EventuallyAdvanced`: After `run` reaches `answered`, it eventually advances to the next step

### Fairness Assumptions
- Weak fairness on `AnswerAsk` action (if continuously enabled, eventually fires)
- Weak fairness on `AdvanceToNextStep` action

### Deadlock Freedom
- The model is deadlock-free under weak fairness; `AwaitingAsk` state is always resolvable by `AnswerAsk`

### Refinement to Rust/Runtime Behavior
- TLA+ `AskState["awaiting"]` refines Rust `EngineSignal::AwaitingAsk`
- TLA+ `AnsweredLog` entries refine `RuntimeJournalEvent::AskAnswered` records in Fjall
- TLA+ `PendingAnswers` refines in-memory `AskTicket` set in `Shard`
- TLA+ `SeqNoCounter` refines `SeqNo` monotonic counter per run in journal header
- TLA+ `AnswerAsk` action refinement: Rust must write `SlotWritten` before `AskAnswered` (POST-001 + POST-002 ordering)

---

## Evidence Command
```bash
tlc -config specs/AskAnswerLifecycle.cfg specs/AskAnswerLifecycle.tla
```

Expected: TLC reports no invariant violations, no deadlock, and temporal properties satisfied for `AskAnswerLifecycle.cfg` bounds.

---

## Waivers
None — all temporal clauses are covered by the TLA+ model above.

- PRE-001 (run in AwaitingAsk state) — covered by `AskState[run] = "awaiting"` enabling condition
- PRE-002 (step index matches) — covered by `PendingAnswers` tuple projection
- PRE-005 (no duplicate answer) — covered by `NoDuplicateAskAnswered` invariant
- PRE-004 (ticket match) — Verus-owned pure equality check
- INV-002 (taint enforcement) — Verus-owned Rust-local invariant
- PRE-003 (payload size) — Verus + Kani checked arithmetic
- POST-004 (durability) — storage integration test (manual-qa + journal replay test)
- POST-005 (secret redaction in diagnostics) — integration test + static analysis
