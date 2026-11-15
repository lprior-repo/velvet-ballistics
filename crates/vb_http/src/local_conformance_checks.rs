use super::*;

pub(super) const PROFILE_HEADERS: &[(&str, &str)] = &[
    ("host", "fixture.invalid"),
    ("accept", "application/vnd.github+json"),
    ("user-agent", "velvet-ballistics/0.1"),
    ("x-github-api-version", "2026-03-10"),
    ("accept-encoding", "identity"),
    ("connection", "close"),
];

pub(super) fn profile(capture: &Capture) -> Result<(), CaseFailure> {
    let request =
        std::str::from_utf8(&capture.request).map_err(|_| CaseFailure::Check("request UTF-8"))?;
    let framed = request
        .strip_suffix("\r\n\r\n")
        .ok_or(CaseFailure::Check("request framing"))?;
    let mut lines = framed.split("\r\n");
    require(
        lines.next() == Some("GET /observed HTTP/1.1"),
        "method/path",
    )?;
    let mut seen = [false; 6];
    for line in lines {
        let (name, value) = line
            .split_once(": ")
            .ok_or(CaseFailure::Check("header framing"))?;
        let index = PROFILE_HEADERS
            .iter()
            .position(|(expected, _)| name.eq_ignore_ascii_case(expected))
            .ok_or(CaseFailure::Check("unlisted header"))?;
        let previous = seen
            .get_mut(index)
            .ok_or(CaseFailure::Check("header index"))?;
        require(!*previous, "duplicate header")?;
        let (_, expected) = PROFILE_HEADERS
            .get(index)
            .ok_or(CaseFailure::Check("header index"))?;
        require(value == *expected, "header exact value")?;
        *previous = true;
    }
    require(seen.iter().all(|present| *present), "missing header")
}

pub(super) fn digest_separation() -> Result<(), CaseFailure> {
    let endpoint = PinnedGet::fixture(std::net::SocketAddr::from(([127, 0, 0, 1], 4567)))
        .map_err(|_| CaseFailure::Check("digest endpoint"))?;
    let http = limits(1024, 1024)?;
    let decode = decode()?;
    let digest = |endpoint, http, decode| {
        WorkflowCountAction::fixture(endpoint, http, decode)
            .request_digest()
            .map_err(|_| CaseFailure::Check("request digest"))
    };
    let base = digest(endpoint.clone(), http, decode)?;
    require(
        base == digest(endpoint.clone(), http, decode)?,
        "digest stable",
    )?;
    let mut peer = endpoint.clone();
    peer.address.set_port(4568);
    require(base != digest(peer, http, decode)?, "digest peer")?;
    let mut path = endpoint.clone();
    path.uri = "http://fixture.invalid/different"
        .parse()
        .map_err(|_| CaseFailure::Check("digest path"))?;
    require(base != digest(path, http, decode)?, "digest URI")?;
    require(
        base != digest(endpoint.clone(), limits(1023, 1024)?, decode)?,
        "digest body limit",
    )?;
    require(
        base != digest(endpoint.clone(), limits(1024, 1023)?, decode)?,
        "digest header limit",
    )?;
    let shorter = HttpLimits::new(1024, 1024, Duration::from_millis(499))
        .map_err(|_| CaseFailure::Check("digest deadline"))?;
    require(
        base != digest(endpoint.clone(), shorter, decode)?,
        "digest deadline separation",
    )?;
    let decode2 = DecodeLimits::new(1023, 8, 128, 128)
        .map_err(|_| CaseFailure::Check("digest decode limit"))?;
    require(
        base != digest(endpoint, http, decode2)?,
        "digest decode separation",
    )
}

pub(super) fn timed_body_case(
    owner: &mut CustodyOwner,
    timing: BodyTiming,
) -> Result<(), CaseFailure> {
    let http = limits(1024, 1024)?;
    let (observed, capture) = with_owned_body_timing(owner, timing, |endpoint| {
        crate::transport::local_observed_get_once(&endpoint, http)
    })?;
    profile(&capture)?;
    let (result, headers_received) = observed;
    require(
        headers_received,
        "production parsed headers before body outcome",
    )?;
    match timing {
        BodyTiming::Immediate => {
            require(!capture.body_held, "positive body not held")?;
            let response = result.map_err(|_| CaseFailure::Check("body before deadline"))?;
            require(
                response.status() == 200 && response.body() == TIMED_BODY,
                "complete timed positive body",
            )
        }
        BodyTiming::HoldUntilOperationReturns => {
            require(capture.body_held, "body held until operation returned")?;
            require(
                matches!(result, Err(crate::AttemptFailure::Timeout)),
                "exact body deadline failure",
            )
        }
        BodyTiming::FixedResponse => Err(CaseFailure::Check("invalid closed timing case")),
    }
}

pub(super) fn profile_digest_separation() -> Result<(), CaseFailure> {
    let endpoint = PinnedGet::fixture(std::net::SocketAddr::from(([127, 0, 0, 1], 4567)))
        .map_err(|_| CaseFailure::Check("profile digest endpoint"))?;
    let action = WorkflowCountAction::fixture(endpoint, limits(1024, 1024)?, decode()?);
    let checks = action
        .local_profile_digest_checks()
        .map_err(|_| CaseFailure::Check("profile digest computation"))?;
    require(
        checks == [true; 5],
        "profile digest positive and accept/agent/version/schema separation",
    )
}
