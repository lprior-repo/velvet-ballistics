use super::*;
use crate::test_support::with_response;
use std::time::Duration;

fn plan(source: &str) -> crate::Result<HurlPlan<'_>> {
    crate::hurl::parse_hurl_with_header_predicates(source)
        .map_err(|error| format!("typed predicate fixture: {error:?}").into())
}
fn bounds() -> Result<HttpLimits, crate::RequestRejection> {
    HttpLimits::new(32, 65536, Duration::from_secs(1))
}

#[test]
fn typed_http_bridge_preserves_duplicates_type_error_and_legacy_mapping() -> crate::Result<()> {
    let plan = plan("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" startsWith \"a\"")?;
    let wire = b"HTTP/1.1 200 OK\r\nX: a\r\nx: b\r\n\r\nbody".to_vec();
    let limits = bounds()?;
    let (typed, capture) = with_response(wire.clone(), false, |endpoint| {
        execute_hurl_with_header_predicates(&plan, &endpoint, limits)
    })?;
    assert!(matches!(
        typed,
        Err(HurlPredicateAttemptFailure::Evaluation(
            HurlPredicateEvalError::HeaderPredicateTypeMismatch { index: 1 }
        ))
    ));
    assert_eq!(capture.accepted, 1);
    let (legacy, capture) = with_response(wire, false, |endpoint| {
        crate::execute_hurl_with_headers(&plan, &endpoint, limits)
    })?;
    assert!(matches!(
        legacy,
        Err(crate::HurlAttemptFailure::Evaluation(
            HurlEvalError::AssertionFailed { index: 1 }
        ))
    ));
    assert_eq!(capture.accepted, 1);
    Ok(())
}

#[test]
fn typed_http_bridge_returns_actual_status_headers_and_opaque_body() -> crate::Result<()> {
    let plan = plan(
        "GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" startsWith \"a\"\nheader \"x\" isString",
    )?;
    let limits = bounds()?;
    let (result, capture) = with_response(
        b"HTTP/1.1 200 OK\r\nX: alpha\r\n\r\n\xff\x00".to_vec(),
        false,
        |endpoint| execute_hurl_with_header_predicates(&plan, &endpoint, limits),
    )?;
    let observed = result?;
    assert_eq!(observed.response().status(), 200);
    assert_eq!(observed.response().body(), &[0xff, 0]);
    assert_eq!(
        observed.headers().collect::<Vec<_>>(),
        vec![crate::ObservedResponseHeader {
            name: "x",
            value: "alpha",
        }]
    );
    assert_eq!(capture.accepted, 1);
    Ok(())
}

#[test]
fn typed_http_bridge_status_precedes_invalid_header_but_matching_status_rejects_it()
-> crate::Result<()> {
    let plan = plan("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" isString")?;
    let limits = bounds()?;
    for status in [201, 200] {
        let mut wire = format!("HTTP/1.1 {status} Status\r\nX: ").into_bytes();
        wire.extend_from_slice(b"\x80\r\n\r\n");
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl_with_header_predicates(&plan, &endpoint, limits)
        })?;
        if status == 201 {
            assert!(matches!(
                result,
                Err(HurlPredicateAttemptFailure::Evaluation(
                    HurlPredicateEvalError::Evaluation(HurlEvalError::RequiredStatus {
                        expected: 200,
                        actual: 201
                    })
                ))
            ));
        } else {
            assert!(matches!(
                result,
                Err(HurlPredicateAttemptFailure::Attempt(
                    AttemptFailure::Transport
                ))
            ));
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}

#[test]
fn typed_http_bridge_body_bound_is_not_hidden_by_predicates() -> crate::Result<()> {
    let plan = plan("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" isString")?;
    let limits = HttpLimits::new(1, 65536, Duration::from_secs(1))?;
    let (result, capture) = with_response(
        b"HTTP/1.1 200 OK\r\nX: a\r\n\r\nab".to_vec(),
        false,
        |endpoint| execute_hurl_with_header_predicates(&plan, &endpoint, limits),
    )?;
    assert!(matches!(
        result,
        Err(HurlPredicateAttemptFailure::Attempt(
            AttemptFailure::BodyLimit
        ))
    ));
    assert_eq!(capture.accepted, 1);
    Ok(())
}

#[test]
fn typed_http_bridge_target_mismatch_precedes_connect() -> crate::Result<()> {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let endpoint = PinnedGet::fixture(listener.local_addr()?)?;
    let plan = plan("GET /other\nHTTP 200\n[Asserts]\nheader \"x\" isString")?;
    assert!(matches!(
        execute_hurl_with_header_predicates(&plan, &endpoint, bounds()?),
        Err(HurlPredicateAttemptFailure::Target)
    ));
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    Ok(())
}

#[test]
fn typed_http_bridge_duplicate_list_and_comma_scalar_are_distinct() -> crate::Result<()> {
    let cases = [
        ("isList", b"X: a\r\nx: b\r\n".as_slice(), vec!["a", "b"]),
        ("isString", b"X: a,b\r\n".as_slice(), vec!["a,b"]),
    ];
    for (predicate, fields, expected_values) in cases {
        let source = format!("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" {predicate}");
        let plan = plan(&source)?;
        let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
        wire.extend_from_slice(fields);
        wire.extend_from_slice(b"\r\nbody");
        let limits = bounds()?;
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl_with_header_predicates(&plan, &endpoint, limits)
        })?;
        let observed = result?;
        assert_eq!(
            observed
                .headers()
                .map(|field| field.value)
                .collect::<Vec<_>>(),
            expected_values
        );
        assert!(observed.headers().all(|field| field.name == "x"));
        assert_eq!(observed.response().body(), b"body");
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}

#[test]
fn typed_http_bridge_header_occurrence_bound_is_exact() -> crate::Result<()> {
    let plan = plan("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" isList")?;
    let limits = bounds()?;
    for count in [64_usize, 65] {
        let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
        for _ in 0..count {
            wire.extend_from_slice(b"X: a\r\n");
        }
        wire.extend_from_slice(b"\r\n");
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl_with_header_predicates(&plan, &endpoint, limits)
        })?;
        if count == 64 {
            let observed = result?;
            assert_eq!(observed.header_count(), 64);
            assert!(
                observed
                    .headers()
                    .all(|field| field.name == "x" && field.value == "a")
            );
            assert!(observed.response().body().is_empty());
        } else {
            assert!(matches!(
                result,
                Err(HurlPredicateAttemptFailure::Attempt(
                    AttemptFailure::HeaderLimit
                ))
            ));
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}

#[test]
fn typed_http_bridge_type_errors_survive_negation_and_retain_later_assertion_index()
-> crate::Result<()> {
    for (operator, fields) in [
        ("startsWith", b"X: a\r\nx: b\r\n".as_slice()),
        ("endsWith", b"X: a\r\nx: b\r\n".as_slice()),
        ("includes", b"X: a\r\n".as_slice()),
    ] {
        for qualifier in ["", "not "] {
            let source = format!(
                "GET /observed\nHTTP 200\n[Asserts]\nstatus == 200\nheader \"x\" {qualifier}{operator} \"a\""
            );
            let plan = plan(&source)?;
            let limits = bounds()?;
            let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
            wire.extend_from_slice(fields);
            wire.extend_from_slice(b"\r\nbody");
            let (typed, capture) = with_response(wire.clone(), false, |endpoint| {
                execute_hurl_with_header_predicates(&plan, &endpoint, limits)
            })?;
            assert!(
                matches!(
                    typed,
                    Err(HurlPredicateAttemptFailure::Evaluation(
                        HurlPredicateEvalError::HeaderPredicateTypeMismatch { index: 2 }
                    ))
                ),
                "{qualifier}{operator}"
            );
            assert_eq!(capture.accepted, 1);
            assert_eq!(
                capture.request.lines().next(),
                Some("GET /observed HTTP/1.1")
            );
            let (legacy, capture) = with_response(wire, false, |endpoint| {
                crate::execute_hurl_with_headers(&plan, &endpoint, limits)
            })?;
            assert!(
                matches!(
                    legacy,
                    Err(crate::HurlAttemptFailure::Evaluation(
                        HurlEvalError::AssertionFailed { index: 2 }
                    ))
                ),
                "{qualifier}{operator}"
            );
            assert_eq!(capture.accepted, 1);
            assert_eq!(
                capture.request.lines().next(),
                Some("GET /observed HTTP/1.1")
            );
        }
    }
    Ok(())
}

#[test]
fn typed_http_bridge_distinguishes_negated_false_or_missing_from_type_errors() -> crate::Result<()>
{
    for (predicate, fields, accepted) in [
        ("not startsWith \"a\"", b"X: b\r\n".as_slice(), true),
        ("not includes \"a\"", b"".as_slice(), true),
        ("includes \"a\"", b"".as_slice(), false),
        ("not contains \"a\"", b"X: a\r\nx: b\r\n".as_slice(), false),
    ] {
        let source =
            format!("GET /observed\nHTTP 200\n[Asserts]\nstatus == 200\nheader \"x\" {predicate}");
        let plan = plan(&source)?;
        let limits = bounds()?;
        let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
        wire.extend_from_slice(fields);
        wire.extend_from_slice(b"\r\nbody");
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl_with_header_predicates(&plan, &endpoint, limits)
        })?;
        if accepted {
            assert_eq!(result?.response().body(), b"body");
        } else {
            assert!(
                matches!(
                    result,
                    Err(HurlPredicateAttemptFailure::Evaluation(
                        HurlPredicateEvalError::Evaluation(HurlEvalError::AssertionFailed {
                            index: 2
                        })
                    ))
                ),
                "{predicate}"
            );
        }
        assert_eq!(capture.accepted, 1);
        assert_eq!(
            capture.request.lines().next(),
            Some("GET /observed HTTP/1.1")
        );
    }
    Ok(())
}
