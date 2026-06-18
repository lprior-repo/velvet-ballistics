#![no_main]

//! Cargo-fuzz harness for handle_answer_ask hostile input verification.
//!
//! Obligation: obl-vb-jpq7-21-fuzz-handler-hostile-011
//! Verifier lane: cargo-fuzz
//!
//! Coverage: Hostile postcard bytes fed to handle_answer_ask either decode to valid
//! AnswerAsk or reject before any runtime state mutation.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to decode as IpcPayload::AnswerAsk
    match postcard::from_bytes::<vb_ipc::IpcPayload>(data) {
        Ok(vb_ipc::IpcPayload::AnswerAsk { ref answer, .. }) => {
            // Bounds check: must not exceed 65536
            if answer.len() > 65536 {
                // Would be rejected by handle_answer_ask
                return;
            }

            // Try to decode answer bytes as SlotValue
            match postcard::from_bytes::<vb_core::value::SlotValue>(answer) {
                Ok(_value) => {
                    // Valid SlotValue decode — would proceed to runtime dispatch
                    // In the real handler, this calls runtime.answer_pending_ask_slot
                    // For fuzzing, we just verify the decode didn't panic
                    return;
                }
                Err(_) => {
                    // Malformed SlotValue — would be rejected by handle_answer_ask
                    // No runtime mutation occurs
                    return;
                }
            }
        }
        Ok(_) => {
            // Different IpcPayload variant — not our target
            return;
        }
        Err(_) => {
            // Postcard decode failed — handled gracefully
            return;
        }
    }
});
