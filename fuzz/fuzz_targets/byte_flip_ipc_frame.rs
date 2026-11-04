//! Byte-flipping mutation testing for IPC frames.
//!
//! Takes valid IPC frames, encodes them, then systematically flips bytes
//! in the header and payload to verify that decoding produces either
//! valid frames or typed errors — never panics.

#![no_main]
use libfuzzer_sys::fuzz_target;
use vb_ipc::{
    IpcCommand, IpcError, IpcFrameHeader, IPC_HEADER_LEN,
};

/// Build a valid IPC header for the given command.
fn make_valid_header(command: IpcCommand, payload_len: u32) -> IpcFrameHeader {
    IpcFrameHeader::new(command, 0, 0, payload_len)
}

/// Assert the result is either a valid header or a typed error.
fn assert_typed_header_result(result: Result<IpcFrameHeader, IpcError>) {
    match result {
        Ok(_) => {
            // Equivalent mutation or valid header
        }
        Err(IpcError::InvalidMagic { .. }) => {}
        Err(IpcError::UnsupportedVersion { .. }) => {}
        Err(IpcError::UnknownCommand(_)) => {}
        Err(IpcError::InvalidCommandFlags { .. }) => {}
        Err(IpcError::ReservedBitsSet { .. }) => {}
        Err(IpcError::ReservedNonZero { .. }) => {}
        Err(IpcError::HeaderDecodeFailed) => {}
        Err(IpcError::PayloadLengthOutOfRange { .. }) => {}
        Err(other) => {
            drop(other);
        }
    }
}

/// Test byte-flipping of the IPC header bytes.
fn test_header_byte_flip(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let encoded = match header.encode() {
        Ok(e) => e,
        Err(_) => return,
    };

    // Corrupt with seed
    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&encoded, data);

    // Decode and verify typed result
    if corrupted.len() == IPC_HEADER_LEN {
        let corrupted_arr: [u8; IPC_HEADER_LEN] = corrupted
            .try_into()
            .unwrap();
        assert_typed_header_result(vb_ipc::frame::decode_frame_header(&corrupted_arr));
    }
}

/// Test targeted header field corruption.
fn test_targeted_header_corruption(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let encoded = match header.encode() {
        Ok(e) => e,
        Err(_) => return,
    };

    // Corrupt magic (offset 0-3)
    let mut magic_flip = encoded;
    magic_flip[0] ^= if !data.is_empty() { data[0] } else { 0xFF };
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&magic_flip));

    // Corrupt version (offset 4-5)
    let mut version_flip = encoded;
    version_flip[4] ^= if data.len() > 1 { data[1] } else { 0xFF };
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&version_flip));

    // Corrupt command ID (offset 6-7)
    let mut command_flip = encoded;
    command_flip[6] ^= if data.len() > 2 { data[2] } else { 0xFF };
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&command_flip));

    // Corrupt payload length (offset 8-11)
    let mut payload_len_flip = encoded;
    payload_len_flip[8] ^= if data.len() > 3 { data[3] } else { 0xFF };
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&payload_len_flip));
}

/// Test complete frame corruption including payload.
fn test_full_frame_corruption(data: &[u8]) {
    let header = make_valid_header(IpcCommand::SubmitRun, 4);
    let encoded_header = match header.encode() {
        Ok(e) => e,
        Err(_) => return,
    };

    let payload = [1u8, 2, 3, 4];
    let mut frame = Vec::with_capacity(IPC_HEADER_LEN + 4);
    frame.extend_from_slice(&encoded_header);
    frame.extend_from_slice(&payload);

    // Corrupt with seed
    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&frame, data);

    // Decode header first
    if corrupted.len() >= IPC_HEADER_LEN {
        let header_bytes: [u8; IPC_HEADER_LEN] = corrupted[..IPC_HEADER_LEN]
            .try_into()
            .unwrap();

        let result = vb_ipc::frame::decode_frame_header(&header_bytes);

        match result {
            Ok(decoded_header) => {
                // Try to decode payload if valid header
                if corrupted.len() > IPC_HEADER_LEN {
                    let payload_section = &corrupted[IPC_HEADER_LEN..];
                    let _ = vb_ipc::frame::decode_frame_payload(
                        &decoded_header,
                        payload_section,
                    );
                }
            }
            Err(IpcError::InvalidMagic { .. }) => {}
            Err(IpcError::UnsupportedVersion { .. }) => {}
            Err(IpcError::UnknownCommand(_)) => {}
            Err(IpcError::InvalidCommandFlags { .. }) => {}
            Err(IpcError::ReservedBitsSet { .. }) => {}
            Err(IpcError::ReservedNonZero { .. }) => {}
            Err(IpcError::PayloadLengthMismatch { .. }) => {}
            Err(IpcError::HeaderDecodeFailed) => {}
            Err(IpcError::PayloadLengthOutOfRange { .. }) => {}
            Err(other) => {
                drop(other);
            }
        }
    }
}

/// Test byte flips on each byte of the header individually.
fn test_individual_byte_flips(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let encoded = match header.encode() {
        Ok(e) => e,
        Err(_) => return,
    };

    // Use seed to determine which byte positions to flip
    let flip_count = if !data.is_empty() {
        (data[0] & 0x0F) + 1
    } else {
        1
    };

    for i in 0..flip_count {
        let i_usize = i as usize;
        let byte_pos = if data.len() > i_usize + 1 {
            data[i_usize + 1] as usize % IPC_HEADER_LEN
        } else {
            i_usize % IPC_HEADER_LEN
        };

        let mut flip_seed = vec![0u8; 16];
        flip_seed[0] = byte_pos as u8;

        let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&encoded, &flip_seed);

        if corrupted.len() == IPC_HEADER_LEN {
            let corrupted_arr: [u8; IPC_HEADER_LEN] = corrupted
                .try_into()
                .unwrap();
            assert_typed_header_result(vb_ipc::frame::decode_frame_header(&corrupted_arr));
        }
    }
}

fuzz_target!(|data: &[u8]| {
    test_header_byte_flip(data);
    test_targeted_header_corruption(data);
    test_full_frame_corruption(data);
    test_individual_byte_flips(data);
});