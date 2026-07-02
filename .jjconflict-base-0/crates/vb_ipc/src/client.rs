#![forbid(unsafe_code)]
//! IPC client for connecting to a velvet_ballistics runtime.

mod connection;
mod error;

pub use connection::{IpcClient, connect_ipc, recv_response, send_command};
pub use error::IpcClientError;

#[cfg(test)]
mod tests;
