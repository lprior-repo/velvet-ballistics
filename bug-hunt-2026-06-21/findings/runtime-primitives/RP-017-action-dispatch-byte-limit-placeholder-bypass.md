# RP-017: Generic Action Dispatch Does Not Enforce Positive Payload Byte Limits

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/action.rs:205`
- **Confidence**: confirmed

## Description
`ActionRegistry::dispatch` calls `dispatch_generic`, which calls `validate_input_bytes`, but the validator ignores the input and cannot enforce `contract.max_input_bytes` for positive limits. Generic runtime dispatch can therefore create a suspended ticket without checking the action payload byte bound.

## Evidence
The generic dispatch path invokes the validator before creating the ticket:

```rust
183: pub fn dispatch_generic(
184:     input: &ActionInput,
185:     contract: &ActionContract,
186: ) -> ActionResult<ActionOutcome> {
187:     validate_input_bytes(input, contract)?;
```

The validator does not inspect `input`; it only rejects the special case where the maximum is zero and the contract expects input slots:

```rust
205: /// Validates that the input payload does not exceed the contract's byte limit.
206: fn validate_input_bytes(_input: &ActionInput, contract: &ActionContract) -> ActionResult<()> {
207:     // Byte-level validation requires encoded_len from the caller.
208:     // This is a structural check placeholder; actual byte counting
209:     // happens at the IPC boundary.
210:     if contract.max_input_bytes == 0 && contract.input_slot_count > 0 {
211:         return Err(ActionError::PayloadTooLarge {
212:             max_bytes: 0,
213:             actual_bytes: 0,
214:         });
215:     }
216:     Ok(())
```

For `max_input_bytes > 0`, the function always returns `Ok(())` regardless of actual payload size.

## Adversarial Check
The comment says byte counting happens at the IPC boundary, but this public dispatch path presents itself as runtime contract validation and has no proof-carrying token that the boundary check occurred. If the boundary is mandatory, `dispatch_generic` should require already-validated input rather than silently accepting an unmeasured payload.

## Suggested Fix
Carry an encoded payload length, or a prevalidated bounded payload type, into `ActionInput`/dispatch. Enforce `actual_bytes <= contract.max_input_bytes` before constructing `ActionTicket`, and remove the placeholder validator if byte validation truly belongs exclusively to another boundary.
