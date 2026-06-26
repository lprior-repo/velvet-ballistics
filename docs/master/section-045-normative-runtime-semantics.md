---
section: 45
title: "Normative Runtime Semantics"
parent: velvet-ballistics-MASTER.md
---

## 45. Normative Runtime Semantics


Every `CompiledNodeKind` variant has exact behavior defined here. The current IR interpreter must produce the specified terminal result, typed error variant and fields, final pc, slot values, slot taints, step states, journal event sequence, action tickets, retry counts, wait/ask scheduling, and replay behavior.

### StepState Transition Contract

Valid transitions:

```text
Pending    → Running, Succeeded, Failed, Cancelled, Skipped
Running    → Succeeded, Failed, Waiting, Asking, Cancelled, Skipped
Waiting    → Running
Asking     → Running
Succeeded  → (terminal)
Failed     → (terminal)
Cancelled  → (terminal)
Skipped    → (terminal)
```

Idempotent re-mark (`state == next`) is valid. All other transitions return `InternalInvariantViolation { reason: "invalid_state_transition" }`.

### EngineSignal / RuntimeSignal

Core engine signals: `Continue`, `Finished(SlotValue, Taint)`, `StepBudgetExhausted`, `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`.

Runtime engine extends `AwaitingAction` to carry `ActionTicket { run, step, seq, action, attempt, idempotency_key }`.

### Node Semantics

#### Nop

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Succeeded |
| Journal | None |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed |
| Errors | `MissingNextStep` if `node.next` is None |

#### SetConst { value: ConstIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Constant pool at `value` index |
| Slots written | `node.output` with `Taint::Clean` |
| Taint | Always Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, const pool bounds |
| Errors | `ConstOutOfBounds`, `MissingOutputSlot`, `MissingNextStep` |

#### Copy { source: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `source` slot value and taint |
| Slots written | `node.output` with source taint |
| Taint | Propagated from source |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed |
| Errors | `SlotOutOfBounds` (covers both out-of-bounds and uninitialized), `MissingOutputSlot`, `MissingNextStep` |

#### EvalExpr { expr: ExprIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Expression program at `expr` index; expression may read arbitrary slots, constants, and accessors |
| Slots written | `node.output` with the joined taint of expression operand slot reads |
| Taint | `join_taint` over loaded expression operand slot taints |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, expression stack depth ≤ 64, expression ops ≤ 256 |
| Errors | `ExprOutOfBounds`, `MissingOutputSlot`, `MissingNextStep`, any `ExprError` (stack overflow, type mismatch, division by zero, integer overflow) |

#### BuildObject { fields: Box<[(SymbolId, SlotIdx)]> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Each field's slot value and taint |
| Slots written | `node.output` with `SlotValue::Object(ObjectId)` and joined field taint |
| Taint | `join_taint` over field slot taints |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, object field count bounds |
| Errors | `SlotOutOfBounds` (any field slot), `MissingOutputSlot`, `MissingNextStep` |
| Side effects | Allocates `ObjectId` in `ValueStore` |

#### BuildList { items: Box<[SlotIdx]> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Each item's slot value and taint |
| Slots written | `node.output` with `SlotValue::List(ListId)` and joined item taint |
| Taint | `join_taint` over item slot taints |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed, list item count bounds |
| Errors | `SlotOutOfBounds` (any item slot), `MissingOutputSlot`, `MissingNextStep` |
| Side effects | Allocates `ListId` in `ValueStore` |

#### Jump { target: StepIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Succeeded |
| Journal | None |
| Suspension | Never |
| Next pc | `target` |
| Resource checks | Budget consumed |
| Errors | None at this node (invalid target caught at next step's `InvalidProgramCounter`) |

#### Finish { result: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `result` slot value |
| Slots written | None (result is returned as signal, not written) |
| Taint | Result taint passed through (no rejection of Secret/DerivedFromSecret) |
| StepState | Pending → Running → Succeeded |
| Journal | RunFinished |
| Suspension | Terminal — run completes |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `SlotOutOfBounds` (result slot uninitialized) |

#### Do { action: ActionId, input: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `input` slot taint (runtime engine also reads slot value for ticket) |
| Slots written | None at suspension time; output written on action completion |
| Taint | Runtime engine checks: `DeterministicPure` rejects non-Clean input. Output taint via `propagate_action_taint()`. `AtLeastOnceExternal` upgrades Secret to DerivedFromSecret. |
| StepState | Pending → Running (stays Running while awaiting action) |
| Journal | ActionScheduled at suspension; ActionCompleted on resume |
| Suspension | Returns `AwaitingAction(ActionTicket)` |
| Next pc | No change at suspension; set to `node.next` on action completion |
| Resource checks | Budget consumed, contract resolution, action ID validation |
| Errors | `UnknownAction` (if contracts non-empty and action not found), `TaintViolation` (DeterministicPure with tainted input) |
| Resume | Action completion writes output slot with value + taint, marks step Succeeded, advances pc |

#### WaitUntil { deadline_slot: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `deadline_slot` (must be I64 or F64) |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Waiting |
| Journal | WaitScheduled |
| Suspension | Returns `AwaitingWait` |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch { expected: "deadline" }` if slot is not numeric |
| Resume | Timer fire marks step Succeeded, sets pc to `node.next` |

#### WaitEvent { event: SlotIdx, timeout_slot: Option<SlotIdx> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `event` slot (numeric), optional `timeout_slot` (numeric) |
| Slots written | None |
| Taint | None |
| StepState | Pending → Running → Waiting |
| Journal | WaitScheduled |
| Suspension | Returns `AwaitingWait` |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch` if event or timeout is not numeric |
| Resume | Timer fire marks step Succeeded, sets pc to `node.next` |

#### Ask { prompt: SlotIdx, timeout_slot: Option<SlotIdx> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `prompt` slot (must be Symbol), optional `timeout_slot` (numeric) |
| Slots written | None at suspension; answer written on AskResume |
| Taint | None at suspension |
| StepState | Pending → Running → Asking |
| Journal | AskScheduled |
| Suspension | Returns `AwaitingAsk` |
| Next pc | No change |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch { expected: "prompt" }` if prompt is not Symbol |
| Resume | AskResume writes answer slot, marks step Succeeded, sets pc to `AskResume.next` |

#### AskResume { answer: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `answer` slot value and taint |
| Slots written | `node.output` with answer value and answer taint (if output is Some) |
| Taint | Propagated from answer slot |
| StepState | Running → Succeeded |
| Journal | SlotWritten, AskAnswered |
| Suspension | Never |
| Next pc | `node.next` |
| Resource checks | Budget consumed |
| Errors | `SlotOutOfBounds`, `MissingNextStep` |

#### Choose { branches: Box<[ExprBranch]>, otherwise: Option<StepIdx> }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Each branch's `ExprIdx` expression evaluated in order |
| Slots written | None (branches redirect control flow) |
| Taint | No taint enforcement on branch conditions |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten for any internal expression evaluations |
| Suspension | Never |
| Next pc | First branch with `Bool(true)` → branch target. None match → `otherwise`. |
| Resource checks | Budget consumed |
| Errors | `TypeMismatch` if expression result is not Bool, `MissingNextStep` if no match and no `otherwise` |

#### ChooseSlot { branches: Box<[SlotBranch]>, otherwise: Option<StepIdx> }

Same as Choose but reads pre-materialized boolean slots instead of evaluating expressions.

#### ForEachStart { input, item_slot, limit, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `input` slot (must be List) |
| Slots written | `item_slot` with first item; `output` with tail list. If empty: `output` with empty list. |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Resource checks | `item_count <= limit`, else `IterationLimitExceeded` |
| Errors | `TypeMismatch` if input is not List, `MissingOutputSlot`, iteration limit |

#### ForEachNext { iterator_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `iterator_slot` (must be List) |
| Slots written | `output` with first item; `iterator_slot` with tail list |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Errors | `TypeMismatch` if iterator is not List |

#### ForEachJoin { output: SlotIdx }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `materialized` slot (must be List) |
| Slots written | `output` with list value |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep`, `TypeMismatch` |

#### TogetherStart { branches, join }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | `output` with empty list |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | First branch entry |
| Resource checks | Branch count ≤ u16, branches non-empty |
| Errors | `TogetherBranchLimitExceeded`, `InvalidCompiledWorkflow` if branches empty, `MissingOutputSlot` |

#### TogetherBranch { branch, entry, join, accumulator }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `accumulator` slot (must be List); `output` for previous result |
| Slots written | `accumulator` with appended result |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `entry` (branch 0) or `entry` for subsequent branches |
| Errors | `TypeMismatch` if accumulator is not List |

#### TogetherJoin { branch_count, accumulator }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `accumulator` slot (List), `output` for last result |
| Slots written | `output` with final merged list |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### CollectStart { source, limit, page_size, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `source` slot (must be List) |
| Slots written | `collector_slot` with first page; pagination state in global table |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Resource checks | `item_count <= limit`, `page_size > 0`, `page_size <= limit` |
| Errors | `TypeMismatch`, `CollectItemLimitExceeded`, `CollectPageLimitExceeded`, `InvalidCompiledWorkflow` |

#### CollectPage { collector_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `collector_slot` validation (must be List) |
| Slots written | None |
| StepState | Running → Succeeded |
| Suspension | Never |
| Next pc | `body` |
| Errors | `TypeMismatch` if collector is not List |

#### CollectNext { collector_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `collector_slot` (List), pagination state |
| Slots written | `collector_slot` with next page |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Cursor exhausted → `done`. Otherwise → `body`. |
| Errors | `TypeMismatch`, `InvalidCompiledWorkflow` if pagination state missing |

#### CollectFinish { collector_slot }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `collector_slot` (List) |
| Slots written | `output` with collector value; pagination state removed |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### ReduceStart { input, accumulator, initial, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Constant pool at `initial`; `input` slot (must be List) |
| Slots written | `accumulator` with initial value; `output` with first item; `input` with tail list |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Errors | `ConstOutOfBounds`, `TypeMismatch`, `MissingOutputSlot` |

#### ReduceNext { iterator_slot, accumulator, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `iterator_slot` (must be List) |
| Slots written | `output` with next item; `iterator_slot` with tail |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Empty → `done`. Non-empty → `body`. |
| Errors | `TypeMismatch` |

#### ReduceFinish { accumulator }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `accumulator` slot |
| Slots written | `output` with accumulator value |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### RepeatStart { max_attempts, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | `output` with packed I64 `(max_attempts << 32) | 0` |
| Taint | Clean |
| StepState | Pending → Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `body` |
| Resource checks | `max_attempts > 0` |
| Errors | `MissingOutputSlot`, `InternalInvariantViolation` if max_attempts is 0 |

#### RepeatAttempt { attempt_slot, body, done }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `attempt_slot` (must be I64 packed state) |
| Slots written | None |
| StepState | Running → Succeeded |
| Suspension | Never |
| Next pc | `body` |
| Errors | `TypeMismatch`, `InternalInvariantViolation` if packed state invalid |

#### RepeatCheck { attempt_slot, body, exhausted }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `attempt_slot` (I64 packed state) |
| Slots written | `attempt_slot` with incremented attempt count |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | Attempts exhausted → `exhausted`. Otherwise → `body`. |
| Errors | `MissingNextStep` if attempts remain but next is None |

#### RepeatFinish { result }

| Aspect | Behavior |
|--------|----------|
| Inputs read | `result` slot |
| Slots written | `output` with result value |
| Taint | Clean |
| StepState | Running → Succeeded |
| Journal | SlotWritten |
| Suspension | Never |
| Next pc | `node.next` |
| Errors | `MissingOutputSlot`, `MissingNextStep` |

#### RetryCheck { policy_slot, body, exhausted }

| Aspect | Behavior |
|--------|----------|
| Inputs read | Retry policy (attempts remaining) |
| Slots written | None |
| StepState | Running → Succeeded |
| Suspension | Never |
| Next pc | Attempts remain → `body`. Exhausted → `exhausted`. |
| Errors | None |

#### ErrorHandler { body, handler }

| Aspect | Behavior |
|--------|----------|
| Inputs read | None |
| Slots written | None |
| StepState | Failed step stays Failed; handler step becomes Running → Succeeded |
| Journal | StepFailed on the failed step |
| Suspension | Never |
| Next pc | `handler` |
| Errors | None at this node |

---
