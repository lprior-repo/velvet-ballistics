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
pub mod trace;

// Re-export public types for ergonomic API surface and backward compatibility
// with existing submodules (frame.rs, client.rs, server.rs) that import from crate::
pub use action_output::IpcActionOutputPayload;
pub use bounded::{BoundedPayload, MaxPayloadBytes, QueueCapacity};
pub use commands::IpcCommand;
pub use constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
pub use error::IpcError;
pub use frame_types::{decode_frame, IpcFrame, IpcFrameHeader};
pub use ingress::{IngressFrame, MemoryIngress};
pub use metrics::{
    AggregateMetrics, IpcMetrics, JournalMetrics, RuntimeMetrics, ShardMetrics,
};
pub use payloads::{IpcPayload, RunListState, RunSummary, SubmitRunPayload};
pub use trace::{IpcTraceEvent, IpcTraceEventKind};

// Public codec API.
pub use codec::{decode_payload, encode_payload};
