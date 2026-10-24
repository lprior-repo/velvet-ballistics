use super::support::{TestResult, at_cut};
use vb_core::frame::RunFrameCheckpointV1;
use vb_core::{ListId, RunFrame, SlotValue, Taint, ValueStore};
use vb_runtime::run_continuation::{RunContinuationError, RunContinuationV1};

pub(super) fn change_frame(
    state: &mut vb_runtime::shard::types::RunState,
    change: impl FnOnce(&mut RunFrameCheckpointV1) -> TestResult,
) -> TestResult {
    let mut frame = state.frame.checkpoint_v1();
    change(&mut frame)?;
    state.frame = RunFrame::from_checkpoint_v1(frame).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn original_arena_prefix_cannot_be_substituted_even_when_graph_is_valid() -> TestResult {
    let fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    for arena in 0..5 {
        let mut state = fixture.state.clone();
        let mut store = state.store.checkpoint_v1();
        match arena {
            0 => *store.symbols.first_mut().ok_or("symbol missing")? = "other".into(),
            1 => {
                *store
                    .lists
                    .first_mut()
                    .ok_or("list missing")?
                    .first_mut()
                    .ok_or("item missing")? = SlotValue::I64(8)
            }
            2 => {
                *store
                    .list_taints
                    .first_mut()
                    .ok_or("taints missing")?
                    .first_mut()
                    .ok_or("taint missing")? = Taint::Secret
            }
            3 => {
                store
                    .objects
                    .first_mut()
                    .ok_or("object missing")?
                    .first_mut()
                    .ok_or("field missing")?
                    .value = SlotValue::I64(8)
            }
            4 => {
                *store.blobs.first_mut().ok_or("blob missing")? =
                    bytes::Bytes::from_static(&[4, 1, 3])
            }
            _ => return Err("unexpected arena".into()),
        }
        state.store = ValueStore::from_checkpoint_v1(store)?;
        assert!(matches!(
            RunContinuationV1::capture(&state, &authority),
            Err(RunContinuationError::OwnedPrefix)
        ));
    }
    Ok(())
}

#[test]
fn root_identity_and_dangling_frame_handles_are_checked() -> TestResult {
    let mut fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    let input = fixture.input;
    change_frame(&mut fixture.state, |frame| {
        *frame
            .slots
            .get_mut(usize::from(input.get()))
            .ok_or("input missing")? = Some(SlotValue::List(ListId::new(0)));
        Ok(())
    })?;
    assert!(matches!(
        RunContinuationV1::capture(&fixture.state, &authority),
        Err(RunContinuationError::OwnedRoot)
    ));
    change_frame(&mut fixture.state, |frame| {
        *frame
            .slots
            .get_mut(usize::from(input.get()))
            .ok_or("input missing")? = Some(SlotValue::List(ListId::new(u32::MAX)));
        Ok(())
    })?;
    assert!(matches!(
        RunContinuationV1::capture(&fixture.state, &authority),
        Err(RunContinuationError::RuntimeStore(_))
    ));
    Ok(())
}

#[test]
fn clean_taint_cannot_replace_an_original_secret_root() -> TestResult {
    let mut fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    let input = fixture.input;
    change_frame(&mut fixture.state, |frame| {
        *frame
            .taint
            .get_mut(usize::from(input.get()))
            .ok_or("taint missing")? = Taint::Clean;
        Ok(())
    })?;
    assert!(matches!(
        RunContinuationV1::capture(&fixture.state, &authority),
        Err(RunContinuationError::RuntimeStore(_))
    ));
    Ok(())
}

#[test]
fn enlarged_arena_cap_is_not_runtime_authority() -> TestResult {
    let mut fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    let mut checkpoint = fixture.state.store.checkpoint_v1();
    checkpoint.max_arena_entries = u64::MAX;
    fixture.state.store = ValueStore::from_checkpoint_v1(checkpoint)?;
    assert!(
        matches!(RunContinuationV1::capture(&fixture.state, &authority),
        Err(RunContinuationError::RuntimeStore(error))
        if matches!(*error, vb_core::root_bundle::OwnedRootBundleError::CapMismatch { .. }))
    );

    Ok(())
}

#[test]
fn canonical_counter_bounds_and_complete_attempt_table_are_checked() -> TestResult {
    let fixture = at_cut(false)?;
    let authority = fixture.authority()?;
    let mut state = fixture.state.clone();
    change_frame(&mut state, |frame| {
        frame.executed = u64::MAX;
        Ok(())
    })?;
    assert!(matches!(
        RunContinuationV1::capture(&state, &authority),
        Err(RunContinuationError::CounterBounds)
    ));
    state = fixture.state.clone();
    state.action_attempts = Box::new([]);
    assert!(matches!(
        RunContinuationV1::capture(&state, &authority),
        Err(RunContinuationError::Dimensions)
    ));
    Ok(())
}
