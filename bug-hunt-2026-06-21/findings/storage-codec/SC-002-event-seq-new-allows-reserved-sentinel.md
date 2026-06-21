# SC-002: `EventSeq::new` allows constructing `u64::MAX` that the decoder rejects as `ReservedSeqSentinel`

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_storage/src/types/seq.rs:11-22` (constructor); `crates/vb_storage/src/keys/decode.rs:207-209, 261-263` (decoder rejection)
- **Confidence**: confirmed

## Description

`EventSeq::new(value)` is an unvalidated `const fn` that accepts any `u64`. The decoder, however, treats `u64::MAX` as a reserved sentinel and returns `KeyDecodeError::ReservedSeqSentinel`. The newtype therefore fails to enforce its own invariant: a value constructed via the public API can be encoded into a key that the decoder will never accept.

## Evidence

```rust
// crates/vb_storage/src/types/seq.rs:11-22
impl EventSeq {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 { self.0 }
    pub const MAX: Self = Self(u64::MAX);          // <-- advertised constant
}
```

```rust
// crates/vb_storage/src/keys/decode.rs:206-209
let seq_val = u64::from_be_bytes(key_array::<8>(bytes, prefix, 9..17)?);
if seq_val == u64::MAX {
    return Err(KeyDecodeError::ReservedSeqSentinel);
}
```

`EventSeq::MAX` is exported as a public constant equal to the value the decoder refuses. Any code path that uses `EventSeq::MAX` and then encodes via `run_event_key` / `run_snapshot_key` / `recovery_stamp_key` produces an undecodable record.

## Adversarial Check

One could argue `MAX` is intended only as a sentinel for "not-a-seq" in memory and never persisted. But there is no compile-time or runtime barrier preventing persistence: `run_event_key` (`crates/vb_storage/src/keys/encode.rs:46-48`) calls `seq.get().to_be_bytes()` unconditionally, and `JournalWriteBatch::append_event` (`crates/vb_storage/src/batch/write_event.rs:17-25`) accepts any `EventSeq` the caller supplies. The validation is asymmetric — enforced on decode, not encode — which violates the round-trip property the codec contract requires.

## Suggested Fix

Either remove the public `MAX` constant, or make `EventSeq::new` return `Result`/use a `NonZeroU64`-style fallible constructor, or add a `validate_seq` check inside the key-encode path that mirrors the decoder's `ReservedSeqSentinel` rule so the encoder cannot emit values the decoder rejects.
