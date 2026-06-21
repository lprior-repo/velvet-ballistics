# SC-012: `classify_payload_len` reports `u32::MAX` instead of the actual length when the source overflows `u32`

- **Severity**: Info
- **Category**: correctness
- **Location**: `crates/vb_storage/src/codec/payload.rs:34-43`
- **Confidence**: confirmed

## Description

When the input length cannot fit in a `u32`, `classify_payload_len` returns `PayloadLenDecision::TooLarge { len: u32::MAX, max }`, discarding the actual byte count. The resulting `JournalError::PayloadTooLarge { len: u32::MAX, .. }` misreports the offending payload size to operators and to the diagnostic-code pipeline.

## Evidence

```rust
// crates/vb_storage/src/codec/payload.rs:34-43
pub(crate) fn classify_payload_len(len: usize, max: u32) -> PayloadLenDecision {
    match u32::try_from(len) {
        Ok(payload_len) if payload_len > max => PayloadLenDecision::TooLarge {
            len: payload_len,
            max,
        },
        Ok(payload_len) => PayloadLenDecision::Accepted(payload_len),
        Err(_) => PayloadLenDecision::TooLarge { len: u32::MAX, max },  // <-- loses real len
    }
}
```

## Adversarial Check

The case can only trigger on 64-bit platforms when `len > u32::MAX`, which requires a >4 GiB payload in memory. The crate's `MAX_*_BYTES` constants are all well below `u32::MAX`, so the `Ok(_) if > max` branch handles realistic overages with the correct `len`. However, the diagnostic surface (the `JournalError::PayloadTooLarge` payload) is part of the storage contract and is consumed by doctor and operator dashboards; reporting `u32::MAX` for an actual >4 GiB input is misleading. The same defect recurs in `crates/vb_storage/src/admission/bytes.rs:63-72` (`classify_compiled_ir_value_len`).

## Suggested Fix

Either widen `PayloadTooLarge { len: u64, max: u32 }` to capture the true size, or introduce a distinct `PayloadLengthOverflow { actual_len: usize }` variant. At minimum, document the saturation behavior on the error variant.
