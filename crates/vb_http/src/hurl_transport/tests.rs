use super::*;
use crate::hurl::{parse_hurl, parse_hurl_with_headers};
use crate::test_support::with_response;
use std::time::Duration;

#[test]
fn status_bridge_evaluates_the_real_exchange_and_retains_opaque_body() -> crate::Result<()> {
    for (source, status, expected) in [
        (
            "GET /observed\nHTTP 200\n[Asserts]\nstatus >= 200\nstatus < 300",
            200,
            None,
        ),
        (
            "GET /observed\nHTTP 200",
            201,
            Some(HurlEvalError::RequiredStatus {
                expected: 200,
                actual: 201,
            }),
        ),
        (
            "GET /observed\nHTTP 200\n[Asserts]\nstatus == 201",
            200,
            Some(HurlEvalError::AssertionFailed { index: 1 }),
        ),
    ] {
        let plan = parse_hurl(source)?;
        let mut wire =
            format!("HTTP/1.1 {status} Status\r\nContent-Length: 2\r\n\r\n").into_bytes();
        wire.extend_from_slice(&[0xff, 0]);
        let limits = HttpLimits::new(2, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl(&plan, &endpoint, limits)
        })?;
        match expected {
            Some(error) => assert!(
                matches!(result, Err(HurlAttemptFailure::Evaluation(actual)) if actual == error)
            ),
            None => {
                let observed = result?;
                assert_eq!(observed.status(), 200);
                assert_eq!(observed.body(), &[0xff, 0]);
            }
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}

#[test]
fn target_and_header_profile_rejections_precede_any_connection() -> crate::Result<()> {
    let limits = HttpLimits::new(8, 4096, Duration::from_secs(1))?;
    let wrong_target = parse_hurl("GET /other\nHTTP 200")?;
    let (result, capture) = with_response(Vec::new(), true, |endpoint| {
        execute_hurl(&wrong_target, &endpoint, limits)
    })?;
    assert!(matches!(result, Err(HurlAttemptFailure::Target)));
    assert_eq!(capture.accepted, 0);
    assert!(capture.request.is_empty());

    let headers =
        parse_hurl_with_headers("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" exists")?;
    let (result, capture) = with_response(Vec::new(), true, |endpoint| {
        execute_hurl(&headers, &endpoint, limits)
    })?;
    assert!(matches!(
        result,
        Err(HurlAttemptFailure::Evaluation(
            HurlEvalError::HeaderObservationRequired
        )),
    ));
    assert_eq!(capture.accepted, 0);
    assert!(capture.request.is_empty());
    Ok(())
}
