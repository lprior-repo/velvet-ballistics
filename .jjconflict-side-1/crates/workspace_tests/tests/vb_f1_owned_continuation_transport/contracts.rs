use super::support::TestResult;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_runtime::run_continuation::{RunContinuationV1, TransportFailure};

pub(super) fn accepted() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: vb_core::ActionId::new(0),
        name: ActionName::new("accepted.transport.contract")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 128,
        max_output_bytes: 256,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

#[test]
fn every_accepted_admission_and_contract_byte_is_checked_before_decode() -> TestResult {
    let fixture = super::action_fixture::at_cut(false)?;
    let authority = fixture.authority()?;
    assert_eq!(fixture.state.action_contracts.len(), 1);
    assert_eq!(fixture.state.action_contracts.first(), Some(&accepted()?));
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    for (name, start) in super::layout::positions(&bytes, &fixture)? {
        let length = match name {
            "admission" => postcard::to_allocvec(
                fixture
                    .state
                    .admission
                    .as_ref()
                    .ok_or("admission missing")?,
            )?
            .len(),
            "action_contracts" => postcard::to_allocvec(&fixture.state.action_contracts)?.len(),
            _ => continue,
        };
        reject_range(&bytes, start, length, &authority)?;
    }
    Ok(())
}

fn reject_range(
    bytes: &[u8],
    start: usize,
    length: usize,
    authority: &vb_runtime::run_continuation::ContinuationAuthority,
) -> TestResult {
    for index in start..start.checked_add(length).ok_or("range overflow")? {
        let mut hostile = bytes.to_vec();
        *hostile.get_mut(index).ok_or("field byte missing")? ^= 128;
        let error = super::golden::reject(&hostile, authority, super::golden::limits(1_048_576)?)?;
        assert!(
            !error.strong_decode_started,
            "decoded changed canonical authority byte"
        );
        assert!(matches!(error.failure, TransportFailure::Bounds(_)));
    }
    Ok(())
}
