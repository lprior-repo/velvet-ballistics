# RP-008: `validate_prompt` only rejects `Bool`; accepts `List`, `Null`, numbers as prompts

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_runtime/src/primitives/wait_ask.rs:100-108`
- **Confidence**: likely

## Description

`validate_prompt` accepts any `SlotValue` except `Bool`, including `List`, `Null`, `I64`, and `F64`. The function is named "validate_prompt" and is used to check that the prompt slot of an `Ask` primitive is "prompt-compatible", but the actual accepted set does not match any plausible prompt semantics (which would typically be `String`-like values).

## Evidence

`crates/vb_runtime/src/primitives/wait_ask.rs:100-108`:

```rust
fn validate_prompt(value: SlotValue) -> Result<(), EngineError> {
    match value {
        SlotValue::Bool(_) => Err(EngineError::TypeMismatch {
            expected: "prompt",
            found: value.type_name(),
        }),
        _ => Ok(()),
    }
}
```

The match rejects only `Bool`. Every other variant (`I64`, `F64`, `List`, `Null`, and any future variants via the `_` arm) passes validation. The host runtime that consumes the `AwaitingAsk` signal must therefore be prepared to render arbitrary values as prompts, including `Null` and binary-encoded `List`.

Comment from the public doc (wait_ask.rs:46-47): "validates it is prompt-compatible". The set {everything except Bool} does not match any documented notion of "prompt-compatible".

## Adversarial Check

It is possible the runtime encodes prompts as `I64` (a prompt-template ID) and uses `List` (a list of template IDs), in which case the wide acceptance is intentional. Without a stronger positive case for that interpretation, however, the code looks like an under-specified validator that exists only to reject one specific type. If `Bool` was rejected because it is a legacy artifact (e.g., a previous version of the type), the validator should be re-grounded against the actual prompt encoding.

Severity Low because the host runtime can still reject bad prompts downstream; this is a defense-in-depth gap rather than a definite correctness bug.

## Suggested Fix

Either:

- Tighten to a positive list: `SlotValue::I64(_) | SlotValue::String(_) => Ok(())`, everything else → `TypeMismatch`. (Requires confirming which variants are valid prompts.)
- Or document the accepted set in the function doc-comment and rename to `validate_prompt_not_bool` if the loose check is intentional.

A Kani harness proving `validate_prompt` rejects arbitrary `SlotValue::Bool(_)` and accepts the documented prompt-shaped variants would lock in the intent.
