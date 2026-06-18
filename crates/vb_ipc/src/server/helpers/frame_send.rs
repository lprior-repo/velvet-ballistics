//! Response assembly & transport for IPC frames.
//!
//! Functions for creating error responses, sending responses to clients,
//! and borrowing the workflow resolver.

use std::io::Write;

use super::super::IpcResponse;
use super::super::error::IpcServerError as ServerError;
use crate::IpcError;
use crate::IpcFrameHeader;

pub(crate) use super::super::error::IpcServerError;

/// Creates a frame error response.
pub(crate) fn frame_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::FrameError {
        message: error.to_string(),
    }
}

/// Borrows the workflow resolver from an optional mutable reference.
pub(crate) fn borrow_workflow_resolver<'a>(
    resolver: &'a mut Option<&mut dyn crate::server::WorkflowResolver>,
) -> Option<&'a mut dyn crate::server::WorkflowResolver> {
    match resolver {
        Some(inner) => Some(&mut **inner),
        None => None,
    }
}

/// Sends a response frame to the client.
pub(crate) fn send_response(
    stream: &mut mio::net::UnixStream,
    write_buffer: &mut Vec<u8>,
    registry: &mio::Registry,
    token: mio::Token,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), ServerError> {
    #[cfg(test)]
    let payload_bytes = if super::test_hooks::FORCE_POSTCARD_FAIL.get() {
        Err(postcard::Error::SerializeBufferFull)
    } else {
        postcard::to_allocvec(response)
    }
    .map_err(|_| ServerError::ResponseEncodeFailed)?;

    #[cfg(not(test))]
    let payload_bytes =
        postcard::to_allocvec(response).map_err(|_| ServerError::ResponseEncodeFailed)?;

    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| ServerError::ResponseEncodeFailed)?;

    let header = IpcFrameHeader::new(
        request_header.command,
        0,
        request_header.correlation,
        payload_len,
    );

    #[cfg(test)]
    let header_bytes = if super::test_hooks::FORCE_HEADER_ENCODE_FAIL.get() {
        Err(crate::IpcError::HeaderEncodeFailed)
    } else {
        header.encode()
    }
    .map_err(|_| ServerError::ResponseEncodeFailed)?;

    #[cfg(not(test))]
    let header_bytes = header
        .encode()
        .map_err(|_| ServerError::ResponseEncodeFailed)?;

    write_buffer.clear();
    write_buffer.extend_from_slice(&header_bytes);
    write_buffer.extend_from_slice(&payload_bytes);

    let written = match stream.write(write_buffer) {
        Ok(count) => count,
        Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => 0,
        Err(source) => return Err(ServerError::ResponseWriteFailed { source }),
    };
    if written > 0 {
        write_buffer.drain(..written).for_each(|_| ());
    }
    if write_buffer.is_empty() {
        #[cfg(test)]
        let flush_result = if super::test_hooks::FORCE_FLUSH_FAIL.get() {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "flush fail",
            ))
        } else {
            stream.flush()
        };

        #[cfg(not(test))]
        let flush_result = stream.flush();

        match flush_result {
            Ok(()) => {}
            Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(source) => return Err(ServerError::ResponseWriteFailed { source }),
        }
    } else {
        registry
            .reregister(
                stream,
                token,
                mio::Interest::READABLE | mio::Interest::WRITABLE,
            )
            .map_err(|source| ServerError::PollFailed { source })?;
    }

    Ok(())
}
