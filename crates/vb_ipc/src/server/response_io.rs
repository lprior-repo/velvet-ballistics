#![forbid(unsafe_code)]
//! Response preparation and partial-write ownership, in protocol order.
use super::{IpcResponse, IpcServerError};
use crate::IpcFrameHeader;
use mio::{Registry, Token, net::UnixStream};
use std::io::Write;

pub(super) fn prepare_response(
    buffer: &mut Vec<u8>,
    request: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    let payload = encode_payload(response)?;
    let length = u32::try_from(payload.len()).map_err(|_| IpcServerError::ResponseEncodeFailed)?;
    let header = IpcFrameHeader::new(request.command, 0, request.correlation, length);
    let bytes = encode_header(&header)?;
    let length = bytes
        .len()
        .checked_add(payload.len())
        .ok_or(IpcServerError::ResponseEncodeFailed)?;
    buffer
        .try_reserve(length)
        .map_err(|_| IpcServerError::ResponseEncodeFailed)?;
    buffer.extend_from_slice(&bytes);
    buffer.extend_from_slice(&payload);
    Ok(())
}

fn encode_payload(response: &IpcResponse) -> Result<Vec<u8>, IpcServerError> {
    #[cfg(test)]
    if super::helpers::test_hooks::FORCE_POSTCARD_FAIL.get() {
        return Err(IpcServerError::ResponseEncodeFailed);
    }
    postcard::to_allocvec(response).map_err(|_| IpcServerError::ResponseEncodeFailed)
}

fn encode_header(header: &IpcFrameHeader) -> Result<[u8; crate::IPC_HEADER_LEN], IpcServerError> {
    #[cfg(test)]
    if super::helpers::test_hooks::FORCE_HEADER_ENCODE_FAIL.get() {
        return Err(IpcServerError::ResponseEncodeFailed);
    }
    header
        .encode()
        .map_err(|_| IpcServerError::ResponseEncodeFailed)
}

pub(super) fn write_response_once(
    stream: &mut UnixStream,
    buffer: &mut Vec<u8>,
) -> Result<(), IpcServerError> {
    let written = match stream.write(buffer) {
        Ok(count) => count,
        Err(source)
            if matches!(
                source.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
            ) =>
        {
            0
        }
        Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
    };
    if written > 0 {
        buffer.drain(..written).for_each(|_| ());
    }
    Ok(())
}

pub(super) fn finish_write(
    stream: &mut UnixStream,
    buffer: &[u8],
    registry: &Registry,
    token: Token,
) -> Result<(), IpcServerError> {
    if buffer.is_empty() {
        return flush_stream(stream);
    }
    registry
        .reregister(
            stream,
            token,
            mio::Interest::READABLE | mio::Interest::WRITABLE,
        )
        .map_err(|source| IpcServerError::PollFailed { source })
}

fn flush_stream(stream: &mut UnixStream) -> Result<(), IpcServerError> {
    match flush_result(stream) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(source) => Err(IpcServerError::ResponseWriteFailed { source }),
    }
}

fn flush_result(stream: &mut UnixStream) -> std::io::Result<()> {
    #[cfg(test)]
    if super::helpers::test_hooks::FORCE_FLUSH_FAIL.get() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "flush fail",
        ));
    }
    stream.flush()
}
