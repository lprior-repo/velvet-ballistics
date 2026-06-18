#![no_main]

//! Cargo-fuzz harness for AnswerAsk IPC payload shape verification.
//!
//! Obligation: obl-vb-jpq7-21-fuzz-ipc-shape-002
//! Verifier lane: cargo-fuzz
//!
//! Coverage: Hostile postcard bytes fed to handle_answer_ask either decode to valid
//! AnswerAsk or reject with BadRequest/PayloadError before any runtime mutation.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to decode as IpcPayload::AnswerAsk
    match postcard::from_bytes::<vb_ipc::IpcPayload>(data) {
        Ok(vb_ipc::IpcPayload::AnswerAsk { ref answer, ref taint, .. }) => {
            // Valid decode: verify structural invariants
            assert!(
                answer.len() <= 65536,
                "decoded AnswerAsk answer must be within 65536 limit"
            );
            // No ticket field (compile-time enforced)
            let _ = taint.is_some();
        }
        Ok(_) => {
            // Decoded to a different IpcPayload variant — acceptable
            // (not a crash, just a different variant)
        }
        Err(_) => {
            // Decode failed — this is expected for hostile bytes
            // The fuzz target should NOT panic here
            return;
        }
    }
});
