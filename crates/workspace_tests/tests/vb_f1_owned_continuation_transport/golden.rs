use super::support::{self, Fixture, TestResult};
use vb_core::value_store::transport::CheckpointTransportError as Bounds;
use vb_runtime::run_continuation::{
    ContinuationAuthority, ContinuationTransportError, ContinuationTransportLimits as Limits,
    RunContinuationV1, TransportFailure,
};

pub(super) fn limits(component: usize) -> TestResult<Limits> {
    Ok(Limits::new(component, 67_108_864, 16_777_216)?)
}

pub(super) fn reject(
    bytes: &[u8],
    authority: &ContinuationAuthority,
    limit: Limits,
) -> TestResult<ContinuationTransportError> {
    RunContinuationV1::from_transport_bytes(bytes, authority, limit)
        .err()
        .ok_or_else(|| "hostile transport accepted".into())
}

#[test]
fn both_actual_cuts_match_complete_ordinary_wire_and_restore_exactly() -> TestResult {
    for after in [false, true] {
        assert_cut(after)?;
    }
    Ok(())
}

fn assert_cut(after: bool) -> TestResult {
    let fixture = support::at_cut(after)?;
    let authority = fixture.authority()?;
    let original_parts = fixture.state.workflow.to_parts();
    let original_frame = fixture.state.frame.checkpoint_v1();
    let original_store = fixture.state.store.checkpoint_v1();
    let decoded = compare_codecs(&fixture, &authority)?;
    let restored = decoded.restore(authority)?;
    assert_eq!(restored.frame.checkpoint_v1(), original_frame);
    assert_eq!(restored.store.checkpoint_v1(), original_store);
    assert_eq!(restored.workflow.to_parts(), original_parts);
    assert_eq!(restored.action_attempts, fixture.state.action_attempts);
    assert_eq!(restored.action_contracts, fixture.state.action_contracts);
    assert_eq!(restored.admission, fixture.state.admission);
    assert_eq!(
        restored.registered_execution,
        vb_runtime::shard::types::RegisteredExecutionMode::ReconcileOnly
    );
    assert_eq!(fixture.state.frame.checkpoint_v1(), original_frame);
    assert_eq!(fixture.state.store.checkpoint_v1(), original_store);
    assert_eq!(fixture.state.workflow.to_parts(), original_parts);
    Ok(())
}

fn compare_codecs(
    fixture: &Fixture,
    authority: &ContinuationAuthority,
) -> TestResult<RunContinuationV1> {
    let ordinary = RunContinuationV1::capture(&fixture.state, authority)?;
    let ordinary_bytes = ordinary.to_bytes()?;
    let bounded =
        RunContinuationV1::capture_transport(&fixture.state, authority, limits(1_048_576)?)?;
    assert_eq!(bounded.as_bytes(), ordinary_bytes);
    let (decoded, usage) =
        RunContinuationV1::from_transport_bytes(bounded.as_bytes(), authority, limits(1_048_576)?)?;
    assert_eq!(decoded.digest(), ordinary.digest());
    assert_eq!(decoded.cut(), ordinary.cut());
    assert!(usage.allocation_bytes <= 67_108_864);
    Ok(decoded)
}

#[test]
fn complete_payload_limit_and_one_over_are_exact() -> TestResult {
    let fixture = support::at_cut(false)?;
    let authority = fixture.authority()?;
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    let exact =
        RunContinuationV1::capture_transport(&fixture.state, &authority, limits(bytes.len())?)?;
    assert_eq!(exact.as_bytes(), bytes);
    let smaller = bytes.len().checked_sub(1).ok_or("empty payload")?;
    let error = reject(&bytes, &authority, limits(smaller)?)?;
    assert!(matches!(
        error.failure,
        TransportFailure::Bounds(Bounds::Bound("component_bytes"))
    ));
    assert!(!error.strong_decode_started);
    assert_eq!(error.usage.allocation_bytes, 0);
    assert!(
        RunContinuationV1::capture_transport(&fixture.state, &authority, limits(smaller)?).is_err()
    );
    Ok(())
}

#[test]
fn operation_charge_limit_is_enforced_before_strong_decode() -> TestResult {
    let fixture = support::at_cut(true)?;
    let authority = fixture.authority()?;
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    let (_, usage) =
        RunContinuationV1::from_transport_bytes(&bytes, &authority, limits(bytes.len())?)?;
    let exact = Limits::new(bytes.len(), usage.allocation_bytes, 16_777_216)?;
    RunContinuationV1::from_transport_bytes(&bytes, &authority, exact)?;
    let one_less = usage.allocation_bytes.checked_sub(1).ok_or("zero charge")?;
    let error = reject(
        &bytes,
        &authority,
        Limits::new(bytes.len(), one_less, 16_777_216)?,
    )?;
    assert!(matches!(
        error.failure,
        TransportFailure::Bounds(Bounds::Budget("allocation_bytes"))
    ));
    assert!(!error.strong_decode_started);
    Ok(())
}

#[test]
fn checkpoint_borrowed_view_has_identical_wire_for_every_owned_arena() -> TestResult {
    let fixture = support::at_cut(false)?;
    let borrowed = postcard::to_allocvec(&fixture.state.store.checkpoint_view_v1())?;
    let owned = postcard::to_allocvec(&fixture.state.store.checkpoint_v1())?;
    assert_eq!(borrowed, owned);
    Ok(())
}
