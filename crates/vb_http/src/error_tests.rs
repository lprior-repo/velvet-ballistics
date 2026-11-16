use super::{AttemptFailure, RequestRejection};
use crate::{HttpLimits, JsonMediaType, PinnedGet, WorkflowObservationFailure};
use std::{error::Error, net::SocketAddr, time::Duration};

const ATTEMPT_FAILURES: [AttemptFailure; 6] = [
    AttemptFailure::Transport,
    AttemptFailure::Timeout,
    AttemptFailure::HeaderLimit,
    AttemptFailure::BodyLimit,
    AttemptFailure::Encoding,
    AttemptFailure::Resources,
];

#[test]
fn constructors_preserve_distinct_request_rejections() {
    let public_peer = SocketAddr::from(([1, 1, 1, 1], 443));
    assert!(matches!(
        PinnedGet::https("http://api.github.com/", public_peer, JsonMediaType::GitHub),
        Err(RequestRejection::Endpoint)
    ));
    assert!(matches!(
        PinnedGet::https(
            "https://api.github.com/",
            SocketAddr::from(([127, 0, 0, 1], 443)),
            JsonMediaType::GitHub,
        ),
        Err(RequestRejection::Peer)
    ));
    assert!(matches!(
        HttpLimits::new(0, 128, Duration::from_secs(1)),
        Err(RequestRejection::Limits)
    ));
}

fn preserves_boxed_identity<E>(expected: E)
where
    E: Error + Send + Sync + Copy + Eq + 'static,
{
    let boxed: Box<dyn Error + Send + Sync> = Box::new(expected);
    assert!(matches!(boxed.downcast_ref::<E>(), Some(actual) if *actual == expected));
    assert!(boxed.source().is_none());
}

#[test]
fn request_rejections_survive_the_public_boxed_error_boundary() {
    for rejected in [
        RequestRejection::Endpoint,
        RequestRejection::Peer,
        RequestRejection::Limits,
    ] {
        preserves_boxed_identity(rejected);
    }
}

#[test]
fn attempt_failures_survive_the_public_boxed_error_boundary() {
    for failure in ATTEMPT_FAILURES {
        preserves_boxed_identity(failure);
    }
}

#[test]
fn workflow_observation_preserves_every_attempt_failure() {
    for failure in ATTEMPT_FAILURES {
        let observed = WorkflowObservationFailure::from(failure);
        assert!(matches!(
            observed,
            WorkflowObservationFailure::Attempt(actual) if actual == failure
        ));
    }
}
