#![forbid(unsafe_code)]
//! Integration tests for vb_ipc.

use vb_core::{DiagnosticCode, RunId, WorkflowDigest};

use crate::{
    BoundedPayload, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IngressFrame, IpcCommand, IpcError,
    IpcFrameHeader, IpcPayload, MaxPayloadBytes, MemoryIngress, QueueCapacity, SubmitRunPayload,
    decode_frame, decode_payload, encode_payload,
};
use bytes::Bytes;

#[cfg(test)]
macro_rules! assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        match &$result {
            Ok(_) => (),
            Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
        }
    }};
}

#[cfg(test)]
macro_rules! prop_assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        let is_ok_result = match &$result {
            Ok(_) => true,
            Err(_) => false,
        };
        prop_assert!(is_ok_result $(, $($arg)+)?);
    }};
}

#[cfg(test)]
macro_rules! prop_assert_err {
    ($result:expr $(, $($arg:tt)+)?) => {{
        let is_err_result = match &$result {
            Ok(_) => false,
            Err(_) => true,
        };
        prop_assert!(is_err_result $(, $($arg)+)?);
    }};
}

include!("tests/tests_ipc_basic.rs");
include!("tests/tests_ipc_errors_frames.rs");
include!("tests/tests_ipc_proptests.rs");
