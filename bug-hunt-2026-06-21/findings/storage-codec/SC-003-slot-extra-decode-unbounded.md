# SC-003: `decode_slot_written_extra` performs unbounded postcard allocation on attacker-controlled bytes

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/slot_extra.rs:60-69`
- **Confidence**: confirmed

## Description

`decode_slot_written_extra` strips the 5-byte `VBSE\x01` prefix and immediately calls `postcard::from_bytes::<SlotWrittenExtraEnvelope>` without any payload length cap. The envelope contains `frame_extra: Option<Vec<u8>>`, so postcard reads a varint length prefix from the input and allocates a `Vec` of that size. A malformed or hostile payload can request arbitrary allocation (up to `usize::MAX` on 64-bit) before any byte of the payload is validated.

## Evidence

```rust
// crates/vb_storage/src/slot_extra.rs:22-28
pub struct SlotWrittenExtraEnvelope {
    pub taint: Taint,
    pub frame_extra: Option<Vec<u8>>,        // <-- unbounded Vec
}

// crates/vb_storage/src/slot_extra.rs:60-69
pub fn decode_slot_written_extra(
    bytes: &[u8],
) -> Result<DecodedSlotWrittenExtra<'_>, SlotWrittenExtraError> {
    match bytes.strip_prefix(SLOT_WRITTEN_EXTRA_PREFIX) {
        Some(payload) => postcard::from_bytes::<SlotWrittenExtraEnvelope>(payload)
            .map(DecodedSlotWrittenExtra::Envelope)
            .map_err(|_| SlotWrittenExtraError::DecodeFailed),
        None => Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes)),
    }
}
```

No `MAX_FRAME_EXTRA_BYTES` constant exists in `constants.rs` and no length check runs before `postcard::from_bytes`. By contrast, every other decode path in the crate (`decode_record`, `decode_accepted_artifact_envelope`, `decode_journal_event`) routes through `MAX_*_PAYLOAD_BYTES` enforcement.

## Adversarial Check

Postcard's varint is bounded by `u32::MAX` for collection lengths in the wire format, which still allows ~4 GiB allocation per call. The slot-extra payload is read from a Fjall value cell that has no built-in size cap (the cell can in principle be up to `MAX_BLOB_BYTES` = 64 MiB). Even at "only" 64 MiB, every slot-written event decode allocates that much memory; a hostile Fjall image that substitutes a malicious varint triggers OOM. The codebase's own engineering rule ("Bounded resources: every Vec/HashMap growth must hit a configured cap") is violated here directly.

## Suggested Fix

Introduce a `MAX_FRAME_EXTRA_BYTES` constant (e.g. 64 KiB), check `payload.len()` against it before invoking `postcard::from_bytes`, and return a new error variant (e.g. `SlotWrittenExtraError::Oversized`) when exceeded. Alternatively, deserialize into a `HeapBytes`-like wrapper that enforces a cap at the postcard layer.
