use super::*;
use crate::hurl::parse_hurl_with_headers;
use crate::test_support::with_response;
use std::time::Duration;

#[test]
fn required_status_precedes_deferred_header_conversion_failure() -> crate::Result<()> {
    let plan = parse_hurl_with_headers("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" exists")?;
    for status in [201, 200] {
        let mut wire = format!("HTTP/1.1 {status} Status\r\nX: ").into_bytes();
        wire.extend_from_slice(b"\x80\r\nContent-Length: 2\r\n\r\n\xff\x00");
        let limits = HttpLimits::new(2, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl_with_headers(&plan, &endpoint, limits)
        })?;
        if status == 201 {
            assert!(matches!(
                result,
                Err(HurlAttemptFailure::Evaluation(
                    HurlEvalError::RequiredStatus {
                        expected: 200,
                        actual: 201,
                    }
                )),
            ));
        } else {
            assert!(matches!(
                result,
                Err(HurlAttemptFailure::Attempt(AttemptFailure::Transport))
            ));
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}

#[test]
fn header_bridge_keeps_scalar_comma_duplicate_and_empty_semantics() -> crate::Result<()> {
    let cases: &[(&str, &[u8], &[&str], bool)] = &[
        ("== \"a,b\"", b"X: a,b\r\n", &["a,b"], true),
        ("== \"a,b\"", b"X: a\r\nx: b\r\n", &["a", "b"], false),
        ("!= \"a\"", b"X: a\r\nx: b\r\n", &["a", "b"], true),
        ("isEmpty", b"X:\r\n", &[""], true),
        ("not exists", b"", &[], true),
    ];
    for &(predicate, fields, expected_values, accepted) in cases {
        let source = format!("GET /observed\nHTTP 200\n[Asserts]\nheader \"x\" {predicate}");
        let plan = parse_hurl_with_headers(&source)?;
        let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
        wire.extend_from_slice(fields);
        wire.extend_from_slice(b"Content-Length: 2\r\n\r\n\xff\x00");
        let limits = HttpLimits::new(2, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl_with_headers(&plan, &endpoint, limits)
        })?;
        if accepted {
            let observed = result?;
            assert_eq!(observed.response().body(), &[0xff, 0]);
            let mut values = observed
                .headers()
                .filter(|field| field.name == "x")
                .map(|field| field.value)
                .collect::<Vec<_>>();
            values.sort_unstable();
            assert_eq!(values, expected_values);
        } else {
            assert!(matches!(
                result,
                Err(HurlAttemptFailure::Evaluation(
                    HurlEvalError::AssertionFailed { index: 1 },
                ))
            ));
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}
