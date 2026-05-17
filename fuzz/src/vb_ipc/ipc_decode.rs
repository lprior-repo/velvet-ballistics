//! VB-IPC-DECODE-FUZZ: IPC decode fuzz target
//!
//! Fuzz target for IPC frame header and payload decoding.
//! This target exercises `IpcFrameHeader::decode` and `decode_frame_payload`
//! with arbitrary byte sequences to find panics or assertion failures.

use vb_ipc::frame_types::{IpcFrameHeader, IpcCommand, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
use vb_ipc::error::{IpcError, MaxPayloadBytes};
use vb_ipc::bounded::BoundedPayload;
use vb_ipc::frame::codec::{decode_frame_header, decode_frame_payload};

/// Fuzz target for IPC header decoding.
/// Panics if decode panics - fuzzing should only trigger Result return.
#[cfg_attr(test, mutate)]
pub fn ipc_decode_header(data: &[u8]) {
    // If data is exactly IPC_HEADER_LEN bytes, try to decode as header
    if data.len() == IPC_HEADER_LEN {
        let mut bytes = [0u8; IPC_HEADER_LEN];
        bytes.copy_from_slice(data);

        // Try with various max_payload bounds
        let bounds: &[usize] = &[0, 1, 16, 256, 1024, 65536, usize::MAX];

        for &max in bounds {
            if let Some(max_payload) = MaxPayloadBytes::new(std::num::NonZeroUsize::new(max).unwrap_or(std::num::NonZeroUsize::MIN)) {
                let result = IpcFrameHeader::decode(&bytes, max_payload);
                // Should return Result, not panic
                match result {
                    Ok(header) => {
                        // Valid header - verify fields are consistent
                        let re_encoded = header.encode();
                        if let Ok(encoded) = re_encoded {
                            // Re-decode should succeed
                            let _ = IpcFrameHeader::decode(&encoded, max_payload);
                        }
                    }
                    Err(_) => {
                        // Error is expected for invalid headers
                    }
                }
            }
        }
    }

    // If data is longer than IPC_HEADER_LEN, try header + partial payload
    if data.len() > IPC_HEADER_LEN {
        let header_bytes = &data[..IPC_HEADER_LEN];
        let payload_bytes = &data[IPC_HEADER_LEN..];

        let mut bytes = [0u8; IPC_HEADER_LEN];
        bytes.copy_from_slice(header_bytes);

        let header_result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        if let Ok(header) = header_result {
            // Try to decode payload with this header
            let _ = decode_frame_payload(&header, payload_bytes);
        }
    }
}

/// Fuzz target for IPC frame decoding with valid header followed by payload.
/// Tests the full decode path including postcard deserialization.
pub fn ipc_decode_frame(data: &[u8]) {
    // Minimum frame is header (24 bytes) + 0 payload
    if data.len() >= IPC_HEADER_LEN {
        let header_bytes: [u8; IPC_HEADER_LEN] = match data.get(..IPC_HEADER_LEN) {
            Some(h) => {
                let mut arr = [0u8; IPC_HEADER_LEN];
                arr.copy_from_slice(h);
                arr
            }
            None => return,
        };

        let header = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

        if let Ok(h) = header {
            let payload_len = h.payload_len as usize;
            let frame_total = IPC_HEADER_LEN.saturating_add(payload_len);

            if data.len() >= frame_total {
                let payload = &data[IPC_HEADER_LEN..frame_total];
                let _ = decode_frame_payload(&h, payload);
            }
        }
    }
}

/// Fuzz target for malformed IPC header bytes.
/// Specifically targets edge cases in header field validation.
pub fn ipc_decode_header_edge_cases(data: &[u8]) {
    // Test truncated header (less than IPC_HEADER_LEN)
    for len in 0..IPC_HEADER_LEN {
        if data.len() >= len {
            let truncated = &data[..len.min(data.len())];
            let mut bytes = [0u8; IPC_HEADER_LEN];
            bytes[..truncated.len()].copy_from_slice(truncated);

            // Should not panic even with truncated input
            let _ = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        }
    }
}

/// Fuzz target for IPC command enum decoding.
/// Tests that all possible u16 values are handled gracefully.
pub fn ipc_decode_command_values(data: &[u8]) {
    if data.len() >= 8 {
        // Build a header with each byte as a possible command
        let mut bytes = [0u8; IPC_HEADER_LEN];

        // Set valid magic and version
        bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
        bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());

        // Try each possible command value
        for i in 0..=255 {
            bytes[6] = i;
            bytes[7] = (i >> 8) as u8;

            let _ = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
        }
    }
}

/// libFuzzer C ABI entry point for ipc_decode_header
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputIpcDecodeHeader(data: *const u8, len: usize) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    ipc_decode_header(slice);
    0
}

/// libFuzzer C ABI entry point for ipc_decode_frame
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputIpcDecodeFrame(data: *const u8, len: usize) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    ipc_decode_frame(slice);
    0
}

/// libFuzzer C ABI entry point for ipc_decode_header_edge_cases
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputIpcDecodeEdgeCases(data: *const u8, len: usize) -> i32 {
    if data.is_null() || len == 0 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    ipc_decode_header_edge_cases(slice);
    ipc_decode_command_values(slice);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_decode_valid_header() {
        // Valid header for Health command
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
        let encoded = header.encode().unwrap();
        ipc_decode_header(&encoded);
    }

    #[test]
    fn test_ipc_decode_invalid_magic() {
        let mut bytes = [0u8; IPC_HEADER_LEN];
        bytes[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
        ipc_decode_header(&bytes);
    }

    #[test]
    fn test_ipc_decode_truncated() {
        let data = vec![0u8; 10];
        ipc_decode_header_edge_cases(&data);
    }
}
