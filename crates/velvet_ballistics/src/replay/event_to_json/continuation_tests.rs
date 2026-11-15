#![forbid(unsafe_code)]

#[path = "../../../../workspace_tests/tests/vb_f1_owned_continuation_common/arenas.rs"]
mod arenas;
#[path = "../../../../workspace_tests/tests/vb_f1_owned_continuation_common/receipt.rs"]
mod receipt;
#[path = "continuation_tests/support.rs"]
mod support;

use super::event_to_json;
use support::TestResult;
use vb_runtime::run_continuation::RunContinuationV1;
use vb_storage::{ContinuationCarrierBudget, ContinuationCarrierHeader, EventSeq, JournalEvent};

fn budget() -> TestResult<ContinuationCarrierBudget> {
    Ok(ContinuationCarrierBudget::new(
        1_048_576, 67_108_864, 16_777_216,
    )?)
}

#[test]
fn actual_continuation_carrier_projects_complete_identity_and_opaque_payload() -> TestResult {
    let (state, authority) = support::at_cut()?;
    let original_frame = state.frame.checkpoint_v1();
    let original_store = state.store.checkpoint_v1();
    let captured = RunContinuationV1::capture(&state, &authority)?;
    let component = captured.to_bytes()?;
    let checked = RunContinuationV1::from_bytes(&component, &authority)?;
    assert_eq!(checked.digest(), captured.digest());
    assert_eq!(state.frame.checkpoint_v1(), original_frame);
    assert_eq!(state.store.checkpoint_v1(), original_store);
    let header = ContinuationCarrierHeader::new(
        state.frame.run_id(),
        EventSeq::new(2),
        1,
        EventSeq::new(1),
        captured.digest().as_bytes(),
    )?;
    let record = vb_storage::encode_owned_continuation(header, &component, &mut budget()?)?;
    let key = vb_storage::keys::run_event_key(header.run(), header.own_seq())?;
    let mut operation = budget()?;
    let view = vb_storage::decode_owned_continuation(
        &key,
        header.run(),
        record.as_bytes(),
        &mut operation,
    )?;
    assert_eq!(view.header(), header);
    assert_eq!(view.component(), component);
    let event = view.into_event(&mut operation)?;
    let JournalEvent::RunContinuation { carrier } = &event else {
        return Err("carrier decoder did not produce RunContinuation".into());
    };
    assert_eq!(carrier.component(), component);
    assert_eq!(
        event_to_json(&event),
        serde_json::json!({
            "type": "RunContinuation", "run": 17, "seq": 2, "attempt": 1,
            "carrier": format!("{carrier:?}")
        })
    );
    let restored = checked.restore(authority)?;
    assert_eq!(restored.frame.checkpoint_v1(), original_frame);
    assert_eq!(restored.store.checkpoint_v1(), original_store);
    Ok(())
}
