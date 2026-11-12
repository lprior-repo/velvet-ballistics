use super::support::*;
use vb_runtime::run_continuation::{ContinuationTransportLimits, RunContinuationV1};
use vb_storage::{decode_owned_continuation, encode_owned_continuation};

#[test]
fn both_real_cuts_have_exact_manual_wire_and_strong_inner_restore() -> TestResult {
    for after in [false, true] {
        roundtrip(after)?;
    }
    Ok(())
}

fn roundtrip(after: bool) -> TestResult {
    let (state, authority, component, header) = fixture(after, 3)?;
    let encoded = encode_owned_continuation(header, &component, &mut budget()?)?;
    let payload = expected_payload(header, &component)?;
    assert_eq!(payload.get(..7), Some([43, 1, 17, 2, 1, 1, 1].as_slice()));
    assert_eq!(encoded.as_bytes(), envelope(&payload, 2)?);
    assert_eq!(encoded.as_bytes().get(4..8), Some([1, 0, 43, 0].as_slice()));
    let mut operation = budget()?;
    let view = decode_owned_continuation(
        &key(header)?,
        header.run(),
        encoded.as_bytes(),
        &mut operation,
    )?;
    assert_eq!(view.header(), header);
    assert_eq!(view.component(), component);
    let decoded = decode_inner(view.component(), &authority, &mut operation)?;
    assert_eq!(decoded.digest().as_bytes(), header.component_digest());
    assert_restored(decoded, authority, &state)?;
    assert!(operation.usage().allocation_bytes <= ALLOCATION);
    assert!(operation.usage().work <= WORK);
    assert!(operation.component_allowance().is_err());
    Ok(())
}

fn decode_inner(
    component: &[u8],
    authority: &vb_runtime::run_continuation::ContinuationAuthority,
    operation: &mut vb_storage::ContinuationCarrierBudget,
) -> TestResult<RunContinuationV1> {
    let allowance = operation.component_allowance()?;
    let (allocation, work) = allowance.limits();
    let (decoded, usage) = RunContinuationV1::from_transport_bytes(
        component,
        authority,
        ContinuationTransportLimits::new(component.len(), allocation, work)?,
    )?;
    allowance.absorb(usage)?;
    Ok(decoded)
}

fn assert_restored(
    decoded: RunContinuationV1,
    authority: vb_runtime::run_continuation::ContinuationAuthority,
    state: &vb_runtime::shard::types::RunState,
) -> TestResult {
    let frame = state.frame.checkpoint_v1();
    let store = state.store.checkpoint_v1();
    let workflow = state.workflow.to_parts();
    let restored = decoded.restore(authority)?;
    assert_eq!(restored.frame.checkpoint_v1(), frame);
    assert_eq!(restored.store.checkpoint_v1(), store);
    assert_eq!(restored.workflow.to_parts(), workflow);
    assert_eq!(restored.action_attempts, state.action_attempts);
    assert_eq!(restored.admission, state.admission);
    assert_eq!(restored.action_contracts, state.action_contracts);
    Ok(())
}

#[test]
fn capture_reserves_outer_before_actual_component_capture() -> TestResult {
    let (state, authority, ordinary, header) = fixture(false, 3)?;
    let mut operation = budget()?;
    let allowance = operation.component_allowance()?;
    let (allocation, work) = allowance.limits();
    let bounded = RunContinuationV1::capture_transport(
        &state,
        &authority,
        ContinuationTransportLimits::new(MAX, allocation, work)?,
    )?;
    allowance.absorb(bounded.usage())?;
    assert_eq!(bounded.as_bytes(), ordinary);
    let record = encode_owned_continuation(header, bounded.as_bytes(), &mut operation)?;
    assert_eq!(
        record.as_bytes(),
        envelope(&expected_payload(header, &ordinary)?, 2)?
    );
    Ok(())
}

#[test]
fn valid_outer_integrity_never_substitutes_for_independent_inner_authority() -> TestResult {
    let (_, _, component, header) = fixture(false, 3)?;
    let (_, different_authority, _, _) = fixture(false, 4)?;
    let record = encode_owned_continuation(header, &component, &mut budget()?)?;
    let mut account = budget()?;
    let view =
        decode_owned_continuation(&key(header)?, header.run(), record.as_bytes(), &mut account)?;
    let allowance = account.component_allowance()?;
    let (allocation, work) = allowance.limits();
    let error = RunContinuationV1::from_transport_bytes(
        view.component(),
        &different_authority,
        ContinuationTransportLimits::new(component.len(), allocation, work)?,
    )
    .err()
    .ok_or("independent input identity bypassed")?;
    assert!(matches!(
        error.failure,
        vb_runtime::run_continuation::TransportFailure::Bounds(
            vb_core::value_store::transport::CheckpointTransportError::Malformed(_)
        )
    ));
    assert!(!error.strong_decode_started);
    allowance.absorb(error.usage)?;
    Ok(())
}
