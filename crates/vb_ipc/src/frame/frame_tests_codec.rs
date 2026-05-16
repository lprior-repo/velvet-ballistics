//! IPC frame tests - roundtrip and codec tests.

use super::{
    decode_frame_header, decode_frame_payload, encode_frame, read_frame_payload,
    write_frame, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes, IPC_HEADER_LEN,
};

#[cfg(test)]
macro_rules! assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        match &$result {
            Ok(_) => (),
            Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
        }
    }};
}

fn assert_command_roundtrip(command: IpcCommand) {
    let frame_result = encode_frame(command, 0, 7, b"");
    assert_ok!(frame_result, "encode should succeed for {command:?}");
    let Ok(frame_bytes) = frame_result else { return; };
    let header_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "header should decode for {command:?}");
    let Ok(header) = header else { return; };
    assert_eq!(header.command, command, "command should roundtrip");
}

fn assert_payload_roundtrip(command: IpcCommand) {
    let payload = b"test";
    let frame = encode_frame(command, 0, 42, payload);
    assert_ok!(frame, "encode should succeed for {command:?}");
    let Ok(frame_bytes) = frame else { return; };
    let header_arr: [u8; IPC_HEADER_LEN] = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => match s.try_into() {
            Ok(a) => a,
            Err(_) => return,
        },
        None => return,
    };
    let decoded = decode_frame_header(&header_arr);
    assert_ok!(decoded, "decode should succeed for {command:?}");
    let Ok(header) = decoded else { return; };
    assert_eq!(header.command, command, "command should roundtrip for {command:?}");
    assert_eq!(header.correlation, 42);
    let payload_len = match usize::try_from(header.payload_len) {
        Ok(v) => v,
        Err(_) => return,
    };
    assert_eq!(payload_len, 4);
    assert_eq!(frame_bytes.get(IPC_HEADER_LEN..), Some(payload.as_slice()));
}

#[test]
fn encode_frame_produces_valid_header_and_payload() {
    let payload = b"test-data";
    let result = encode_frame(IpcCommand::Health, 0, 99, payload);
    assert_ok!(result, "encode_frame should succeed");
    let Ok(frame) = result else { return; };
    assert!(frame.len() > IPC_HEADER_LEN, "frame should contain header plus payload");
    let header_slice = match frame.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_bytes: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header_result = decode_frame_header(&header_bytes);
    assert_ok!(header_result, "header should decode");
    let Ok(header) = header_result else { return; };
    assert_eq!(header.command, IpcCommand::Health);
    assert_eq!(header.correlation, 99);
    let payload_len = match usize::try_from(header.payload_len) {
        Ok(v) => v,
        Err(_) => return,
    };
    assert_eq!(payload_len, payload.len());
    assert_eq!(frame.get(IPC_HEADER_LEN..), Some(payload.as_slice()));
}

#[test]
fn encode_frame_roundtrip_with_empty_payload() {
    let payload: &[u8] = b"";
    let frame_result = encode_frame(IpcCommand::Health, 0, 42, payload);
    assert_ok!(frame_result, "encode should succeed");
    let Ok(frame_bytes) = frame_result else { return; };
    let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "header should decode");
    let Ok(header) = header else { return; };
    assert_eq!(header.command, IpcCommand::Health);
    assert_eq!(header.correlation, 42);
    assert_eq!(header.payload_len, 0);
    let payload_section = frame_bytes.get(IPC_HEADER_LEN..);
    assert_eq!(payload_section, Some(&[][..]));
}

#[test]
fn encode_frame_roundtrip_with_large_payload() {
    let payload = vec![0xAB_u8; 1024];
    let frame_result = encode_frame(IpcCommand::SubmitRun, 0, 99, &payload);
    assert_ok!(frame_result, "encode should succeed");
    let Ok(frame_bytes) = frame_result else { return; };
    let header_bytes_slice = match frame_bytes.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_bytes_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "header should decode");
    let Ok(header) = header else { return; };
    assert_eq!(header.command, IpcCommand::SubmitRun);
    assert_eq!(header.correlation, 99);
    assert_eq!(header.payload_len, 1024);
    let payload_section = frame_bytes.get(IPC_HEADER_LEN..);
    assert_eq!(payload_section, Some(payload.as_slice()));
}

#[test]
fn decode_frame_payload_succeeds_for_matching_length() {
    let payload = crate::IpcPayload::Health;
    let payload_bytes = postcard::to_allocvec(&payload);
    assert_ok!(payload_bytes, "payload should encode");
    let Ok(payload_bytes) = payload_bytes else { return; };
    let payload_len = match u32::try_from(payload_bytes.len()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, payload_len);
    let result = decode_frame_payload(&header, &payload_bytes);
    assert_ok!(result, "payload should decode");
    let Ok(decoded) = result else { return; };
    assert_eq!(decoded, crate::IpcPayload::Health);
}

#[test]
fn write_frame_produces_valid_frame_on_writer() {
    let mut writer = Vec::new();
    let result = write_frame(&mut writer, IpcCommand::Shutdown, 0, 55, b"bye");
    assert_ok!(result, "write_frame should succeed");
    assert!(writer.len() > IPC_HEADER_LEN, "should contain header + payload");
    let header_slice = match writer.get(..IPC_HEADER_LEN) {
        Some(s) => s,
        None => return,
    };
    let header_arr: [u8; IPC_HEADER_LEN] = match header_slice.try_into() {
        Ok(h) => h,
        Err(_) => return,
    };
    let header = decode_frame_header(&header_arr);
    assert_ok!(header, "written header should decode");
    let Ok(header) = header else { return; };
    assert_eq!(header.command, IpcCommand::Shutdown);
    assert_eq!(header.correlation, 55);
}

#[test]
fn read_frame_payload_returns_exact_bytes_when_available() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let payload_data = b"test";
    let mut cursor = std::io::Cursor::new(payload_data.as_slice());
    let result = read_frame_payload(&mut cursor, &header);
    assert_ok!(result, "payload should read");
    let Ok(payload) = result else { return; };
    assert_eq!(payload.as_slice(), b"test");
}

#[test]
fn encode_empty_payload_succeeds() {
    let payload: &[u8] = b"";
    let result = encode_frame(IpcCommand::Health, 0, 1, payload);
    assert_ok!(result, "empty payload should encode");
    let Ok(frame) = result else { return; };
    assert_eq!(frame.len(), IPC_HEADER_LEN);
}

#[test]
fn encode_payload_at_max_boundary_succeeds() {
    let max = MaxPayloadBytes::DEFAULT.get();
    let payload = vec![0xAB_u8; max];
    let result = encode_frame(IpcCommand::SubmitRun, 0, 1, &payload);
    assert_ok!(result, "max-size payload should encode");
    let Ok(frame) = result else { return; };
    assert_eq!(frame.len(), IPC_HEADER_LEN.checked_add(max).map_or(0, |v| v));
}

#[test]
fn decode_frame_roundtrip_preserves_submit_run_command() { assert_command_roundtrip(IpcCommand::SubmitRun); }
#[test]
fn decode_frame_roundtrip_preserves_submit_run_inline_command() { assert_command_roundtrip(IpcCommand::SubmitRunInline); }
#[test]
fn decode_frame_roundtrip_preserves_cancel_run_command() { assert_command_roundtrip(IpcCommand::CancelRun); }
#[test]
fn decode_frame_roundtrip_preserves_inspect_run_command() { assert_command_roundtrip(IpcCommand::InspectRun); }
#[test]
fn decode_frame_roundtrip_preserves_list_events_command() { assert_command_roundtrip(IpcCommand::ListEvents); }
#[test]
fn decode_frame_roundtrip_preserves_answer_ask_command() { assert_command_roundtrip(IpcCommand::AnswerAsk); }
#[test]
fn decode_frame_roundtrip_preserves_complete_action_command() { assert_command_roundtrip(IpcCommand::CompleteAction); }
#[test]
fn decode_frame_roundtrip_preserves_fail_action_command() { assert_command_roundtrip(IpcCommand::FailAction); }
#[test]
fn decode_frame_roundtrip_preserves_drain_trace_command() { assert_command_roundtrip(IpcCommand::DrainTrace); }
#[test]
fn decode_frame_roundtrip_preserves_health_command() { assert_command_roundtrip(IpcCommand::Health); }
#[test]
fn decode_frame_roundtrip_preserves_shutdown_command() { assert_command_roundtrip(IpcCommand::Shutdown); }

#[test]
fn roundtrip_submit_run_payload() { assert_payload_roundtrip(IpcCommand::SubmitRun); }
#[test]
fn roundtrip_submit_run_inline_payload() { assert_payload_roundtrip(IpcCommand::SubmitRunInline); }
#[test]
fn roundtrip_cancel_run_payload() { assert_payload_roundtrip(IpcCommand::CancelRun); }
#[test]
fn roundtrip_inspect_run_payload() { assert_payload_roundtrip(IpcCommand::InspectRun); }
#[test]
fn roundtrip_list_events_payload() { assert_payload_roundtrip(IpcCommand::ListEvents); }
#[test]
fn roundtrip_answer_ask_payload() { assert_payload_roundtrip(IpcCommand::AnswerAsk); }
#[test]
fn roundtrip_complete_action_payload() { assert_payload_roundtrip(IpcCommand::CompleteAction); }
#[test]
fn roundtrip_fail_action_payload() { assert_payload_roundtrip(IpcCommand::FailAction); }
#[test]
fn roundtrip_drain_trace_payload() { assert_payload_roundtrip(IpcCommand::DrainTrace); }
#[test]
fn roundtrip_health_payload() { assert_payload_roundtrip(IpcCommand::Health); }
#[test]
fn roundtrip_shutdown_payload() { assert_payload_roundtrip(IpcCommand::Shutdown); }
