use super::{ScalarField, decode_integer_body};
use crate::actions::{
    RepositoryRef, WorkflowCountAction, WorkflowObservationFailure, project_workflow_count,
};
use crate::{
    DecodeError, DecodeLimits, HttpLimits, HttpObservation, JsonMediaType, PinnedGet,
    RequestRejection, ResponseStatus, decode_workflow_count_body,
};
use std::net::{Ipv6Addr, SocketAddr};
use std::time::Duration;

fn decode_limits() -> Result<DecodeLimits, DecodeError> {
    DecodeLimits::new(4096, 16, 512, 1024)
}

#[test]
fn integer_boundaries_reach_public_count_and_real_projection() -> Result<(), DecodeError> {
    let limits = decode_limits()?;
    let cases: &[(&[u8], i64)] = &[
        (br#"{"total_count":0}"#, 0),
        (br#"{"total_count":42}"#, 42),
        (br#"{"total_count":9007199254740993}"#, 9_007_199_254_740_993),
        (br#"{"total_count":9223372036854775807}"#, i64::MAX),
    ];
    for &(body, expected) in cases {
        assert_eq!(
            decode_integer_body(body, limits, ScalarField::WorkflowCount),
            Ok(expected),
        );
        assert_eq!(
            decode_workflow_count_body(body, limits).map(|count| count.get()),
            Ok(expected),
        );
        let observed = HttpObservation {
            status: ResponseStatus(200),
            body: body.to_vec(),
            json_media: true,
        };
        assert_eq!(
            project_workflow_count(&observed, limits).map(|count| count.get()),
            Ok(expected),
        );
    }
    Ok(())
}

#[test]
fn sign_and_integer_overflow_are_typed_range_failures() -> Result<(), DecodeError> {
    let limits = decode_limits()?;
    let cases: &[&[u8]] = &[
        br#"{"total_count":-0}"#,
        br#"{"total_count":-1}"#,
        br#"{"total_count":-9223372036854775808}"#,
        br#"{"total_count":9223372036854775808}"#,
        br#"{"total_count":18446744073709551615}"#,
        br#"{"total_count":18446744073709551616}"#,
        br#"{"total_count":-18446744073709551615}"#,
        br#"{"total_count":-18446744073709551616}"#,
    ];
    for &body in cases {
        assert_eq!(
            decode_workflow_count_body(body, limits),
            Err(DecodeError::WorkflowCountOutOfRange),
        );
        let observed = HttpObservation {
            status: ResponseStatus(200),
            body: body.to_vec(),
            json_media: true,
        };
        assert_eq!(
            project_workflow_count(&observed, limits),
            Err(WorkflowObservationFailure::Decode(
                DecodeError::WorkflowCountOutOfRange,
            )),
        );
    }
    Ok(())
}

#[test]
fn valid_json_nonintegers_keep_type_failure_distinct_from_range() -> Result<(), DecodeError> {
    let limits = decode_limits()?;
    let cases: &[&[u8]] = &[
        br#"{"total_count":null}"#,
        br#"{"total_count":true}"#,
        br#"{"total_count":false}"#,
        br#"{"total_count":"42"}"#,
        br#"{"total_count":""}"#,
        br#"{"total_count":{}}"#,
        br#"{"total_count":[]}"#,
        br#"{"total_count":1.0}"#,
        br#"{"total_count":1e0}"#,
        br#"{"total_count":-0.0}"#,
        br#"{"total_count":-0e0}"#,
        br#"{"total_count":1e999}"#,
    ];
    for &body in cases {
        assert_eq!(
            decode_workflow_count_body(body, limits),
            Err(DecodeError::InvalidWorkflowCount),
        );
        let observed = HttpObservation {
            status: ResponseStatus(200),
            body: body.to_vec(),
            json_media: true,
        };
        assert_eq!(
            project_workflow_count(&observed, limits),
            Err(WorkflowObservationFailure::Decode(DecodeError::InvalidWorkflowCount)),
        );
    }
    Ok(())
}

#[test]
fn only_decoded_exact_root_count_is_selected() -> Result<(), DecodeError> {
    let limits = decode_limits()?;
    let cases: &[(&[u8], Result<i64, DecodeError>)] = &[
        (br#"{"total_\u0063ount":7}"#, Ok(7)),
        (br#"{"total_count":5,"ignored":{"total_count":6},"x":1,"x":2}"#, Ok(5)),
        (br#"{}"#, Err(DecodeError::MissingWorkflowCount)),
        (br#"{"TOTAL_COUNT":3}"#, Err(DecodeError::MissingWorkflowCount)),
        (br#"{"total_count_extra":3}"#, Err(DecodeError::MissingWorkflowCount)),
        (br#"{"total_count ":3}"#, Err(DecodeError::MissingWorkflowCount)),
        (br#"{"payload":{"total_count":9}}"#, Err(DecodeError::MissingWorkflowCount)),
        (br#"{"payload":[{"total_count":9}]}"#, Err(DecodeError::MissingWorkflowCount)),
        (br#"{"repository_id":9}"#, Err(DecodeError::MissingWorkflowCount)),
        (br#"{"total_count":{"total_count":1}}"#, Err(DecodeError::InvalidWorkflowCount)),
    ];
    for &(body, expected) in cases {
        assert_eq!(
            decode_workflow_count_body(body, limits).map(|count| count.get()),
            expected,
        );
        let observed = HttpObservation {
            status: ResponseStatus(200),
            body: body.to_vec(),
            json_media: true,
        };
        assert_eq!(
            project_workflow_count(&observed, limits).map(|count| count.get()),
            expected.map_err(WorkflowObservationFailure::Decode),
        );
    }
    Ok(())
}

#[test]
fn escaped_duplicates_preserve_duplicate_failure_regardless_of_types() -> Result<(), DecodeError> {
    let limits = decode_limits()?;
    let cases: &[&[u8]] = &[
        br#"{"total_count":1,"total_count":2}"#,
        br#"{"total_count":1,"total_\u0063ount":2}"#,
        br#"{"\u0074otal_count":null,"total_count":-0}"#,
        br#"{"total_count":18446744073709551616,"total_count":[]}"#,
    ];
    for &body in cases {
        assert_eq!(
            decode_workflow_count_body(body, limits),
            Err(DecodeError::DuplicateWorkflowCount),
        );
        let observed = HttpObservation {
            status: ResponseStatus(200),
            body: body.to_vec(),
            json_media: true,
        };
        assert_eq!(
            project_workflow_count(&observed, limits),
            Err(WorkflowObservationFailure::Decode(DecodeError::DuplicateWorkflowCount)),
        );
    }
    Ok(())
}

#[test]
fn full_json_validation_precedes_adapter_semantics() -> Result<(), DecodeError> {
    let limits = decode_limits()?;
    let cases: &[&[u8]] = &[
        br#"{"total_count":42,"ignored":[0,]}"#,
        br#"{"total_count":-0,"ignored":"\q"}"#,
        br#"{"total_count":null,"ignored":01}"#,
        br#"{"total_count":1,"total_count":2,"ignored":trueX}"#,
        br#"{"ignored":[false}"#,
        br#"{"total_count":42}{}"#,
        br#"{"total_count":42"#,
        br#"{"total_count":1e}"#,
        br#"{"total_count":+1}"#,
        b"{\"total_count\":42,\"ignored\":\"\xff\"}",
    ];
    for &body in cases {
        assert_eq!(
            decode_workflow_count_body(body, limits),
            Err(DecodeError::InvalidJson),
        );
        let observed = HttpObservation {
            status: ResponseStatus(200),
            body: body.to_vec(),
            json_media: true,
        };
        assert_eq!(
            project_workflow_count(&observed, limits),
            Err(WorkflowObservationFailure::Decode(DecodeError::InvalidJson)),
        );
    }
    Ok(())
}

#[test]
fn resource_errors_propagate_before_selected_field_errors() -> Result<(), DecodeError> {
    let cases: &[(&[u8], DecodeLimits, DecodeError)] = &[
        (br#"{"total_count":42}"#, DecodeLimits::new(17, 16, 512, 1024)?, DecodeError::BodyLimit),
        (br#"{"total_count":-0,"ignored":[0]}"#, DecodeLimits::new(4096, 1, 512, 1024)?, DecodeError::DepthLimit),
        (br#"{"total_count":42}"#, DecodeLimits::new(4096, 16, 2, 1024)?, DecodeError::NodeLimit),
        (br#"{"total_count":null,"ignored":"abcdefghijkl"}"#, DecodeLimits::new(4096, 16, 512, 11)?, DecodeError::StringLimit),
        (br#"{"total_count":1,"total_count":2,"ignored":[]}"#, DecodeLimits::new(4096, 1, 512, 1024)?, DecodeError::DepthLimit),
    ];
    for &(body, limits, error) in cases {
        assert_eq!(decode_workflow_count_body(body, limits), Err(error));
        let observed = HttpObservation {
            status: ResponseStatus(200),
            body: body.to_vec(),
            json_media: true,
        };
        assert_eq!(
            project_workflow_count(&observed, limits),
            Err(WorkflowObservationFailure::Decode(error)),
        );
    }
    Ok(())
}

#[test]
fn exact_minimum_count_budgets_still_allow_projection() -> Result<(), DecodeError> {
    let limits = DecodeLimits::new(18, 1, 3, 11)?;
    let observed = HttpObservation {
        status: ResponseStatus(200),
        body: br#"{"total_count":42}"#.to_vec(),
        json_media: true,
    };
    assert_eq!(
        decode_workflow_count_body(observed.body(), limits).map(|count| count.get()),
        Ok(42),
    );
    assert_eq!(
        project_workflow_count(&observed, limits).map(|count| count.get()),
        Ok(42),
    );
    Ok(())
}

#[test]
fn response_projection_orders_status_media_and_decode() -> Result<(), DecodeError> {
    let tiny_limits = DecodeLimits::new(1, 1, 1, 1)?;
    let cases = [
        (503, false, WorkflowObservationFailure::Status(ResponseStatus(503))),
        (503, true, WorkflowObservationFailure::Status(ResponseStatus(503))),
        (201, true, WorkflowObservationFailure::Status(ResponseStatus(201))),
        (204, true, WorkflowObservationFailure::Status(ResponseStatus(204))),
        (301, true, WorkflowObservationFailure::Status(ResponseStatus(301))),
        (404, true, WorkflowObservationFailure::Status(ResponseStatus(404))),
        (200, false, WorkflowObservationFailure::MediaType),
        (200, true, WorkflowObservationFailure::Decode(DecodeError::BodyLimit)),
    ];
    for (status, json_media, expected) in cases {
        let observed = HttpObservation {
            status: ResponseStatus(status),
            body: b"not JSON".to_vec(),
            json_media,
        };
        assert_eq!(project_workflow_count(&observed, tiny_limits), Err(expected));
    }
    let malformed = HttpObservation {
        status: ResponseStatus(200),
        body: b"not JSON".to_vec(),
        json_media: true,
    };
    assert_eq!(
        project_workflow_count(&malformed, decode_limits()?),
        Err(WorkflowObservationFailure::Decode(DecodeError::InvalidJson)),
    );
    Ok(())
}

#[test]
fn repository_segment_limits_apply_to_both_parts() -> Result<(), RequestRejection> {
    let exact = "a".repeat(100);
    let over = "a".repeat(101);
    for (owner, repository) in [
        ("a", "Z0-_."),
        ("-._", "repo"),
        (exact.as_str(), "repo"),
        ("owner", exact.as_str()),
        (exact.as_str(), exact.as_str()),
    ] {
        RepositoryRef::new(owner, repository)?;
    }
    assert_eq!(RepositoryRef::new(&over, "repo"), Err(RequestRejection::Endpoint));
    assert_eq!(RepositoryRef::new("owner", &over), Err(RequestRejection::Endpoint));
    Ok(())
}

#[test]
fn repository_segments_reject_path_and_non_ascii_injection() {
    for segment in [
        "", ".", "..", "a/b", "a\\b", "a?b", "a#b", "a%2fb", "a:b", "a@b",
        "a b", "a\tb", "a\r\nb", "a\0b", "é", "ａ", "a\u{202e}b",
    ] {
        assert_eq!(RepositoryRef::new(segment, "repo"), Err(RequestRejection::Endpoint));
        assert_eq!(RepositoryRef::new("owner", segment), Err(RequestRejection::Endpoint));
    }
}

#[test]
fn pinned_endpoint_accepts_only_the_fixed_https_authority() -> Result<(), RequestRejection> {
    let peer = SocketAddr::from(([1, 1, 1, 1], 443));
    for uri in [
        "https://api.github.com/observed?x=1",
        "https://api.github.com:443/observed?x=1",
    ] {
        let endpoint = PinnedGet::https(uri, peer, JsonMediaType::GitHub)?;
        assert_eq!(endpoint.uri.to_string(), uri);
        assert_eq!(endpoint.address, peer);
        assert_eq!(endpoint.media, JsonMediaType::GitHub);
    }
    for uri in [
        "http://api.github.com/observed",
        "https://example.com/observed",
        "https://api.github.com.evil.example/observed",
        "https://api.github.com./observed",
        "https://user@api.github.com/observed",
        "https://api.github.com:444/observed",
        "https://api.github.com:80/observed",
        "https://api.github.com/observed#fragment",
        "https://api.github.com/observed#",
        "/observed", "not a URI", "",
    ] {
        assert_eq!(
            PinnedGet::https(uri, peer, JsonMediaType::GitHub).err(),
            Some(RequestRejection::Endpoint),
        );
    }
    Ok(())
}

#[test]
fn pinned_endpoint_uri_byte_limit_is_inclusive() -> Result<(), RequestRejection> {
    let peer = SocketAddr::from(([1, 1, 1, 1], 443));
    // Fixed prefix is 23 ASCII bytes; these are independent input-boundary fixtures.
    let exact = format!("https://api.github.com/{}", "a".repeat(2025));
    let over = format!("https://api.github.com/{}", "a".repeat(2026));
    assert_eq!(exact.len(), 2048);
    assert_eq!(over.len(), 2049);
    PinnedGet::https(&exact, peer, JsonMediaType::GitHub)?;
    assert_eq!(
        PinnedGet::https(&over, peer, JsonMediaType::GitHub).err(),
        Some(RequestRejection::Endpoint),
    );
    Ok(())
}

#[test]
fn pinned_peer_rejects_nonpublic_ipv4_ipv6_and_other_ports() -> Result<(), RequestRejection> {
    let uri = "https://api.github.com/observed";
    for address in [[1, 1, 1, 1], [8, 8, 8, 8]] {
        PinnedGet::https(uri, SocketAddr::from((address, 443)), JsonMediaType::GitHub)?;
    }
    for address in [
        [0, 1, 2, 3], [10, 1, 2, 3], [127, 0, 0, 1], [100, 64, 0, 1],
        [100, 127, 255, 254], [169, 254, 1, 1], [172, 16, 0, 1], [172, 31, 255, 254],
        [192, 0, 2, 1], [192, 168, 1, 1], [192, 88, 99, 1], [198, 18, 0, 1],
        [198, 19, 255, 254], [198, 51, 100, 1], [203, 0, 113, 1],
        [224, 0, 0, 1], [239, 1, 2, 3], [255, 255, 255, 255],
    ] {
        assert_eq!(
            PinnedGet::https(uri, SocketAddr::from((address, 443)), JsonMediaType::GitHub).err(),
            Some(RequestRejection::Peer),
        );
    }
    for address in [Ipv6Addr::LOCALHOST, Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111)] {
        assert_eq!(
            PinnedGet::https(uri, SocketAddr::from((address, 443)), JsonMediaType::GitHub).err(),
            Some(RequestRejection::Peer),
        );
    }
    for port in [0, 80, 442, 444, 65_535] {
        assert_eq!(
            PinnedGet::https(uri, SocketAddr::from(([1, 1, 1, 1], port)), JsonMediaType::GitHub).err(),
            Some(RequestRejection::Peer),
        );
    }
    Ok(())
}

#[test]
fn workflow_action_constructor_uses_real_repository_and_peer_validation() -> Result<(), RequestRejection> {
    let limits = decode_limits().map_err(|_| RequestRejection::Limits)?;
    let http = HttpLimits::new(1024, 2048, Duration::from_secs(1))?;
    let repository = RepositoryRef::new(&"a".repeat(100), &"Z".repeat(100))?;
    WorkflowCountAction::new(
        &repository, SocketAddr::from(([1, 1, 1, 1], 443)), http, limits,
    )?;
    for peer in [
        SocketAddr::from(([127, 0, 0, 1], 443)),
        SocketAddr::from(([10, 0, 0, 1], 443)),
        SocketAddr::from(([1, 1, 1, 1], 80)),
        SocketAddr::from((Ipv6Addr::LOCALHOST, 443)),
    ] {
        assert!(matches!(
            WorkflowCountAction::new(&repository, peer, http, limits),
            Err(RequestRejection::Peer),
        ));
    }
    Ok(())
}
