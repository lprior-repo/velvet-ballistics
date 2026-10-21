#![forbid(unsafe_code)]
//! IPC server construction and public polling boundary.
use super::{IpcServer, IpcServerError, WorkflowResolver};
use mio::net::UnixListener;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::path::Path;
use vb_runtime::runtime::Runtime;

pub(crate) const SERVER_TOKEN: Token = Token(0);
pub(crate) const MAX_CLIENTS: usize = 256;

impl IpcServer {
    /// Creates a new IPC server bound to the given Unix socket path.
    pub fn bind(socket_path: &Path) -> Result<Self, IpcServerError> {
        if socket_path.exists() {
            std::fs::remove_file(socket_path)
                .map_err(|source| IpcServerError::BindFailed { source })?;
        }

        let mut listener = UnixListener::bind(socket_path)
            .map_err(|source| IpcServerError::BindFailed { source })?;

        let poll = Poll::new().map_err(|source| IpcServerError::PollFailed { source })?;

        poll.registry()
            .register(&mut listener, SERVER_TOKEN, Interest::READABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        let events = Events::with_capacity(MAX_CLIENTS);

        Ok(Self {
            poll,
            listener,
            events,
            clients: HashMap::new(),
            next_token: 1,
            accept_ready: false,
            shutdown_requested: false,
            rejected_clients: 0,
            last_client_error: None,
            #[cfg(test)]
            test_poll_result: None,
        })
    }

    /// Polls one bounded reactor turn; readiness survives a turn budget.
    pub fn poll_once(
        &mut self,
        runtime: &mut Runtime,
        timeout: Option<std::time::Duration>,
    ) -> Result<bool, IpcServerError> {
        self.poll_once_with_resolver(runtime, timeout, None)
    }

    /// Polls one bounded reactor turn with an explicit workflow resolver.
    pub fn poll_once_with_resolver(
        &mut self,
        runtime: &mut Runtime,
        timeout: Option<std::time::Duration>,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<bool, IpcServerError> {
        self.poll_turn(runtime, timeout, resolver)?;
        #[cfg(test)]
        if let Some(result) = self.test_poll_result.take() {
            return result;
        }
        Ok(!self.shutdown_requested || self.has_pending_responses())
    }
}
