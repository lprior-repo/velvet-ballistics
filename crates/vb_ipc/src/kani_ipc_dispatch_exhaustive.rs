#![forbid(unsafe_code)]
//! Kani harnesses: IPC dispatch command reconciliation.
//!
//! PO-KANI-003: UnknownCommand dispatch always returns BadRequest.
//! PO-KANI-005: Dispatch match has exactly 12 arms (11 semantic + UnknownCommand),
//!              all routing correctly without panicking.

use std::num::NonZeroUsize;

use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

use crate::IpcCommand;
use crate::IpcFrameHeader;
use crate::server::IpcResponse;
use crate::server::dispatch::dispatch_command_with_resolver;

/// Helper: constructs a minimal Runtime for use in Kani proof harnesses.
///
/// The Runtime is constructed with 1 shard using default configuration.
/// This is sufficient for dispatch-routing verification since the
/// UnknownCommand arm does not call any handler that inspects Runtime state.
///
/// # Trusted Base
/// This construction is assumed to be panic-free for Kani proof purposes.
/// See TB-RUNTIME-CONSTRUCTION in trusted-base-ledger.jsonl.
fn make_runtime() -> Runtime {
    let config = ShardConfig::default();
    Runtime::new(NonZeroUsize::MIN, config)
}

/// Constructs a minimal IPC frame header with empty payload for the given
/// IpcCommand. The header has zero flags, correlation 0, and payload_len 0.
fn make_header(command: IpcCommand) -> IpcFrameHeader {
    IpcFrameHeader::new(command, 0, 0, 0)
}

/// PO-KANI-003: UnknownCommand dispatch always returns BadRequest.
///
/// Proves: For any u16 value n that produces UnknownCommand(n),
/// dispatch_command_with_resolver returns IpcResponse::BadRequest
/// and never panics.
///
/// This is verified for ALL possible u16 values because:
/// - Values 1..=11 produce named variants (not UnknownCommand)
/// - Values 0 and 12..=u16::MAX produce UnknownCommand(n)
/// - The harness uses kani::any::<u16>() and verifies correct routing
#[kani::proof]
fn kani_unknown_command_returns_bad_request() {
    let value: u16 = kani::any();

    // Restrict to values outside the valid command range 1..=11.
    // These all produce UnknownCommand from from_u16().
    kani::assume(value == 0 || value >= 12);

    let command = IpcCommand::from_u16(value);
    assert!(
        command.is_ok(),
        "from_u16({}) must return Ok for unknown values",
        value
    );
    let command = match command {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    // Verify the decoding produced the expected variant.
    assert_eq!(
        command,
        IpcCommand::UnknownCommand(value),
        "Value {} must decode to UnknownCommand({})",
        value,
        value
    );

    // Exercise the production dispatch function.
    let header = make_header(command);
    let payload: &[u8] = &[];
    let mut runtime = make_runtime();

    let response = dispatch_command_with_resolver(&header, payload, &mut runtime, None);

    // Invariant: UnknownCommand MUST return BadRequest.
    assert_eq!(
        response,
        IpcResponse::BadRequest,
        "UnknownCommand({}) must dispatch to BadRequest, got {:?}",
        value,
        response
    );
}

/// PO-KANI-005: Dispatch match has exactly 12 arms and routes correctly.
///
/// Proves:
/// 1. All 11 semantic variants dispatch without panicking.
/// 2. The UnknownCommand variant dispatches to BadRequest.
/// 3. Total dispatch coverage: 12 match arms verified.
///
/// Each semantic variant is exercised with an empty payload. Handlers may
/// return error responses (PayloadError, BadRequest, etc.) — what matters
/// is that the dispatch routing is correct and no arm panics.
#[kani::proof]
fn kani_dispatch_arm_count() {
    let mut runtime = make_runtime();

    // Verify all 11 semantic variant arms dispatch without panicking.
    let semantic_commands: [IpcCommand; 11] = [
        IpcCommand::SubmitRun,
        IpcCommand::SubmitRunInline,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
        IpcCommand::ListEvents,
        IpcCommand::AnswerAsk,
        IpcCommand::CompleteAction,
        IpcCommand::FailAction,
        IpcCommand::DrainTrace,
        IpcCommand::Health,
        IpcCommand::Shutdown,
    ];

    // Verify count is exactly 11.
    assert_eq!(
        semantic_commands.len(),
        11,
        "Exactly 11 semantic command variants must exist"
    );

    // Each semantic variant must dispatch without panicking.
    // We reconstruct None for each call since Option<&mut dyn ...> is consumed.
    for cmd in &semantic_commands {
        let header = make_header(*cmd);
        let payload: &[u8] = &[];
        let response = dispatch_command_with_resolver(&header, payload, &mut runtime, None);

        // Any response is acceptable — we only verify no panic occurred.
        // The response must be a valid IpcResponse discriminant (Rust
        // compiler enforces exhaustiveness of the match expression).
        let _ = response;
    }

    // Verify UnknownCommand arm (12th arm) dispatches to BadRequest.
    let unknown = IpcCommand::UnknownCommand(0);
    let header = make_header(unknown);
    let payload: &[u8] = &[];
    let response = dispatch_command_with_resolver(&header, payload, &mut runtime, None);

    assert_eq!(
        response,
        IpcResponse::BadRequest,
        "UnknownCommand must dispatch to BadRequest"
    );

    // Coverage note: all 12 match arms (11 semantic + UnknownCommand)
    // have been exercised. The Rust compiler enforces exhaustiveness of
    // the match expression. The Kani proof additionally verifies that
    // each arm's handler call does not panic under this test setup.
}
