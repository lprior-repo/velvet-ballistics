//! IPC frame encoding and decoding utilities.
//!
//! Submodules contain types, codec functions, validation, and tests.

pub mod frame_types;
pub mod frame_codec;
pub mod frame_validate;
#[cfg(test)]
mod frame_tests_protocol;
#[cfg(test)]
mod frame_tests_codec;
#[cfg(test)]
mod frame_tests_validate;
#[cfg(test)]
mod frame_tests_adversarial;

// Re-export commonly used items for backward compatibility
pub use frame_codec::{
    decode_frame_header, decode_frame_payload, encode_frame, write_frame,
};
pub use frame_types::{IpcCommand, IpcFrameHeader, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
pub use frame_validate::{
    read_frame_header, read_frame_header_bounded, read_frame_payload,
    read_frame_payload_bounded, validate_frame_bounds, validate_frame_magic,
};

pub use crate::{MaxPayloadBytes, IpcError};
