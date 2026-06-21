# SC-011: `decode_envelope_only` does not verify the payload BLAKE3 digest

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/codec/envelope.rs:27-62`
- **Confidence**: confirmed

## Description

`decode_envelope_only` decodes the header (which validates the header CRC) and returns the raw payload slice but never calls `verify_digest_match` against the header's `payload_digest`. The crate's own engineering rules require "CRC / digest verification must be mandatory, never skippable". The doc-comment claims the function is "useful for inspection-only workflows", but a caller that uses the raw bytes to make any decision (e.g., doctor's diagnosis) is operating on unverified data.

## Evidence

```rust
// crates/vb_storage/src/codec/envelope.rs:31-52
pub fn decode_envelope_only(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    let header: RecordHeader = decode_record_header(bytes, expected_magic, max_payload_len)?;
    let payload_start = RECORD_HEADER_BYTES;
    let payload_len: usize = header.payload_len.try_into()...;
    let payload_end = payload_start.checked_add(payload_len)...;
    let raw_payload = bytes.get(payload_start..payload_end)...;
    if payload_end != bytes.len() { ... }
    // <-- No verify_digest_match(raw_payload, header.payload_digest) call
    let envelope = RecordEnvelope { ... };
    Ok((envelope, raw_payload))
}
```

Compare with `decode_record_payload` (`crates/vb_storage/src/codec/payload.rs:76-95`) which calls `verify_digest_match(payload, header.payload_digest)?` between the bounds check and the trailing-bytes check. `decode_envelope_only` skips the equivalent step.

## Adversarial Check

The function's docstring justifies the omission by saying the caller wants raw bytes "without calling postcard deserialization". That justifies skipping postcard decode, not skipping BLAKE3. A bit-flip on disk (the exact failure mode BLAKE3 is meant to detect) produces a `RecordEnvelope` whose declared `payload_digest` does not match the returned slice, and the caller has no way to know. The doctor diagnostic workflow is supposed to surface corruption; without digest verification, doctor itself will silently present corrupt bytes as authoritative. The engineering rule cited in the bug-hunt brief explicitly disallows this trade-off.

## Suggested Fix

Add `verify_digest_match(raw_payload, header.payload_digest)?` immediately before constructing the `RecordEnvelope`. Callers that truly want to skip the hash (none should) can request a separate `_skip_digest` function gated behind a feature flag, but the default must verify.
