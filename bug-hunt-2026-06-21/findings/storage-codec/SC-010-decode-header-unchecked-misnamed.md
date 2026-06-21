# SC-010: `decode_record_header_unchecked_len` is misnamed — every read is bounds-checked

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/codec/header.rs:80-93`
- **Confidence**: confirmed

## Description

The function name `decode_record_header_unchecked_len` suggests unchecked length handling, but every `read_u16`/`read_u32`/`read_u64`/`digest_from_header` call inside it does full bounds checking via `bytes.get(..end).ok_or(JournalError::UnexpectedEof)?` in `crates/vb_storage/src/binary.rs:12-33`. The "unchecked" refers only to the fact that the outer caller (`decode_record_header`) pre-slices the input to `RECORD_HEADER_BYTES` before delegating. The name misleads reviewers into thinking the function is unsafe to call without that pre-slice.

## Evidence

```rust
// crates/vb_storage/src/codec/header.rs:80-93
pub(crate) fn decode_record_header_unchecked_len(
    header: &[u8],
) -> Result<RecordHeader, JournalError> {
    Ok(RecordHeader {
        magic: read_u32(header, 0)?,
        schema_version: read_u16(header, 4)?,
        record_kind: read_u16(header, 6)?,
        header_len: read_u32(header, 8)?,
        payload_len: read_u32(header, 12)?,
        sequence: read_u64(header, 16)?,
        payload_digest: digest_from_header(header)?,
        header_checksum: read_u32(header, CRC_OFFSET)?,
    })
}
```

Each `read_*` returns `Err(JournalError::UnexpectedEof)` on out-of-bounds (`crates/vb_storage/src/binary.rs:13-33`). The function is total over all `&[u8]` slices.

## Adversarial Check

This is purely a naming defect, but the Holzman Rust / NASA Power-of-Ten doctrine in this repo explicitly requires that names communicate safety contracts. A reviewer skimming for `unchecked` patterns will flag this function as requiring a manual proof of bounds — and waste time re-verifying what the body already guarantees. The `unchecked_` prefix in particular conflicts with the crate-level `#![forbid(unsafe_code)]` posture, which is meant to advertise totality.

## Suggested Fix

Rename to `decode_record_header_fields` (or `decode_record_header_raw`) to reflect that the function safely decodes header fields from any slice long enough for the individual `read_*` calls.
