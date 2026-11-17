use super::*;
use crate::test_support::with_response;
use std::time::Duration;

#[test]
fn captured_headers_preserve_empty_duplicate_comma_values_and_opaque_body() -> crate::Result<()> {
    let mut wire =
        b"HTTP/1.1 200 OK\r\nX: a\r\nx: b\r\nX:\r\nX: a,b\r\nContent-Length: 3\r\n\r\n".to_vec();
    wire.extend_from_slice(&[0xff, 0, 0x80]);
    let limits = HttpLimits::new(3, 4096, Duration::from_secs(1))?;
    let (result, capture) = with_response(wire, false, |endpoint| {
        get_once_with_headers(&endpoint, limits)
    })?;
    let observed = result?;
    assert_eq!(observed.response().status(), 200);
    assert_eq!(observed.response().body(), &[0xff, 0, 0x80]);
    assert_eq!(observed.header_count(), 5);
    for _ in 0..2 {
        let mut values = observed
            .headers()
            .filter(|field| field.name == "x")
            .map(|field| field.value)
            .collect::<Vec<_>>();
        values.sort_unstable();
        assert_eq!(values, ["", "a", "a,b", "b"]);
        assert_eq!(
            observed
                .headers()
                .filter(|field| field.name == "content-length")
                .map(|field| field.value)
                .collect::<Vec<_>>(),
            ["3"],
        );
    }
    assert_eq!(capture.accepted, 1);
    Ok(())
}

#[test]
fn occurrence_and_wire_header_limits_reject_without_truncation() -> crate::Result<()> {
    for count in [64_usize, 65] {
        let mut wire = b"HTTP/1.1 200 OK\r\n".to_vec();
        for _ in 0..count {
            wire.extend_from_slice(b"X: a\r\n");
        }
        wire.extend_from_slice(b"\r\n");
        let limits = HttpLimits::new(8, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire, false, |endpoint| {
            get_once_with_headers(&endpoint, limits)
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
            assert!(matches!(result, Err(AttemptFailure::HeaderLimit)));
        }
        assert_eq!(capture.accepted, 1);
    }
    let wire = format!("HTTP/1.1 200 OK\r\nX: {}\r\n\r\n", "a".repeat(256)).into_bytes();
    for (cap, accepted) in [(1024, true), (64, false)] {
        let limits = HttpLimits::new(8, cap, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire.clone(), false, |endpoint| {
            get_once_with_headers(&endpoint, limits)
        })?;
        if accepted {
            let observed = result?;
            assert_eq!(observed.header_count(), 1);
            assert_eq!(
                observed.headers().next().map(|field| field.value.len()),
                Some(256)
            );
        } else {
            assert!(matches!(result, Err(AttemptFailure::HeaderLimit)));
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}
