#[path = "kind_refs_fixture.rs"]
mod fixture;

use crate::workflow::validate_node_kind::validate_node_kind;
use crate::workflow::{CompiledNodeKind, WorkflowError, WorkflowParts};

fn check_kind(parts: &WorkflowParts, kind: CompiledNodeKind) -> Result<(), WorkflowError> {
    validate_node_kind(&kind, parts)
}

#[test]
fn three_start_kinds_validate() -> Result<(), Box<dyn std::error::Error>> {
    for kind in fixture::starts() {
        let parts = fixture::parts(kind.clone());
        check_kind(&parts, kind)?;
    }
    Ok(())
}

#[test]
fn lane_end_validates_with_owner() -> Result<(), Box<dyn std::error::Error>> {
    let parts = fixture::parts(fixture::together_start());
    let kind = CompiledNodeKind::ConcurrentLaneEnd {
        owner: crate::ids::StepIdx::new(0),
        result: None,
    };
    check_kind(&parts, kind)?;
    Ok(())
}

#[test]
fn group_join_validates_with_owner() -> Result<(), Box<dyn std::error::Error>> {
    let parts = fixture::parts(fixture::together_start());
    let kind = CompiledNodeKind::ConcurrentGroupJoin {
        owner: crate::ids::StepIdx::new(0),
    };
    check_kind(&parts, kind)?;
    Ok(())
}

#[test]
fn lane_end_rejects_missing_owner() -> Result<(), Box<dyn std::error::Error>> {
    let parts = fixture::parts(fixture::together_start());
    let kind = CompiledNodeKind::ConcurrentLaneEnd {
        owner: crate::ids::StepIdx::new(99),
        result: None,
    };
    let res = check_kind(&parts, kind);
    assert!(
        matches!(res, Err(WorkflowError::StepOutOfBounds { .. })),
        "expected StepOutOfBounds, got {:?}",
        res
    );
    Ok(())
}

#[test]
fn group_join_rejects_missing_owner() -> Result<(), Box<dyn std::error::Error>> {
    let parts = fixture::parts(fixture::together_start());
    let kind = CompiledNodeKind::ConcurrentGroupJoin {
        owner: crate::ids::StepIdx::new(99),
    };
    let res = check_kind(&parts, kind);
    assert!(
        matches!(res, Err(WorkflowError::StepOutOfBounds { .. })),
        "expected StepOutOfBounds, got {:?}",
        res
    );
    Ok(())
}

#[path = "kind_refs_local_tests.rs"]
mod local;

#[path = "kind_refs_envelope_tests.rs"]
mod envelope;

#[path = "kind_refs_owner_tests.rs"]
mod owners;

#[path = "kind_refs_ownership_tests.rs"]
mod ownership;
