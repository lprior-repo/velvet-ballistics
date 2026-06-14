# Fjall Storage Journal

Fjall is the required embedded durability substrate for the current Backend / IR Interpreter Complete milestone. It stores workflow source, compiled IR, run headers, journal events, snapshots, blobs, and indexes; it is not the in-memory execution state.

## Key Layout

Journal keys are fixed-width big-endian bytes:

```text
events: [RunId_16B | EventSeq_8B] = 24 bytes
```

Big-endian encoding preserves numeric ordering during prefix/range scans.

## Event Encoding

Internal events use compact binary encoding through `postcard`. JSONL is a public observability projection and must not be the primary durable journal format.

The storage API exposes explicit durability names: `append_journaled` writes without a caller-visible fsync barrier, while `append_strict` appends and calls `PersistMode::SyncAll` before returning.

Duplicate `(RunId, EventSeq)` appends are rejected. Event history is immutable; insert overwrite behavior from the underlying key-value store is not exposed as a journal operation.

## Durability Modes

| Mode | Meaning | Crash Behavior |
| --- | --- | --- |
| Volatile | no Fjall append | run is lost on process crash |
| Journaled | bounded group commit via `JournalWriterQueue` | acknowledged data-loss window until persistence barrier |
| Strict | synchronous `PersistMode::SyncAll` after critical writes | strongest local durability, highest latency |

Default policy target:

- `RunAccepted` is durable before acknowledgement.
- `StepStarted` for side-effecting actions is durable before the external effect.
- `StepSucceeded` for side-effecting actions is durable before downstream side effects.
- Pure `save`/`choose` chains may group-commit when replay semantics remain valid.

## Recovery

Current-scope recovery loads accepted artifacts by digest and never reparses YAML for existing runs. Full recovery replays the journal when no snapshot exists. Snapshot recovery hydrates from the latest snapshot and replays the tail journal.

Recovery must reconstruct slot values, slot taint, step lifecycle, pending action state where supported, and terminal outcomes from durable records. Unsupported live recovery states must fail closed with typed errors instead of hydrating a broken `RunFrame`.

The master drift register still treats pending-action hydration and strict acknowledgement behavior as high-risk evidence areas. Do not claim crash safety without end-to-end recovery evidence.

## Primitive Durability Proof Matrix

This table is the current `CompiledNodeKind` durability ledger required by `velvet-ballistics-MASTER.md` Section 40. The freshness gate is `python3 scripts/check-primitive-durability-doc.py`; it compares this table with `crates/vb_core/src/workflow/types.rs` so new primitives cannot land without a completion-proof row.

`VerificationProof.durable=true` means the accepted artifact and the listed per-primitive proof events have crossed the strict persistence barrier before an acknowledgement that depends on them. Volatile execution and journaled execution without the strict barrier must not be cited as durable-completion proof.

<!-- BEGIN PRIMITIVE DURABILITY PROOF MATRIX -->
| Primitive | Completion journal events | Recovery proof | `VerificationProof.durable` gate |
| --- | --- | --- | --- |
| `Nop` | `StepStarted` then `StepSucceeded`. | Replay marks the step succeeded and advances only through the recorded next step. | Strict proof requires both lifecycle events durable before acknowledgement. |
| `SetConst` | `StepStarted`, `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the constant slot value/taint from `SlotWrittenEvent` before accepting success. | Strict proof requires slot write and success event durable before downstream effects. |
| `Copy` | `StepStarted`, destination `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the copied slot and taint from the durable slot-write envelope. | Strict proof requires copied value evidence durable before success acknowledgement. |
| `EvalExpr` | `StepStarted`, expression-result `SlotWrittenEvent`, then `StepSucceeded`. | Replay uses the encoded slot value/taint rather than re-evaluating expression side effects. | Strict proof requires result slot evidence durable before success acknowledgement. |
| `BuildObject` | `StepStarted`, object-handle `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the object handle and taint from durable slot evidence. | Strict proof requires object slot evidence durable before success acknowledgement. |
| `BuildList` | `StepStarted`, list-handle `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the list handle and taint from durable slot evidence. | Strict proof requires list slot evidence durable before success acknowledgement. |
| `Do` | `StepStarted`, `ActionScheduledTicket` or `ActionScheduled` before dispatch; after result, `ActionCompletedEnvelope` or `ActionCompletedEvent`, output `SlotWrittenEvent`, then `StepSucceeded`. | Replay treats scheduled-without-completion as pending and blocks duplicate non-idempotent redispatch; completed envelopes hydrate the output slot. | Strict proof requires schedule evidence durable before external dispatch and completion/output durable before downstream effects. |
| `Choose` | `StepStarted` then `StepSucceeded` for the selected branch decision. | Replay trusts the recorded step completion and validated IR targets, not a new graph search. | Strict proof requires branch completion event durable before advancing dependent steps. |
| `ChooseSlot` | `StepStarted` then `StepSucceeded` for the selected slot-branch decision. | Replay trusts the durable branch completion and prevalidated numeric target. | Strict proof requires branch completion event durable before advancing dependent steps. |
| `ForEachStart` | `StepStarted`, iterator-state `SlotWrittenEvent`, then `StepSucceeded`. | Replay hydrates iterator state from durable slot evidence before any body resume. | Strict proof requires iterator state durable before body scheduling. |
| `ForEachNext` | `StepStarted`, item/update `SlotWrittenEvent`, then `StepSucceeded` or completion to `done`. | Replay resumes with the durable item/update state or recognizes the recorded done transition. | Strict proof requires item/update evidence durable before next body dispatch. |
| `ForEachJoin` | `StepStarted`, joined-output `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs ordered loop output from durable joined-output evidence. | Strict proof requires join output durable before downstream effects. |
| `TogetherStart` | `StepStarted`, branch-schedule/join-state `SlotWrittenEvent`, then `StepSucceeded`. | Replay uses durable branch/join state to avoid reordering or duplicating branches. | Strict proof requires branch/join state durable before branch execution is acknowledged. |
| `TogetherBranch` | `StepStarted`, branch accumulator `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs branch contribution from durable accumulator evidence. | Strict proof requires branch contribution durable before join acknowledgement. |
| `TogetherJoin` | `StepStarted`, joined accumulator `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs joined branch output from durable accumulator evidence. | Strict proof requires join accumulator durable before downstream effects. |
| `CollectStart` | `StepStarted`, collector-state `SlotWrittenEvent`, then `StepSucceeded`. | Replay hydrates collector pagination state from durable slot evidence. | Strict proof requires collector state durable before page work is acknowledged. |
| `CollectPage` | `StepStarted`, page-result `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the processed page and pagination cursor from durable evidence. | Strict proof requires page result durable before requesting the next page. |
| `CollectNext` | `StepStarted`, collector cursor `SlotWrittenEvent`, then `StepSucceeded` or completion to `done`. | Replay resumes from the durable cursor or recognizes the recorded done transition. | Strict proof requires cursor evidence durable before the next page action. |
| `CollectFinish` | `StepStarted`, materialized collection `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the final materialized collection from durable slot evidence. | Strict proof requires collection output durable before downstream effects. |
| `ReduceStart` | `StepStarted`, accumulator `SlotWrittenEvent`, then `StepSucceeded`. | Replay hydrates the initial accumulator from durable slot evidence. | Strict proof requires accumulator evidence durable before body execution. |
| `ReduceNext` | `StepStarted`, updated accumulator `SlotWrittenEvent`, then `StepSucceeded` or completion to `done`. | Replay resumes from durable accumulator evidence and avoids double-applying reducer work. | Strict proof requires updated accumulator durable before next reducer step. |
| `ReduceFinish` | `StepStarted`, final accumulator `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the final reduced value from durable slot evidence. | Strict proof requires final accumulator durable before downstream effects. |
| `RepeatStart` | `StepStarted`, repeat-attempt state `SlotWrittenEvent`, then `StepSucceeded`. | Replay hydrates attempt state from durable evidence before retry/body scheduling. | Strict proof requires attempt state durable before body execution. |
| `RepeatAttempt` | `StepStarted`, attempt-result/update `SlotWrittenEvent`, then `StepSucceeded`. | Replay resumes from durable attempt state and avoids double-counting an attempt. | Strict proof requires attempt update durable before repeat check. |
| `RepeatCheck` | `StepStarted`, repeat-state `SlotWrittenEvent`, then `StepSucceeded` or transition to `done`. | Replay uses durable attempt/check state to preserve max-attempt bounds. | Strict proof requires check state durable before scheduling another attempt. |
| `RepeatFinish` | `StepStarted`, repeat result `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the repeat result from durable slot evidence. | Strict proof requires repeat result durable before downstream effects. |
| `WaitUntil` | `StepStarted`, `WaitScheduledEvent`; after timer/resume, `StepSucceeded`. | Replay keeps the run waiting when only schedule evidence exists, or resumes only after durable resume/success evidence. | Strict proof requires schedule evidence durable before suspend acknowledgement and success durable before continuation. |
| `WaitEvent` | `StepStarted`, `WaitScheduledEvent`; after event/resume, `StepSucceeded`. | Replay keeps the run waiting without the durable resume event and resumes only from recorded evidence. | Strict proof requires wait schedule durable before suspend acknowledgement and success durable before continuation. |
| `Ask` | `StepStarted`, `AskScheduledEvent`; after answer, `AskAnsweredEvent` or `RunAnswered`, answer `SlotWrittenEvent`, then `StepSucceeded`. | Replay keeps the run asking until durable answer evidence exists, then hydrates the answer slot. | Strict proof requires ask schedule durable before suspend acknowledgement and answer/output durable before continuation. |
| `AskResume` | `StepStarted`, answer `SlotWrittenEvent`, then `StepSucceeded`. | Replay reconstructs the answer slot from durable evidence before continuing. | Strict proof requires answer slot durable before success acknowledgement. |
| `RetryCheck` | `StepStarted`, `RetryScheduledEvent` when retrying or `StepSucceeded`/terminal failure evidence when exhausted. | Replay uses durable retry evidence to avoid exceeding retry bounds or dropping exhausted state. | Strict proof requires retry/exhaustion evidence durable before another attempt or terminal acknowledgement. |
| `ErrorHandler` | `StepStarted`, handler-routing `SlotWrittenEvent` when an error slot is written, then `StepSucceeded` for handled flow or `RunFailedEvent` for unhandled flow. | Replay reconstructs error-slot evidence and recorded route before handler execution. | Strict proof requires error evidence durable before handler/downstream execution. |
| `Jump` | `StepStarted` then `StepSucceeded` for the numeric target transition. | Replay trusts the durable transition and prevalidated numeric target. | Strict proof requires jump completion durable before target execution is acknowledged. |
| `Finish` | `StepStarted`, optional final `StepSucceeded`, then `RunFinished`. | Replay treats `RunFinished` as terminal outcome proof and hydrates the result slot from prior slot evidence. | Strict proof requires terminal event durable before completion acknowledgement. |
<!-- END PRIMITIVE DURABILITY PROOF MATRIX -->
