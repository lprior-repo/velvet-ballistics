use crate::AttemptFailure;
use std::io::Read;
use ureq::http::HeaderMap;

pub(super) fn check_encoding(headers: &HeaderMap) -> Result<(), AttemptFailure> {
    if headers
        .get_all("content-encoding")
        .iter()
        .any(|value| !value.as_bytes().eq_ignore_ascii_case(b"identity"))
    {
        return Err(AttemptFailure::Encoding);
    }
    Ok(())
}

pub(super) fn json_media(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all("content-type").iter();
    let Some(value) = values.next() else {
        return false;
    };
    if values.next().is_some() {
        return false;
    }
    let Ok(value) = value.to_str() else {
        return false;
    };
    let mut parts = value.split(';');
    let Some(media) = parts.next() else {
        return false;
    };
    let media = media.trim();
    if !media.eq_ignore_ascii_case("application/json")
        && !media.eq_ignore_ascii_case("application/vnd.github+json")
    {
        return false;
    }
    match (parts.next(), parts.next()) {
        (None, None) => true,
        (Some(charset), None) => {
            let Some((name, value)) = charset.trim().split_once('=') else {
                return false;
            };
            let value = value.trim();
            let unquoted = match (value.strip_prefix('"'), value.strip_suffix('"')) {
                (Some(without_first), Some(_)) => without_first.strip_suffix('"'),
                _ => Some(value),
            };
            name.trim().eq_ignore_ascii_case("charset")
                && matches!(unquoted, Some(v) if v.eq_ignore_ascii_case("utf-8"))
        }
        _ => false,
    }
}

pub(super) fn read_body(mut reader: impl Read, limit: usize) -> Result<Vec<u8>, AttemptFailure> {
    let mut body = Vec::new();
    body.try_reserve_exact(limit)
        .map_err(|_| AttemptFailure::Resources)?;
    let mut buffer = [0_u8; 4096];
    for _ in 0..=crate::limits::MAX_BODY_BYTES {
        let remaining = limit
            .checked_sub(body.len())
            .ok_or(AttemptFailure::BodyLimit)?;
        let count = remaining
            .checked_add(1)
            .ok_or(AttemptFailure::BodyLimit)?
            .min(buffer.len());
        let destination = buffer.get_mut(..count).ok_or(AttemptFailure::BodyLimit)?;
        let read = reader.read(destination).map_err(map_io_error)?;
        if read == 0 {
            return Ok(body);
        }
        if read > remaining {
            return Err(AttemptFailure::BodyLimit);
        }
        body.extend_from_slice(destination.get(..read).ok_or(AttemptFailure::Transport)?);
    }
    Err(AttemptFailure::BodyLimit)
}

pub(super) fn declared_length(
    headers: &HeaderMap,
    limit: usize,
) -> Result<Option<usize>, AttemptFailure> {
    let mut values = headers.get_all("content-length").iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() || headers.contains_key("transfer-encoding") {
        return Err(AttemptFailure::Transport);
    }
    let value = value.to_str().map_err(|_| AttemptFailure::Transport)?;
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AttemptFailure::Transport);
    }
    let length = value
        .parse::<usize>()
        .map_err(|_| AttemptFailure::BodyLimit)?;
    if length > limit {
        return Err(AttemptFailure::BodyLimit);
    }
    Ok(Some(length))
}

fn map_io_error(error: std::io::Error) -> AttemptFailure {
    if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        return AttemptFailure::Timeout;
    }
    match error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<ureq::Error>())
    {
        Some(ureq::Error::Timeout(_)) => AttemptFailure::Timeout,
        _ => AttemptFailure::Transport,
    }
}
