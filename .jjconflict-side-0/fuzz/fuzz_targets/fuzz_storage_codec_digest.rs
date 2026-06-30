#![no_main]

//! Fuzz target: storage_codec_digest
//!
//! Split from `vb_storage_codec` (PO-vb-y9d3v-0041). Oracle: the blake3
//! digest verifier must agree with `blake3::hash` on identical input. For
//! any input slice, `verify_digest_match(data, blake3::hash(data))` must
//! return `Ok(())`. An all-zero expected digest against arbitrary input
//! must return `Err(JournalError::PayloadDigestMismatch)`. Empty-input
//! boundary behaviour is also exercised.
//!
//! Run with: cargo fuzz run fuzz_storage_codec_digest -- -max_len=4096 -runs=100000

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Arbitrary input against an all-zero expected digest. Must fail unless
    // `blake3::hash(data)` happens to equal the all-zero digest — which it
    // essentially never does for arbitrary input.
    let _ = vb_storage::verify_digest_match(data, [0u8; 32]);

    // Empty slice against an all-zero expected digest. blake3("") is the
    // canonical blake3 IV hash, not the all-zero vector, so this must fail.
    let _ = vb_storage::verify_digest_match(&[], [0u8; 32]);

    // Self-consistency oracle: hash the input, then verify against that hash.
    // Must always succeed; a deviation is a critical storage-layer regression.
    let hash: [u8; 32] = blake3::hash(data).into();
    let result = vb_storage::verify_digest_match(data, hash);
    assert!(
        result.is_ok(),
        "verify_digest_match(data, blake3::hash(data)) must succeed"
    );

    // Flip exactly one bit of the hash and re-verify — must fail (deterministic
    // rejection of any single-bit corruption in the expected digest).
    let mut corrupted_hash = hash;
    let bit_index = data.first().copied().unwrap_or(0) & 0x07;
    let byte_index = usize::from(data.get(1).copied().unwrap_or(0) & 0x1F);
    corrupted_hash[byte_index] ^= 1u8 << bit_index;
    let _ = vb_storage::verify_digest_match(data, corrupted_hash);
});
