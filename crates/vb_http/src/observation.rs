use crate::AttemptFailure;
use ureq::http::HeaderMap;

pub(crate) const MAX_OBSERVED_HEADERS: usize = 64;
pub(crate) const MAX_OBSERVED_HEADER_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseStatus(pub(crate) u16);

impl std::fmt::Display for ResponseStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}

/// Opaque response bytes produced by the transport's bounded body reader.
/// Construction is crate-private; this observation grants no execution authority.
pub struct HttpObservation {
    pub(crate) status: ResponseStatus,
    pub(crate) body: Vec<u8>,
    pub(crate) json_media: bool,
}

impl HttpObservation {
    pub fn status(&self) -> u16 {
        self.status.0
    }

    pub fn response_status(&self) -> ResponseStatus {
        self.status
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn has_json_media_type(&self) -> bool {
        self.json_media
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedResponseHeader<'response> {
    pub name: &'response str,
    pub value: &'response str,
}

pub(crate) struct ObservedResponseHeaders {
    fields: Vec<(String, String)>,
}

impl ObservedResponseHeaders {
    pub(crate) fn capture(headers: &HeaderMap) -> Result<Self, AttemptFailure> {
        validate_capture(headers)?;
        let mut fields = Vec::new();
        fields
            .try_reserve_exact(headers.len())
            .map_err(|_| AttemptFailure::Resources)?;
        for (name, value) in headers.iter().take(MAX_OBSERVED_HEADERS) {
            let text =
                std::str::from_utf8(value.as_bytes()).map_err(|_| AttemptFailure::Transport)?;
            fields.push((copy_field(name.as_str())?, copy_field(text)?));
        }
        Ok(Self { fields })
    }
}

fn validate_capture(headers: &HeaderMap) -> Result<(), AttemptFailure> {
    if headers.len() > MAX_OBSERVED_HEADERS {
        return Err(AttemptFailure::HeaderLimit);
    }
    let mut total = 0_usize;
    for (name, value) in headers.iter().take(MAX_OBSERVED_HEADERS) {
        total = total
            .checked_add(name.as_str().len())
            .and_then(|size| size.checked_add(value.as_bytes().len()))
            .ok_or(AttemptFailure::HeaderLimit)?;
        if total > MAX_OBSERVED_HEADER_BYTES {
            return Err(AttemptFailure::HeaderLimit);
        }
        // The checked aggregate bound also bounds value validation before any copy.
        if !value
            .as_bytes()
            .iter()
            .all(|byte| *byte == b'\t' || (b' '..=b'~').contains(byte))
        {
            return Err(AttemptFailure::Transport);
        }
    }
    Ok(())
}

fn copy_field(text: &str) -> Result<String, AttemptFailure> {
    if text.len() > MAX_OBSERVED_HEADER_BYTES {
        return Err(AttemptFailure::HeaderLimit);
    }
    let mut owned = String::new();
    owned
        .try_reserve_exact(text.len())
        .map_err(|_| AttemptFailure::Resources)?;
    owned.push_str(text);
    Ok(owned)
}

pub struct HttpObservationWithHeaders {
    response: HttpObservation,
    headers: ObservedResponseHeaders,
}

impl HttpObservationWithHeaders {
    pub(crate) fn new(response: HttpObservation, headers: ObservedResponseHeaders) -> Self {
        Self { response, headers }
    }

    pub fn response(&self) -> &HttpObservation {
        &self.response
    }

    pub fn header_count(&self) -> usize {
        self.headers.fields.len()
    }

    /// Parsed map order, with one borrowed view per occurrence; no wire-order claim.
    pub fn headers(&self) -> impl ExactSizeIterator<Item = ObservedResponseHeader<'_>> + '_ {
        self.headers
            .fields
            .iter()
            .map(|(name, value)| ObservedResponseHeader {
                name: name.as_str(),
                value: value.as_str(),
            })
    }
}

#[cfg(test)]
#[path = "observation_tests.rs"]
mod tests;
