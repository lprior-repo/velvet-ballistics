//! IPC client request building.

use crate::client_conn::IpcClient;
use crate::client_error::IpcClientError;
use crate::{IpcCommand, IpcPayload};

/// Sends a typed IPC command through an existing client.
pub fn send_command(
    client: &mut IpcClient,
    command: IpcCommand,
    correlation: u64,
    payload: &IpcPayload,
) -> Result<(), IpcClientError> {
    client.send_command(command, correlation, payload)
}
