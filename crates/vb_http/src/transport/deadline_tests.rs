use super::*;
use crate::test_support::with_response;
use std::io::{self, Read};
use std::time::Duration;

#[test]
fn withheld_response_times_out_in_both_observation_profiles() -> crate::Result<()> {
    for capture_headers in [false, true] {
        let limits = HttpLimits::new(8, 1024, Duration::from_millis(50))?;
        // Newly specified fixture mode withholds until the bounded operation returns.
        let (result, capture) = with_response(Vec::new(), true, |endpoint| {
            if capture_headers {
                get_once_with_headers(&endpoint, limits).map(|_| ())
            } else {
                get_once(&endpoint, limits).map(|_| ())
            }
        })?;
        assert_eq!(result, Err(AttemptFailure::Timeout));
        assert_eq!(capture.accepted, 1);
        assert_eq!(
            capture.request.lines().next(),
            Some("GET /observed HTTP/1.1")
        );
    }
    Ok(())
}

struct FailedRead(io::ErrorKind);

impl Read for FailedRead {
    fn read(&mut self, _destination: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from(self.0))
    }
}

#[test]
fn bounded_reader_distinguishes_timeout_kinds_from_other_io_failures() {
    assert_eq!(
        body::read_body(io::Cursor::new(b"ok"), 2),
        Ok(b"ok".to_vec())
    );
    for (kind, expected) in [
        (io::ErrorKind::TimedOut, AttemptFailure::Timeout),
        (io::ErrorKind::WouldBlock, AttemptFailure::Timeout),
        (io::ErrorKind::UnexpectedEof, AttemptFailure::Transport),
        (io::ErrorKind::ConnectionReset, AttemptFailure::Transport),
    ] {
        assert_eq!(body::read_body(FailedRead(kind), 2), Err(expected));
    }
}
