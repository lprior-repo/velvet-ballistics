use crate::{AttemptFailure, HttpLimits, HttpObservation, PinnedGet, ResponseStatus};
use ureq::http::Uri;

mod body;
use body::{check_encoding, declared_length, json_media, read_body};
use ureq::unversioned::resolver::{ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

#[derive(Debug)]
struct PinnedResolver {
    uri: Uri,
    address: std::net::SocketAddr,
}

impl Resolver for PinnedResolver {
    fn resolve(
        &self,
        uri: &Uri,
        _config: &ureq::config::Config,
        _timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        if uri.authority() != self.uri.authority() || uri.scheme() != self.uri.scheme() {
            return Err(ureq::Error::HostNotFound);
        }
        let mut addresses = self.empty();
        addresses
            .try_push(self.address)
            .map_err(|_| ureq::Error::HostNotFound)?;
        Ok(addresses)
    }
}

/// One synchronous GET. A fresh agent has no pooled connection to retry or reuse.
/// Redirects, proxies, compression and ambient DNS are excluded from this profile.
pub fn get_once(
    endpoint: &PinnedGet,
    limits: HttpLimits,
) -> Result<HttpObservation, AttemptFailure> {
    Ok(exchange(endpoint, limits, false)?.0)
}
/// One GET with complete parsed header capture for the explicit bounded profile.
pub fn get_once_with_headers(
    endpoint: &PinnedGet,
    limits: HttpLimits,
) -> Result<crate::HttpObservationWithHeaders, AttemptFailure> {
    let (response, headers) = get_once_for_headers(endpoint, limits)?;
    Ok(crate::HttpObservationWithHeaders::new(response, headers?))
}
/// Header conversion failures stay deferred so Hurl can preserve required-status priority.
pub(crate) fn get_once_for_headers(
    endpoint: &PinnedGet,
    limits: HttpLimits,
) -> Result<
    (
        HttpObservation,
        Result<crate::observation::ObservedResponseHeaders, AttemptFailure>,
    ),
    AttemptFailure,
> {
    let (response, headers) = exchange(endpoint, limits, true)?;
    Ok((response, headers.ok_or(AttemptFailure::Transport)?))
}
fn exchange(
    endpoint: &PinnedGet,
    limits: HttpLimits,
    capture_headers: bool,
) -> Result<
    (
        HttpObservation,
        Option<Result<crate::observation::ObservedResponseHeaders, AttemptFailure>>,
    ),
    AttemptFailure,
> {
    let started = std::time::Instant::now();
    let config = ureq::Agent::config_builder()
        .https_only(endpoint.uri.scheme_str() == Some("https"))
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .max_idle_connections(0)
        .max_idle_connections_per_host(0)
        // ureq retains incomplete response headers in this fixed-size buffer.
        // Fit the validated wire cap plus one byte so overflow remains observable.
        .input_buffer_size(limits.header_bytes.saturating_add(1).max(4096))
        .output_buffer_size(4096)
        .max_response_header_size(limits.header_bytes)
        .timeout_global(Some(limits.deadline()))
        .build();
    let resolver = PinnedResolver {
        uri: endpoint.uri.clone(),
        address: endpoint.address,
    };
    let agent = ureq::Agent::with_parts(config, DefaultConnector::default(), resolver);
    let profile = crate::request_profile::for_media(endpoint.media);
    let mut request = agent
        .get(endpoint.uri.clone())
        .header("accept", profile.accept)
        .header("user-agent", profile.user_agent)
        .header("accept-encoding", "identity")
        .header("connection", "close");
    if let Some(version) = profile.api_version {
        request = request.header("x-github-api-version", version);
    }
    let mut response = request.call().map_err(map_error)?;
    #[cfg(feature = "local-conformance")]
    LOCAL_HEADERS_PARSED.with(|parsed| parsed.set(true));
    check_encoding(response.headers())?;
    let declared = declared_length(response.headers(), limits.body_bytes)?;
    // This opt-in profile cannot observe trailers. Reject before reading a chunked
    // body, rather than consuming discarded trailer data or fabricating absence.
    if capture_headers
        && (response.headers().contains_key("transfer-encoding")
            || response.headers().contains_key("trailer"))
    {
        return Err(AttemptFailure::Transport);
    }

    let json_media = json_media(response.headers());
    let code = response.status().as_u16();
    if !(100..=599).contains(&code) {
        return Err(AttemptFailure::Transport);
    }
    let status = ResponseStatus(code);
    let header_capture = capture_headers
        .then(|| crate::observation::ObservedResponseHeaders::capture(response.headers()));
    let body = read_body(response.body_mut().as_reader(), limits.body_bytes)?;
    if matches!(declared, Some(expected) if expected != body.len()) {
        return Err(AttemptFailure::Transport);
    }
    if started.elapsed() > limits.deadline() {
        return Err(AttemptFailure::Timeout);
    }
    Ok((
        HttpObservation {
            status,
            body,
            json_media,
        },
        header_capture,
    ))
}

fn map_error(error: ureq::Error) -> AttemptFailure {
    match error {
        ureq::Error::Timeout(_) => AttemptFailure::Timeout,
        ureq::Error::LargeResponseHeader(_, _) => AttemptFailure::HeaderLimit,
        _ => AttemptFailure::Transport,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod deadline_tests;

#[cfg(test)]
mod header_tests;

// Opt-in local-conformance feature phase observation. Not a receipt, authority or remote claim.
#[cfg(feature = "local-conformance")]
std::thread_local! {
    static LOCAL_HEADERS_PARSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(feature = "local-conformance")]
struct LocalHeaderProbeReset;

#[cfg(feature = "local-conformance")]
impl Drop for LocalHeaderProbeReset {
    fn drop(&mut self) {
        LOCAL_HEADERS_PARSED.with(|parsed| parsed.set(false));
    }
}

#[cfg(feature = "local-conformance")]
pub(crate) fn local_observed_get_once(
    endpoint: &PinnedGet,
    limits: HttpLimits,
) -> (Result<HttpObservation, AttemptFailure>, bool) {
    LOCAL_HEADERS_PARSED.with(|parsed| parsed.set(false));
    let _reset = LocalHeaderProbeReset;
    let result = get_once(endpoint, limits);
    let parsed = LOCAL_HEADERS_PARSED.with(std::cell::Cell::get);
    (result, parsed)
}

#[cfg(all(test, feature = "local-conformance"))]
#[test]
fn local_header_probe_guard_clears_marker_on_unwind() {
    LOCAL_HEADERS_PARSED.with(|parsed| parsed.set(false));
    let result = std::panic::catch_unwind(|| {
        let _reset = LocalHeaderProbeReset;
        LOCAL_HEADERS_PARSED.with(|parsed| parsed.set(true));
        panic!("intentional probe unwind");
    });
    let parsed = LOCAL_HEADERS_PARSED.with(std::cell::Cell::get);
    assert!(result.is_err());
    assert!(!parsed);
}
