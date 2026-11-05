//! Byte-flipping mutation testing for IPC frames.
//!
//! Takes valid IPC frames, encodes them, then systematically flips bytes
//! in the header and payload to verify that decoding produces either
//! valid frames or typed errors — never panics.

#![no_main]
use libfuzzer_sys::fuzz_target;
use vb_ipc::{IPC_HEADER_LEN, IpcCommand, IpcError, IpcFrameHeader};

fn make_valid_header(command: IpcCommand, payload_len: u32) -> IpcFrameHeader {
    IpcFrameHeader::new(command, 0, 0, payload_len)
}

fn assert_typed_header_result(result: Result<IpcFrameHeader, IpcError>) {
    match result {
        Ok(_) => {}
        Err(IpcError::InvalidMagic { .. }) => {}
        Err(IpcError::UnsupportedVersion { .. }) => {}
        Err(IpcError::UnknownCommand(_)) => {}
        Err(IpcError::InvalidCommandFlags { .. }) => {}
        Err(IpcError::ReservedBitsSet { .. }) => {}
        Err(IpcError::ReservedNonZero { .. }) => {}
        Err(IpcError::HeaderDecodeFailed) => {}
        Err(IpcError::PayloadLengthOutOfRange { .. }) => {}
        Err(_) => {}
    }
}

fn byte_or_ff(data: &[u8], index: usize) -> u8 {
    match data.get(index) {
        Some(byte) => *byte,
        None => 0xFF,
    }
}

fn flip_at(bytes: &mut [u8], index: usize, xor: u8) {
    if let Some(slot) = bytes.get_mut(index) {
        *slot ^= xor;
    }
}

fn test_header_byte_flip(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let encoded = match header.encode() {
        Ok(e) => e,
        Err(_) => return,
    };

    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&encoded, data);

    if let Ok(corrupted_arr) = <[u8; IPC_HEADER_LEN]>::try_from(corrupted) {
        assert_typed_header_result(vb_ipc::frame::decode_frame_header(&corrupted_arr));
    }
}

fn test_targeted_header_corruption(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let encoded = match header.encode() {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut magic_flip = encoded;
    flip_at(&mut magic_flip, 0, byte_or_ff(data, 0));
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&magic_flip));

    let mut version_flip = encoded;
    flip_at(&mut version_flip, 4, byte_or_ff(data, 1));
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&version_flip));

    let mut command_flip = encoded;
    flip_at(&mut command_flip, 6, byte_or_ff(data, 2));
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&command_flip));

    let mut payload_len_flip = encoded;
    flip_at(&mut payload_len_flip, 8, byte_or_ff(data, 3));
    assert_typed_header_result(vb_ipc::frame::decode_frame_header(&payload_len_flip));
}

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

    let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&frame, data);

    let Some(header_bytes) = corrupted
        .get(..IPC_HEADER_LEN)
        .and_then(|slice| <[u8; IPC_HEADER_LEN]>::try_from(slice).ok())
    else {
        return;
    };

    match vb_ipc::frame::decode_frame_header(&header_bytes) {
        Ok(decoded_header) => {
            if corrupted.len() > IPC_HEADER_LEN
                && let Some(payload_section) = corrupted.get(IPC_HEADER_LEN..)
            {
                match vb_ipc::frame::decode_frame_payload(&decoded_header, payload_section) {
                    Ok(_) | Err(_) => {}
                }
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
        Err(_) => {}
    }
}

fn test_individual_byte_flips(data: &[u8]) {
    let header = make_valid_header(IpcCommand::Health, 0);
    let encoded = match header.encode() {
        Ok(e) => e,
        Err(_) => return,
    };

    let flip_count = match data.first() {
        Some(byte) => (byte & 0x0F).checked_add(1).unwrap_or(1),
        None => 1,
    };

    for i in 0..flip_count {
        let index = usize::from(i);
        let byte_pos = match index.checked_add(1).and_then(|next| data.get(next)) {
            Some(byte) => usize::from(*byte) % IPC_HEADER_LEN,
            None => index % IPC_HEADER_LEN,
        };

        let seed_byte = u8::try_from(byte_pos).unwrap_or(0);
        let mut flip_seed = vec![seed_byte];
        flip_seed.resize(16, 0);

        let corrupted = fuzz_lib::byte_flip_mutation::corrupted_version(&encoded, &flip_seed);

        if let Ok(corrupted_arr) = <[u8; IPC_HEADER_LEN]>::try_from(corrupted) {
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
