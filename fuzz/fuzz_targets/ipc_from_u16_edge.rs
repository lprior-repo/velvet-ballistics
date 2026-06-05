#![forbid(unsafe_code)]
//! Fuzz target: from_u16() edge-case robustness for hostile u16 inputs.
//!
//! PO-FUZZ-002: Complement to Kani exhaustive proof — fuzz tests the
//! compiled binary's from_u16() with edge-case u16 values (0, u16::MAX,
//! boundaries, special bit patterns) to catch any runtime-vs-model mismatches.
//!
//! The input bytes are interpreted as little-endian u16 values. For each
//! pair of bytes, from_u16() is called. The target asserts:
//! 1. No panic occurs (catch_unwind).
//! 2. The result is always Ok (never Err).
//! 3. For values 1..=11, the variant matches the expected named variant.
//! 4. For all other values, the result is UnknownCommand(n) with n preserved.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_from_u16_edge(data);
});

/// Exercises IpcCommand::from_u16() for every complete u16 value in the input.
/// Inputs shorter than 2 bytes are ignored silently.
fn fuzz_from_u16_edge(data: &[u8]) {
    // Process every complete u16 (2-byte little-endian) in the input.
    for chunk in data.chunks_exact(2) {
        let value = u16::from_le_bytes([chunk[0], chunk[1]]);
        exercise_from_u16(value);
    }
}

fn exercise_from_u16(value: u16) {
    use vb_ipc::IpcCommand;

    // Catch any panic — the function must never panic for any u16.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        IpcCommand::from_u16(value)
    }));

    let parsed = match result {
        Ok(parsed) => parsed,
        Err(_panic) => {
            // Panic is a blocking defect for any u16 input.
            panic!(
                "from_u16({}) panicked — must be panic-free for all u16",
                value
            );
        }
    };

    // from_u16 must always return Ok.
    let command = match parsed {
        Ok(cmd) => cmd,
        Err(e) => {
            panic!(
                "from_u16({}) returned Err({:?}) — must be Ok for all u16",
                value, e
            );
        }
    };

    // Verify correct variant mapping.
    match value {
        1 => assert_eq!(
            command,
            IpcCommand::SubmitRun,
            "value 1 must be SubmitRun"
        ),
        2 => assert_eq!(
            command,
            IpcCommand::SubmitRunInline,
            "value 2 must be SubmitRunInline"
        ),
        3 => assert_eq!(
            command,
            IpcCommand::CancelRun,
            "value 3 must be CancelRun"
        ),
        4 => assert_eq!(
            command,
            IpcCommand::InspectRun,
            "value 4 must be InspectRun"
        ),
        5 => assert_eq!(
            command,
            IpcCommand::ListEvents,
            "value 5 must be ListEvents"
        ),
        6 => assert_eq!(
            command,
            IpcCommand::AnswerAsk,
            "value 6 must be AnswerAsk"
        ),
        7 => assert_eq!(
            command,
            IpcCommand::CompleteAction,
            "value 7 must be CompleteAction"
        ),
        8 => assert_eq!(
            command,
            IpcCommand::FailAction,
            "value 8 must be FailAction"
        ),
        9 => assert_eq!(
            command,
            IpcCommand::DrainTrace,
            "value 9 must be DrainTrace"
        ),
        10 => assert_eq!(
            command,
            IpcCommand::Health,
            "value 10 must be Health"
        ),
        11 => assert_eq!(
            command,
            IpcCommand::Shutdown,
            "value 11 must be Shutdown"
        ),
        _ => assert_eq!(
            command,
            IpcCommand::UnknownCommand(value),
            "value {} must be UnknownCommand({})",
            value,
            value
        ),
    }
}
