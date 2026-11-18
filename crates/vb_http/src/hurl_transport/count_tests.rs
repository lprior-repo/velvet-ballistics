use super::*;
use crate::hurl::{HurlPredicateEvalError, parse_hurl_with_headers};
use crate::test_support::with_response;
use std::time::Duration;

#[test]
fn count_filters_preserve_duplicate_occurrences_and_exact_capture_bound() -> crate::Result<()> {
    for count in [2_usize, 64, 65] {
        let source = format!(
            "GET /observed\nHTTP 200\n[Asserts]\nstatus == 200\nheader \"x\" count == {count}\nheader \"x\" count not < {count}",
        );
        let plan = parse_hurl_with_headers(&source)?;
        let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
        for _ in 0..count {
            wire.extend_from_slice(b"X: a\r\n");
        }
        wire.extend_from_slice(b"\r\n\xff\x00");
        let limits = HttpLimits::new(2, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire, false, |endpoint| {
            execute_hurl_with_headers(&plan, &endpoint, limits)
        })?;
        if count <= 64 {
            let observed = result?;
            assert_eq!(observed.header_count(), count);
            assert!(
                observed
                    .headers()
                    .all(|field| field.name == "x" && field.value == "a")
            );
            assert_eq!(observed.response().body(), &[0xff, 0]);
        } else {
            assert!(matches!(
                result,
                Err(HurlAttemptFailure::Attempt(AttemptFailure::HeaderLimit))
            ));
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}

#[test]
fn count_type_errors_survive_not_and_retain_later_assertion_index() -> crate::Result<()> {
    for fields in [b"".as_slice(), b"X: a\r\n".as_slice()] {
        for qualifier in ["", "not "] {
            let source = format!(
                "GET /observed\nHTTP 200\n[Asserts]\nstatus == 200\nheader \"x\" count {qualifier}== 0",
            );
            let plan = parse_hurl_with_headers(&source)?;
            let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
            wire.extend_from_slice(fields);
            wire.extend_from_slice(b"\r\n\xff\x00");
            let limits = HttpLimits::new(2, 4096, Duration::from_secs(1))?;
            let (legacy, capture) = with_response(wire.clone(), false, |endpoint| {
                execute_hurl_with_headers(&plan, &endpoint, limits)
            })?;
            assert!(matches!(
                legacy,
                Err(HurlAttemptFailure::Evaluation(
                    HurlEvalError::AssertionFailed { index: 2 },
                ))
            ));
            assert_eq!(capture.accepted, 1);
            let (typed, capture) = with_response(wire, false, |endpoint| {
                crate::execute_hurl_with_header_predicates(&plan, &endpoint, limits)
            })?;
            assert!(matches!(
                typed,
                Err(crate::HurlPredicateAttemptFailure::Evaluation(
                    HurlPredicateEvalError::HeaderPredicateTypeMismatch { index: 2 },
                ))
            ));
            assert_eq!(capture.accepted, 1);
        }
    }
    for (qualifier, accepted) in [("", false), ("not ", true)] {
        let source = format!(
            "GET /observed\nHTTP 200\n[Asserts]\nstatus == 200\nheader \"x\" count {qualifier}== 3",
        );
        let plan = parse_hurl_with_headers(&source)?;
        let limits = HttpLimits::new(2, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(
            b"HTTP/1.1 200 OK\r\nX: a\r\nx: b\r\n\r\n\xff\x00".to_vec(),
            false,
            |endpoint| execute_hurl_with_headers(&plan, &endpoint, limits),
        )?;
        if accepted {
            assert_eq!(result?.response().body(), &[0xff, 0]);
        } else {
            assert!(matches!(
                result,
                Err(HurlAttemptFailure::Evaluation(
                    HurlEvalError::AssertionFailed { index: 2 },
                ))
            ));
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}
