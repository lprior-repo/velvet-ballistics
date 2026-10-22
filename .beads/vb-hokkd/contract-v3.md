# Registered Action Dispatch Contract V3

Source-bound to actual signatures. No invented APIs. Plain English.

## Existing Signatures (verified, unchanged)

`Runtime::execute_registered_action(adapter, run, wire_step, grant)`
- Returns `Result<RegisterReceipt, RegisteredExecutionError>`
- Calls `FjallRegister::execute(grant, ticket, value)` after context validation
- Enqueues completion post-effect; queue failure preserves receipt

`Runtime::reconcile_registered_action(adapter, run, wire_step, grant)`
- Returns `Result<RegisterReceipt, RegisteredExecutionError>`
- Calls `FjallRegister::reconcile(grant, ticket, value)` — never production effect
- Requires `catalog.require_receipt_recovery(artifact, action)` first

`FjallRegister::execute(grant, ticket, value)`
- Returns `Result<RegisterReceipt, ExecutorError>`
- Bounded 4096 receipts, 512 bytes each, target idempotency built-in

`Shard::registered_action_context(ticket)`
- Returns `RuntimeResult<RegisteredActionContext>`
- Reads actual frame input taint, requires `Taint::Clean`
- Reads actual slot, requires `SlotValue::I64`
- Serializes locally for `encoded_len` only; discards bytes
- No encoded bytes, no per-run policy pin in output

`LivePolicy::evaluate(ticket, pinned_admission, input_bytes, taint)`
- Returns `RuntimeResult<DecisionRecord>`
- Emergency overlay active → Denied at Emergency layer
- Unconfigured → Denied at Live layer
- Matching rule → rule's code; non-matching → Denied

## Proposed Context Extension (explicit, not yet implemented)

`RegisteredActionContext` gains per-run retained admission pin:
- Carries pin from `state.admission` at run admission, not reconstructed at dispatch
- Rejects `PinnedAdmissionPolicy::Unverified` for new live authority
- Preserves independent artifact digest validation via catalog

Runtime dispatch boundary before `adapter.execute`:
- Evaluate ticket via `LivePolicy::evaluate` with retained pin and actual input bytes/taint
- Journal complete `DecisionRecord` with shard-owned `EventSeq` before any effect
- `DecisionRecord` has exactly nine fields; no `SourceSpanId` field
- Journal failure → no new effect, return journal error

Default-deny: new `Runtime` starts `Unconfigured`; first action evaluates to Denied
until trusted host config. No silently restored Allow.

Emergency control: durable store-scoped marker (child vb-ct31x), not runtime-instance bool.
Runtime checks current store-scoped marker before every new effect; load/corruption/unavailable
fail closed. Runtime may cache pure policy config actor-owned, but no mutable accessor,
no clear overlay. No mutable setter/reset; no `()` acknowledgement of failed persistence.

Historical reconcile: unchanged — reads existing receipt, never invokes production effect,
preserves `catalog.require_receipt_recovery`, artifact/grant matching, adapter reconcile.

Remaining unresolved: tag38 journal formatter finalization blocks canonical storage encoding.
