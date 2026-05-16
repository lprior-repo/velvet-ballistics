#![forbid(unsafe_code)]
//! IPC frame encoding and decoding utilities.
//!
//! Submodules contain types, codec functions, validation, and I/O operations.

pub mod codec;
pub mod validate;
pub mod io;

// Re-export commonly used items for convenience
pub use codec::{decode_frame_header, decode_frame_payload, encode_frame};
pub use io::{
    read_frame_header, read_frame_header_bounded, read_frame_payload, read_frame_payload_bounded,
    write_frame,
};
pub use validate::{validate_frame_bounds, validate_frame_magic};