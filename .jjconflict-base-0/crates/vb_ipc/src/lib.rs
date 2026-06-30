#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::needless_pass_by_value)]
//! Bounded memory ingress and binary IPC for Velvet Ballastics.
//!
//! This crate deliberately exposes memory/IPC-shaped primitives only. HTTP is
//! not part of the hot control plane.

pub mod action_output;
pub mod bounded;
pub mod client;
pub mod codec;
pub mod commands;
pub mod constants;
pub mod error;
pub mod frame;
pub mod frame_types;
pub mod ingress;
pub mod metrics;
pub mod payloads;
pub mod server;

#[cfg(kani)]
pub mod kani_ipc_header;

#[cfg(kani)]
pub mod kani_ipc_header_rejects_oversize;

#[cfg(kani)]
pub mod kani_ipc_decode_order;

#[cfg(kani)]
pub mod kani_flag_validation;

pub use crate::action_output::IpcActionOutputPayload;
pub use crate::bounded::{BoundedPayload, MaxPayloadBytes, QueueCapacity};
pub use crate::codec::{decode_payload, encode_payload};
pub use crate::commands::IpcCommand;
pub use crate::constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
pub use crate::error::IpcError;
#[cfg(test)]
pub(crate) use crate::error::u32_to_usize;
pub use crate::frame::{
    decode_frame_header, decode_frame_payload, encode_frame, read_frame_header,
    read_frame_header_bounded, read_frame_payload, read_frame_payload_bounded,
    validate_frame_bounds, validate_frame_magic, write_frame,
};
pub use crate::frame_types::{IpcFrame, IpcFrameHeader, decode_frame};
pub use crate::ingress::{IngressFrame, MemoryIngress, MemoryIngressSender};
pub use crate::metrics::{
    AggregateMetrics, IpcMetrics, JournalMetrics, RuntimeMetrics, ShardMetrics,
};
pub use crate::payloads::{IpcPayload, IpcTraceEvent, IpcTraceEventKind, SubmitRunPayload};

#[cfg(test)]
mod tests;

// #[cfg(test)]
// mod property_tests;
