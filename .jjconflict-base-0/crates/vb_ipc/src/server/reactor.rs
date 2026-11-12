#![forbid(unsafe_code)]
//! Bounded readiness ownership for the synchronous mio shell.
use super::helpers::MagicValidationState;
use super::impl_::{MAX_CLIENTS, SERVER_TOKEN};
use super::{ClientConnection, ClosePolicy, IpcServer, IpcServerError, WorkflowResolver};
use arrayvec::ArrayVec;
use mio::{Interest, Token};
use vb_runtime::runtime::Runtime;

impl IpcServer {
    /// Number of sockets rejected at the configured client admission limit.
    #[must_use]
    pub const fn rejected_clients(&self) -> u64 {
        self.rejected_clients
    }

    /// Whether admitted response bytes still need writable-event service.
    #[must_use]
    pub fn has_pending_responses(&self) -> bool {
        self.clients
            .values()
            .any(|client| !client.write_buffer.is_empty())
    }

    /// Takes the latest bounded peer/cleanup diagnostic.
    pub fn take_client_error(&mut self) -> Option<IpcServerError> {
        self.last_client_error.take()
    }

    pub(super) fn poll_turn(
        &mut self,
        runtime: &mut Runtime,
        timeout: Option<std::time::Duration>,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<(), IpcServerError> {
        let timeout = if self.has_ready_work() {
            Some(std::time::Duration::ZERO)
        } else {
            timeout
        };
        self.poll
            .poll(&mut self.events, timeout)
            .map_err(|source| IpcServerError::PollFailed { source })?;
        self.record_events();
        self.accept_pending()?;
        self.service_clients(runtime, resolver)
    }

    fn has_ready_work(&self) -> bool {
        (!self.shutdown_requested && self.accept_ready)
            || self.clients.values().any(|client| {
                client.write_ready
                    || (!self.shutdown_requested
                        && client.read_ready
                        && client.write_buffer.is_empty())
            })
    }

    fn record_events(&mut self) {
        for event in &self.events {
            if event.token() == SERVER_TOKEN {
                self.accept_ready = true;
            } else if let Some(client) = self.clients.get_mut(&event.token().0) {
                client.read_ready |= event.is_readable() || event.is_read_closed();
                client.write_ready |= event.is_writable() || event.is_write_closed();
            }
        }
    }

    fn accept_pending(&mut self) -> Result<(), IpcServerError> {
        if self.shutdown_requested || !self.accept_ready {
            return Ok(());
        }
        for _ in 0..MAX_CLIENTS {
            match self.listener.accept() {
                Ok((stream, _)) => self.admit_client(stream)?,
                Err(source) if source.kind() == std::io::ErrorKind::WouldBlock => {
                    self.accept_ready = false;
                    break;
                }
                Err(source) if source.kind() == std::io::ErrorKind::Interrupted => {}
                Err(source) => return Err(IpcServerError::AcceptFailed { source }),
            }
        }
        Ok(())
    }

    fn admit_client(&mut self, stream: mio::net::UnixStream) -> Result<(), IpcServerError> {
        if self.clients.len() >= MAX_CLIENTS {
            self.rejected_clients = self.rejected_clients.saturating_add(1);
            drop(stream);
            return Ok(());
        }
        self.register_client(stream)
    }

    #[cfg(test)]
    pub(crate) fn accept_client(&mut self) -> Result<(), IpcServerError> {
        if self.clients.len() >= MAX_CLIENTS {
            return Err(IpcServerError::TooManyClients);
        }
        let (stream, _) = self
            .listener
            .accept()
            .map_err(|source| IpcServerError::AcceptFailed { source })?;
        self.register_client(stream)
    }

    fn register_client(&mut self, mut stream: mio::net::UnixStream) -> Result<(), IpcServerError> {
        let token = Token(self.next_token);
        self.next_token = self
            .next_token
            .checked_add(1)
            .ok_or(IpcServerError::TooManyClients)?;
        self.poll
            .registry()
            .register(&mut stream, token, Interest::READABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;
        drop(self.clients.insert(token.0, new_client(stream)));
        Ok(())
    }

    fn client_tokens(&self) -> Result<ArrayVec<usize, MAX_CLIENTS>, IpcServerError> {
        let mut tokens = ArrayVec::new();
        for token in self.clients.keys().copied() {
            tokens
                .try_push(token)
                .map_err(|_| IpcServerError::TooManyClients)?;
        }
        Ok(tokens)
    }

    fn service_clients(
        &mut self,
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<(), IpcServerError> {
        let tokens = self.client_tokens()?;
        let mut resolver = resolver;
        for token in tokens {
            let resolution = super::borrow_workflow_resolver(&mut resolver);
            let result = self.service_client(token, runtime, resolution);
            self.apply_client_result(token, result)?;
        }
        Ok(())
    }

    fn apply_client_result(
        &mut self,
        token: usize,
        result: Result<bool, IpcServerError>,
    ) -> Result<(), IpcServerError> {
        match result {
            Ok(true) => self.remove_client(token),
            Ok(false) => {}
            Err(
                error @ (IpcServerError::PollFailed { .. }
                | IpcServerError::ResponseEncodeFailed
                | IpcServerError::ResponsePending),
            ) => return Err(error),
            Err(error) => {
                self.last_client_error = Some(error);
                self.remove_client(token);
            }
        }
        Ok(())
    }

    fn service_client(
        &mut self,
        token: usize,
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<bool, IpcServerError> {
        if self.clients.get(&token).is_some_and(|c| c.write_ready) && self.handle_writable(token)? {
            return Ok(true);
        }
        if !self.shutdown_requested
            && self
                .clients
                .get(&token)
                .is_some_and(|c| c.read_ready && c.write_buffer.is_empty())
        {
            return self.handle_readable(token, runtime, resolver);
        }
        Ok(false)
    }

    pub(crate) fn remove_client(&mut self, token: usize) {
        if let Some(mut client) = self.clients.remove(&token)
            && let Err(source) = self.poll.registry().deregister(&mut client.stream)
        {
            self.last_client_error = Some(IpcServerError::PollFailed { source });
        }
    }
}

fn new_client(stream: mio::net::UnixStream) -> ClientConnection {
    ClientConnection {
        stream,
        read_buffer: Vec::new(),
        write_buffer: Vec::new(),
        magic_state: MagicValidationState::AwaitingMagic,
        read_ready: true,
        write_ready: false,
        close_policy: ClosePolicy::KeepAlive,
    }
}
