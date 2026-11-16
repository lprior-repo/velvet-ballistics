use super::*;
use crate::test_support::with_response;
use std::time::Duration;

#[test]
fn exchange_preserves_opaque_bytes_status_and_explicit_media_rules() -> crate::Result<()> {
    for (status, fields, json) in [
        (201, "", false),
        (201, "Content-Type: application/json\r\n", true),
        (201, "Content-Type: APPLICATION/VND.GITHUB+JSON; charset=\"UTF-8\"\r\n", true),
        (201, "Content-Type: text/json\r\n", false),
        (201, "Content-Type: application/json; charset=latin1\r\n", false),
        (201, "Content-Type: application/json; charset=utf-8; x=y\r\n", false),
        (201, "Content-Type: application/json\r\nContent-Type: application/json\r\n", false),
        (302, "Location: /other\r\nContent-Type: application/json\r\n", true),
    ] {
        let mut wire = format!(
            "HTTP/1.1 {status} Status\r\n{fields}Content-Length: 3\r\n\r\n",
        )
        .into_bytes();
        wire.extend_from_slice(&[0xff, 0, 0x80]);
        let limits = HttpLimits::new(3, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire, false, |endpoint| get_once(&endpoint, limits))?;
        let observed = result?;
        assert_eq!(observed.status(), status);
        assert_eq!(observed.body(), &[0xff, 0, 0x80]);
        assert_eq!(observed.has_json_media_type(), json, "{fields}");
        assert_eq!(capture.accepted, 1);
        assert_eq!(capture.request.lines().next(), Some("GET /observed HTTP/1.1"));
    }
    Ok(())
}

#[test]
fn body_framing_and_encoding_failures_remain_typed() -> crate::Result<()> {
    let cases: &[(&[u8], Option<AttemptFailure>)] = &[
        (b"HTTP/1.1 200 OK\r\n\r\nabcd", None),
        (b"HTTP/1.1 200 OK\r\nContent-Encoding: IDENTITY\r\n\r\nabcd", None),
        (b"HTTP/1.1 200 OK\r\n\r\nabcde", Some(AttemptFailure::BodyLimit)),
        (b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nabcde", Some(AttemptFailure::BodyLimit)),
        (b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nabc", Some(AttemptFailure::Transport)),
        (b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nab", Some(AttemptFailure::Transport)),
        (b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n0\r\n\r\n", Some(AttemptFailure::Transport)),
        (b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 0\r\n\r\n", Some(AttemptFailure::Encoding)),
        (b"HTTP/1.1 200 OK\r\nContent-Encoding: identity\r\nContent-Encoding: gzip\r\n\r\n", Some(AttemptFailure::Encoding)),
        (b"HTTP/1.1 200 OK\r\nContent-Encoding: identity, identity\r\n\r\n", Some(AttemptFailure::Encoding)),
    ];
    for &(wire, expected) in cases {
        let limits = HttpLimits::new(4, 4096, Duration::from_secs(1))?;
        let (result, capture) = with_response(wire.to_vec(), false, |endpoint| {
            get_once(&endpoint, limits)
        })?;
        match expected {
            Some(error) => assert!(matches!(result, Err(actual) if actual == error), "{wire:?}"),
            None => assert_eq!(result?.body(), b"abcd"),
        }
        assert_eq!(capture.accepted, 1);
    }
    Ok(())
}
