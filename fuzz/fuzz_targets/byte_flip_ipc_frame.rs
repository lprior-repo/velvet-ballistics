//! Byte-flipping mutation testing for IPC frames.
//!
//! Takes valid IPC frames, encodes them, then systematically flips bytes
//! in the header and payload to verify that decoding produces either
//! valid frames or typed errors — never panics.

#![no_main]
use libfuzzer_sys::fuzz_target;
use vb_ipc::{IPC_HEADER_LEN, IpcCommand, IpcError, IpcFrameHeader};

/// Build a valid IPC header for the given command.
fn make_valid_header(command: IpcCommand, payload_len: u32) -> IpcFrameHeader {
    IpcFrameHeader::new(command, 0, 0, payload_len)
}

/// Assert the result is either a valid header or a typed error.
fn assert_typed_header_result(result: Result<IpcFrameHeader, IpcError>) {
    // Both a valid header and any typed decode error are acceptable.
    if let Err(error) = result {
        let _ = error;
    }
}

/// Test byte-flipping of the IPC header bytes.
fn test_header_byte_flip(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let Ok(encoded) = header.encode() else {
        return;
    };

    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&encoded, data);

    if let Ok(corrupted_arr) = <[u8; IPC_HEADER_LEN]>::try_from(corrupted.as_slice()) {
        assert_typed_header_result(vb_ipc::frame::decode_frame_header(&corrupted_arr));
    }
}

/// Test targeted header field corruption.
fn test_targeted_header_corruption(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let Ok(encoded) = header.encode() else {
        return;
    };

    // Corrupt magic (offset 0-3)
    let mut magic_flip = encoded;
    if let Some(byte) = magic_flip.get_mut(0) {
        *byte ^= data.first().copied().unwrap_or(0xFF);
    }
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&magic_flip));

    // Corrupt version (offset 4-5)
    let mut version_flip = encoded;
    if let Some(byte) = version_flip.get_mut(4) {
        *byte ^= data.get(1).copied().unwrap_or(0xFF);
    }
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&version_flip));

    // Corrupt command ID (offset 6-7)
    let mut command_flip = encoded;
    if let Some(byte) = command_flip.get_mut(6) {
        *byte ^= data.get(2).copied().unwrap_or(0xFF);
    }
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&command_flip));

    // Corrupt payload length (offset 8-11)
    let mut payload_len_flip = encoded;
    if let Some(byte) = payload_len_flip.get_mut(8) {
        *byte ^= data.get(3).copied().unwrap_or(0xFF);
    }
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&payload_len_flip));
}

/// Test complete frame corruption including payload.
fn test_full_frame_corruption(data: &[u8]) {
    let header = make_valid_header(IpcCommand::SubmitRun, 4);
    let Ok(encoded_header) = header.encode() else {
        return;
    };

    let payload = [1u8, 2, 3, 4];
    let mut frame = Vec::with_capacity(IPC_HEADER_LEN.saturating_add(4));
    frame.extend_from_slice(&encoded_header);
    frame.extend_from_slice(&payload);

    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&frame, data);

    let Some(header_bytes) = corrupted.get(..IPC_HEADER_LEN) else {
        return;
    };
    let Ok(header_bytes) = <[u8; IPC_HEADER_LEN]>::try_from(header_bytes) else {
        return;
    };
    let result = vb_ipc::frame::decode_frame_header(&header_bytes);

    match result {
        Ok(decoded_header) => {
            let payload_result = corrupted.get(IPC_HEADER_LEN..).map(|payload_section| {
                vb_ipc::frame::decode_frame_payload(&decoded_header, payload_section)
            });
            if let Some(Err(error)) = payload_result {
                let _ = error;
            }
        }
        // Any typed decode error is an acceptable mutation outcome.
        Err(error) => {
            let _ = error;
        }
    }
}

/// Test byte flips on each byte of the header individually.
fn test_individual_byte_flips(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let Ok(encoded) = header.encode() else {
        return;
    };

    let flip_count = data
        .first()
        .copied()
        .map_or(1, |first| (first & 0x0F).saturating_add(1));

    for i in 0..flip_count {
        let i_usize = usize::from(i);
        let byte_pos = match data.get(i_usize.saturating_add(1)) {
            Some(&b) => usize::from(b).checked_rem(IPC_HEADER_LEN).unwrap_or(0),
            None => i_usize.checked_rem(IPC_HEADER_LEN).unwrap_or(0),
        };

        let mut flip_seed = vec![0u8; 16];
        if let Some(slot) = flip_seed.first_mut() {
            *slot = u8::try_from(byte_pos).unwrap_or(u8::MAX);
        }

        let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&encoded, &flip_seed);

        if let Ok(corrupted_arr) = <[u8; IPC_HEADER_LEN]>::try_from(corrupted.as_slice()) {
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
