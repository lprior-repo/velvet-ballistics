---
section: 19
title: "Action ABI"
parent: velvet-ballistics-MASTER.md
---

## 19. Action ABI


Actions are native Rust operations registered by numeric `ActionId` at compile time. Runtime dispatch never string-lookups action names.

Action names are resolved to `ActionId` during compile. The runtime dispatches by `ActionId` only. There is no `async_trait`, no dynamic string lookup, and no JSON input/output model.

Action contract:

```rust
pub struct ActionContract {
    pub id: ActionId,
    pub input_slot_count: u16,
    pub output_slot_count: u16,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub timeout_ms: u64,
    pub idempotency: Idempotency,
}

pub enum Idempotency {
    DeterministicPure,
    IdempotentExternal,
    AtLeastOnceExternal,
}

pub struct ActionInput {
    pub run: RunId,
    pub step: StepIdx,
    pub action: ActionId,
    pub input: SlotIdx,
    pub ticket: ActionTicket,
}

pub struct ActionOutput {
    pub output: SlotIdx,
    pub status: ActionOutcome,
}

pub type ActionResult<T> = Result<T, ActionError>;

pub struct ActionTicket {
    pub run: RunId,
    pub step: StepIdx,
    pub seq: SeqNo,
    pub action: ActionId,
    pub attempt: u16,
    pub idempotency_key: u128,
}

pub struct ActionOutputReady {
    pub output_slot: SlotIdx,
    pub value: SlotValue,
    pub taint: Taint,
    pub encoded_len: u32,
}

pub struct ActionFailure {
    pub code: ActionFailureCode,
    pub retryable: bool,
    pub taint: Taint,
    pub detail: Option<BlobId>,
    pub encoded_len: u32,
}

pub enum ActionFailureCode {
    Rejected,
    Timeout,
    RateLimited,
    ResourceExhausted,
    ExternalUnavailable,
    InvalidInput,
    PermissionDenied,
    Conflict,
    Unknown,
}

pub enum ActionError {
    UnknownAction { action: ActionId },
    InvalidTicket { ticket: ActionTicket },
    PayloadTooLarge { len: u32, max: u32 },
    OutputSlotOutOfBounds { slot: SlotIdx },
    NonIdempotentReplayBlocked { ticket: ActionTicket },
    CompletionAlreadyRecorded { ticket: ActionTicket },
    QueueFull,
    EncodingFailed,
    DispatchFailed,
}

pub enum ActionOutcome {
    Ready(ActionOutputReady),
    Suspended(ActionTicket),
    Failed(ActionFailure),
}
```

Action ABI referenced types are part of the stable binary contract. `ActionOutputReady.value` must be a handle-only `SlotValue`; large action output bytes are stored as a blob and returned as `SlotValue::Blob(BlobId)`. `encoded_len` is the Postcard payload byte length for the completion payload and must be `<= ActionContract.max_output_bytes` and `<= ResourceContract.max_blob_bytes` when a blob is produced. `ActionFailure.detail` is optional and must point to a bounded blob; error details never use heap strings in hot state.

Action completion payloads are encoded with the binary record envelope using `record_kind` `ActionCompleted` or `ActionFailed`. The payload contains `ActionTicket`, target output slot, outcome discriminant, `SlotValue` handle or `ActionFailure`, taint, and encoded length. Decode must validate ticket/run/step/action equality, output slot bounds, payload length bounds, idempotency policy, and duplicate completion before mutating a frame.

Taint propagation: action input taint is read from the input slot. `DeterministicPure` and `IdempotentExternal` actions must return output taint at least as restrictive as input taint; a clean result from tainted input is rejected unless the action contract declares a validator-proven declassification policy. `AtLeastOnceExternal` actions propagate taint conservatively as `DerivedFromSecret` when any input is `Secret` or `DerivedFromSecret`. Failure detail taint follows the same rule and secret-tainted failure details must not enter public diagnostics without redaction.

Retry and replay semantics: `DeterministicPure` may be re-executed during replay. `IdempotentExternal` may be retried or replay-completed only with the same `ActionTicket.idempotency_key`. `AtLeastOnceExternal` may be attempted more than once only according to a bounded retry policy and must not be re-executed during recovery after a scheduled journal record; recovery waits for explicit completion/failure or marks the run blocked by policy. Duplicate completion with the same ticket and same digest is idempotently ignored; duplicate completion with different digest returns `ActionError::CompletionAlreadyRecorded` and a replay divergence error.

Static dispatch shape:

```rust
pub fn dispatch_action(action: ActionId, input: ActionInput) -> ActionResult<ActionOutcome> {
    match action {
        ActionId(0) => action_0(input),
        ActionId(1) => action_1(input),
        _ => Err(ActionError::UnknownAction { action }),
    }
}
```

Action rules:

1. Compile resolves action names to `ActionId`.
2. Runtime dispatches by `ActionId` only.
3. Inputs and outputs use `SlotValue` handles and blob references, not JSON values.
4. External side effects are at-least-once unless declared otherwise.
5. Action completion is explicit and journaled.
6. Action failures are typed and can enter `try_again` or `on_error` flows.
7. Replay policy must prevent accidental duplicate non-idempotent effects.
8. `Ready` resumes immediately with bounded output.
9. `Suspended` returns an `ActionTicket` and resumes only through direct API or IPC completion.
10. `Failed` returns typed failure data suitable for `RetryCheck` or `ErrorHandler`.

---
