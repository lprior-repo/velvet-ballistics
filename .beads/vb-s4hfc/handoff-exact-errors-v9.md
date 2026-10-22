# Handoff: Exact Nested Postcard Error (v9)

**Bead:** vb-s4hfc
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-s4hfc-local3090-exact-errors
**Date:** 2026-10-22

## Summary

Replaced generic wildcard `Err(JournalError::PostcardDecodeFailed(_))` in `malformed_payload_rejected` test with the exact nested postcard error variant.

## Exact Nested Error

The malformed input `[1u8, 2u8, 3u8]` produces:

```
JournalError::PostcardDecodeFailed(postcard::Error::SerdeDeCustom)
```

`SerdeDeCustom` is a **unit variant** (no message payload). The message "unknown stable JournalEvent record kind tag" is discarded by `serde::de::Error::custom()` and replaced with the unit variant.

**Root cause:** Byte 0 of the malformed payload (`1`) is interpreted as an enum variant tag. During deserialization, `record_kind_from_tag(1)` calls `RecordKind::from_id(1)`, which fails and triggers `E::custom("unknown stable JournalEvent record kind tag")`. This becomes `postcard::Error::SerdeDeCustom` via the serde error trait implementation.

## Code Changes

**File:** `crates/vb_storage/tests/policy_decision_full_record.rs`

Before:
```rust
match result {
    Err(JournalError::PostcardDecodeFailed(_)) => {}
    other => {
        return Err(format!(
            "malformed payload should yield PostcardDecodeFailed; got {other:?}"
        ).into());
    }
}
```

After:
```rust
match result {
    Err(JournalError::PostcardDecodeFailed(postcard::Error::SerdeDeCustom)) => {}
    other => {
        return Err(format!(
            "malformed payload should yield PostcardDecodeFailed(SerdeDeCustom); got {other:?}"
        ).into());
    }
}
```

## Verification Method

1. Added temporary diagnostic test `diag_malformed_error_type` that decoded the same malformed input and printed the exact error variant via `eprintln!("{e:?}")`.
2. Ran with `-- --nocapture` to capture stderr output.
3. Observed: `DIAG POSTCARD ERROR: SerdeDeCustom`
4. Confirmed via postcard source (`/cache/cargo-shared/registry/src/.../postcard-1.1.3/src/error.rs`) that `SerdeDeCustom` is a unit variant.
5. Removed diagnostic test, updated production test with exact variant.
6. Ran rustfmt on `policy_decision_full_record.rs` and `pd_helpers.rs`.
7. Ran full test suite: 15/15 tests passed.

## Evidence

- Test log: `.beads/vb-s4hfc/evidence/EXACT_ERRORS_V9_01.stdout`
- All 15 tests passed including `malformed_payload_rejected`
- `rustfmt --check` passed on changed file

## Notes

- Input is deterministic; error variant is fixed for this exact byte sequence.
- Other postcard error variants (DeserializeBadVarint, DeserializeUnexpectedEnd, etc.) are NOT matched; test is now precise.