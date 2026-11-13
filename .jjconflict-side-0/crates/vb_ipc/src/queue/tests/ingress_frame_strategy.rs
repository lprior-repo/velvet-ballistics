use crate::bounded::MaxPayloadBytes;
use crate::commands::IpcCommand;
use crate::frame_types::IpcFrameHeader;
use crate::ingress::IngressFrame;
use bytes::Bytes;
use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};

fn arb_raw_payload() -> impl Strategy<Value = Bytes> {
    proptest::collection::vec(any::<u8>(), 0..=1024usize).prop_map(Bytes::from)
}

fn checked_frame(run: RunId, digest: WorkflowDigest, payload: Bytes) -> Option<IngressFrame> {
    let result = IngressFrame::new(run, digest, payload, MaxPayloadBytes::DEFAULT);
    assert_eq!(result.as_ref().map(|_| ()), Ok(()));
    match result {
        Ok(frame) => Some(frame),
        Err(_) => None,
    }
}

pub(super) fn arb_ingress_frame() -> impl Strategy<Value = IngressFrame> {
    (
        any::<u64>(),
        any::<[u8; 32]>(),
        prop_oneof![arb_raw_payload(), arb_header_payload()],
    )
        .prop_filter_map("frame construction", move |(run, digest, payload)| {
            checked_frame(RunId::new(run), WorkflowDigest::from_bytes(digest), payload)
        })
}

fn known_commands() -> Vec<IpcCommand> {
    vec![
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
    ]
}

fn arb_known_command() -> impl Strategy<Value = IpcCommand> {
    proptest::sample::select(known_commands())
}

fn arb_unknown_command() -> impl Strategy<Value = IpcCommand> {
    prop_oneof![Just(0u16), 12u16..=u16::MAX].prop_map(IpcCommand::UnknownCommand)
}

fn arb_command() -> impl Strategy<Value = IpcCommand> {
    prop_oneof![arb_known_command(), arb_unknown_command()]
}

fn arb_header_payload() -> impl Strategy<Value = Bytes> {
    (0u16..=1000u16).prop_flat_map(|length| {
        (
            arb_command(),
            any::<u16>(),
            any::<u64>(),
            proptest::collection::vec(any::<u8>(), usize::from(length)),
        )
            .prop_filter_map(
                "checked header encoding",
                move |(command, flags, correlation, payload)| {
                    wire_bytes(command, flags, correlation, length, payload)
                },
            )
    })
}

fn wire_bytes(
    command: IpcCommand,
    flags: u16,
    correlation: u64,
    length: u16,
    payload: Vec<u8>,
) -> Option<Bytes> {
    assert_eq!(payload.len(), usize::from(length));
    let encoded = IpcFrameHeader::new(command, flags, correlation, u32::from(length)).encode();
    assert_eq!(encoded.as_ref().map(|_| ()), Ok(()));
    match encoded {
        Ok(header) => {
            let mut bytes = Vec::from(header);
            bytes.extend_from_slice(&payload);
            Some(Bytes::from(bytes))
        }
        Err(_) => None,
    }
}
