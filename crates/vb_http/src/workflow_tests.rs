use crate::{
    DecodeError, DecodeLimits, HttpLimits, RepositoryRef, WorkflowCount, WorkflowCountAction,
    decode_workflow_count_body,
};
use std::{net::SocketAddr, time::Duration};

fn limits() -> Result<DecodeLimits, DecodeError> {
    DecodeLimits::new(1024, 8, 128, 128)
}

#[test]
fn workflow_count_is_exact_and_accepts_zero() -> Result<(), DecodeError> {
    for value in [0_i64, 42, 9_007_199_254_740_993, i64::MAX] {
        let body = format!("{{\"total_count\":{value},\"workflows\":[]}}");
        assert_eq!(
            decode_workflow_count_body(body.as_bytes(), limits()?)?.get(),
            value
        );
    }
    assert_eq!(
        WorkflowCount::try_from(-1),
        Err(DecodeError::WorkflowCountOutOfRange)
    );
    Ok(())
}

#[test]
fn workflow_count_rejects_duplicates_wrong_types_and_ranges() -> Result<(), DecodeError> {
    assert_eq!(
        decode_workflow_count_body(br#"{"total_count":0,"total_\u0063ount":1}"#, limits()?),
        Err(DecodeError::DuplicateWorkflowCount)
    );
    assert_eq!(
        decode_workflow_count_body(br#"{"id":42}"#, limits()?),
        Err(DecodeError::MissingWorkflowCount)
    );
    for token in ["-0", "-1", "9223372036854775808", "18446744073709551616"] {
        let body = format!("{{\"total_count\":{token}}}");
        assert_eq!(
            decode_workflow_count_body(body.as_bytes(), limits()?),
            Err(DecodeError::WorkflowCountOutOfRange)
        );
    }
    for token in ["null", "true", "1.0", "1e0", "\"1\""] {
        let body = format!("{{\"total_count\":{token}}}");
        assert_eq!(
            decode_workflow_count_body(body.as_bytes(), limits()?),
            Err(DecodeError::InvalidWorkflowCount)
        );
    }
    Ok(())
}

#[test]
fn workflow_count_requires_complete_bounded_body() -> Result<(), DecodeError> {
    for body in [
        br#"{"total_count":42}{}"#.as_slice(),
        br#"{"total_count":42,"workflows":["#,
    ] {
        assert_eq!(
            decode_workflow_count_body(body, limits()?),
            Err(DecodeError::InvalidJson)
        );
    }
    assert_eq!(
        decode_workflow_count_body(br#"{"total_count":42}"#, DecodeLimits::new(32, 2, 2, 16)?),
        Err(DecodeError::NodeLimit)
    );
    Ok(())
}

#[test]
fn workflow_count_digest_binds_destination_and_each_resource_limit() -> crate::Result<()> {
    let digest = |repository: &RepositoryRef, peer, http, decode| {
        WorkflowCountAction::new(repository, peer, http, decode)?.request_digest()
    };
    let repository = RepositoryRef::new("owner", "repo")?;
    let peer = SocketAddr::from(([1, 1, 1, 1], 443));
    let http = HttpLimits::new(1024, 2048, Duration::from_secs(1))?;
    let decode = DecodeLimits::new(1024, 8, 128, 128)?;
    let original = digest(&repository, peer, http, decode)?;
    assert_eq!(original, digest(&repository, peer, http, decode)?);
    for (field, changed) in [
        ("owner", RepositoryRef::new("other-owner", "repo")?),
        ("repository", RepositoryRef::new("owner", "other-repo")?),
    ] {
        assert_ne!(original, digest(&changed, peer, http, decode)?, "{field}");
    }
    assert_ne!(
        original,
        digest(&repository, SocketAddr::from(([1, 1, 1, 2], 443)), http, decode)?,
        "pinned peer"
    );
    for (field, changed) in [
        (
            "HTTP body bytes",
            HttpLimits::new(1025, 2048, Duration::from_secs(1))?,
        ),
        (
            "HTTP header bytes",
            HttpLimits::new(1024, 2049, Duration::from_secs(1))?,
        ),
        (
            "HTTP deadline",
            HttpLimits::new(1024, 2048, Duration::from_secs(2))?,
        ),
    ] {
        assert_ne!(original, digest(&repository, peer, changed, decode)?, "{field}");
    }
    for (field, changed) in [
        ("JSON body bytes", DecodeLimits::new(1025, 8, 128, 128)?),
        ("JSON depth", DecodeLimits::new(1024, 9, 128, 128)?),
        ("JSON nodes", DecodeLimits::new(1024, 8, 129, 128)?),
        ("JSON string bytes", DecodeLimits::new(1024, 8, 128, 129)?),
    ] {
        assert_ne!(original, digest(&repository, peer, http, changed)?, "{field}");
    }
    Ok(())
}
