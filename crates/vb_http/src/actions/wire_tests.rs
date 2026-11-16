use super::*;
use crate::test_support::with_response;
use std::time::Duration;

fn header_values<'a>(request: &'a str, name: &str) -> Vec<&'a str> {
    request
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
        .collect()
}

#[test]
fn workflow_observation_sends_the_finite_wire_profile_once() -> crate::Result<()> {
    let body = br#"{"total_count":42}"#;
    let mut wire = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len(),
    )
    .into_bytes();
    wire.extend_from_slice(body);
    let http = HttpLimits::new(128, 4096, Duration::from_secs(1))?;
    let decode = DecodeLimits::new(128, 4, 16, 32)?;
    let (result, capture) = with_response(wire, false, |endpoint| {
        WorkflowCountAction::fixture(endpoint, http, decode).observe()
    })?;
    assert_eq!(result?.get(), 42);
    assert_eq!(capture.accepted, 1);
    assert_eq!(capture.request.lines().next(), Some("GET /observed HTTP/1.1"));
    for (name, expected) in [
        ("host", "fixture.invalid"),
        ("accept", "application/vnd.github+json"),
        ("user-agent", "velvet-ballistics/0.1"),
        ("x-github-api-version", "2026-03-10"),
        ("accept-encoding", "identity"),
        ("connection", "close"),
    ] {
        assert_eq!(header_values(&capture.request, name), vec![expected], "{name}");
    }
    assert!(header_values(&capture.request, "authorization").is_empty());
    assert!(header_values(&capture.request, "cookie").is_empty());
    Ok(())
}

#[test]
fn request_digest_binds_each_finite_wire_and_projection_profile_field() -> crate::Result<()> {
    let action = WorkflowCountAction::new(
        &RepositoryRef::new("owner", "repo")?,
        SocketAddr::from(([1, 1, 1, 1], 443)),
        HttpLimits::new(128, 4096, Duration::from_secs(1))?,
        DecodeLimits::new(128, 4, 16, 32)?,
    )?;
    let base = crate::request_profile::GITHUB;
    let digest = action.request_digest()?;
    assert_eq!(digest, action.digest_with_profile(base)?);
    for changed in [
        crate::request_profile::RequestProfile { accept: "application/json", ..base },
        crate::request_profile::RequestProfile { user_agent: "profile-test/1", ..base },
        crate::request_profile::RequestProfile { api_version: Some("2099-01-01"), ..base },
        crate::request_profile::RequestProfile { api_version: None, ..base },
        crate::request_profile::RequestProfile { projection_schema: "workflow-count-v2", ..base },
    ] {
        assert_ne!(digest, action.digest_with_profile(changed)?);
    }
    assert_eq!(digest, action.request_digest()?);
    Ok(())
}
