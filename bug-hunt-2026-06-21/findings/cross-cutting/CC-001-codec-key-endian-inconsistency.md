# CC-001: Codec endian inconsistency between record headers (LE) and storage keys (BE) is undocumented

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/binary.rs:11-63` (record-header LE); `crates/vb_storage/src/keys/encode.rs:157-204` (storage-key BE)
- **Confidence**: confirmed

## Description

The 60-byte record header uses little-endian byte order for every integer field (`u16::from_le_bytes`, `u32::from_le_bytes`, `u64::from_le_bytes` in `binary.rs`), while every storage key uses big-endian byte order (`to_be_bytes()` in `keys/encode.rs`). Both choices are individually defensible — LE for native CPU efficiency on the wire, BE for sorted LSM range scans — but neither file documents the rationale or even mentions the other side. A maintainer touching one side has no signal that the other side exists with the opposite convention.

## Evidence

```rust
// crates/vb_storage/src/binary.rs:11-17
/// Reads a little-endian u16 from bytes at offset.
pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, JournalError> {
    let end = offset.checked_add(2).ok_or(JournalError::UnexpectedEof)?;
    let slice = bytes.get(offset..end).ok_or(JournalError::UnexpectedEof)?;
    let raw = <[u8; 2]>::try_from(slice).map_err(|_| JournalError::UnexpectedEof)?;
    Ok(u16::from_le_bytes(raw))                                  // <-- LE
}
```

```rust
// crates/vb_storage/src/keys/encode.rs:161-174
fn sequenced_run_key(prefix: u8, run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
    let mut key = ArrayVec::<u8, JOURNAL_KEY_BYTES>::new();
    key.try_push(prefix).map_err(|_| JournalError::KeyCapacity)?;
    key.try_extend_from_slice(&run.get().to_be_bytes())...;       // <-- BE
    key.try_extend_from_slice(&seq.get().to_be_bytes())...;
    key.into_inner().map_err(|_| JournalError::KeyCapacity)
}
```

Neither file's module doc references the other.

## Adversarial Check

Both conventions are correct in isolation. The defect is the absence of a documented contract stating which side uses which endianness and why. A maintainer adding, say, a new fixed-width field to the record header who reflexively copies `to_be_bytes()` from the keys module will silently corrupt the wire format. The Holzman rule "Code Shall be Written to be Read" calls for explicit cross-references when conventions diverge.

## Suggested Fix

Add a paragraph to `crates/vb_storage/src/binary.rs` and `crates/vb_storage/src/keys/encode.rs` documenting the convention:

```text
//! # Endianness contract
//!
//! Record headers (`binary.rs`, `codec/header.rs`) use LITTLE-endian byte
//! order — chosen for native CPU efficiency on the wire-format decode path.
//!
//! Storage keys (`keys/encode.rs`, `keys/decode.rs`) use BIG-endian byte
//! order — chosen so that Fjall's lexicographic key sort produces numeric
//! order without transformation.
//!
//! Do not mix conventions: adding a new field to a record header must use
//! LE; adding a new field to a key must use BE.
```
