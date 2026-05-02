//! IPC client response parsing.
//!
//! Response handling methods are implemented in `client_conn` where `IpcClient`
//! is defined, as they require access to the private `stream` field.

pub use crate::client_conn::recv_response;
