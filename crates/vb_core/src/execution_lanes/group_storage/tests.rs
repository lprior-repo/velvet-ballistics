use super::*;
use crate::execution_lanes::{LaneToken, PreparedGroupEvent};
use crate::ids::StepIdx;
use crate::value::{SlotValue, Taint};

#[test]
fn empty_foreach_is_terminal_and_retires() -> Result<(), GroupError> {
    let mut storage = GroupStorage::reserve(0, 1)?;
    assert_eq!(storage.active(), None);
    assert_eq!(storage.active_mut().as_deref(), None);

    let id = GroupId::new(StepIdx::new(7), 1).map_err(GroupError::from)?;
    storage.activate(GroupMode::ForEachOrdered, id, 0, 1)?;

    let active = storage.active().ok_or(GroupError::InvalidTransition)?;
    assert_eq!(active.phase(), GroupPhase::Terminal(Termination::Ordered));
    assert_eq!(active.outcome(), GroupOutcome::Ordered(&[]));
    assert_eq!(active.revision(), 0);
    assert_eq!(active.live_count(), 0);

    storage.retire()?;
    assert_eq!(storage.active(), None);
    assert_eq!(storage.active_mut().as_deref(), None);
    Ok(())
}

fn id(generation: u64) -> Result<GroupId, GroupError> {
    GroupId::new(StepIdx::new(7), generation).map_err(GroupError::from)
}

fn control(storage: &mut GroupStorage) -> Result<ActiveGroup<'_>, GroupError> {
    storage.active_mut().ok_or(GroupError::InvalidTransition)
}

/// Caller must assert journal success; no physical durability evidence.
fn commit(prepared: PreparedGroupEvent<'_>) -> Result<GroupTransition, GroupError> {
    let receipt = GroupCommitReceipt::from_committed_event(prepared.event());
    prepared.commit(receipt).map_err(GroupError::from)
}

fn admit(g: &mut ActiveGroup<'_>) -> Result<LaneToken, GroupError> {
    let outcome = commit(g.prepare_admit()?)?;
    match outcome {
        GroupTransition::Admitted(t) => Ok(t),
        _ => Err(GroupError::InvalidTransition),
    }
}

fn complete(
    g: &mut ActiveGroup<'_>,
    t: LaneToken,
    value: SlotValue,
    taint: Taint,
) -> Result<(), GroupError> {
    let outcome = LaneCompletion::Succeeded { value, taint };
    let result = commit(g.prepare_completion(t, outcome)?)?;
    assert_eq!(result, GroupTransition::Changed(g.phase()));
    Ok(())
}

type BufferIdentity = (
    (*const Slot, usize),
    (*const ResultCell, usize),
    (*const LaneRecord, usize),
);

fn buffer_identity(s: &GroupStorage) -> BufferIdentity {
    let (slots, results, lanes) = match &s.state {
        State::Vacant(r) => (&r.slots, &r.results, &r.lanes),
        State::Active(g) => (&g.pool.slots, &g.results, &g.lanes),
    };
    (
        (slots.as_ptr(), slots.capacity()),
        (results.as_ptr(), results.capacity()),
        (lanes.as_ptr(), lanes.capacity()),
    )
}

struct ActivationAttempt {
    mode: GroupMode,
    group: GroupId,
    total_items: u32,
    declared_cap: u16,
}

fn reject_activation(
    storage: &mut GroupStorage,
    attempt: ActivationAttempt,
    error: GroupError,
) -> Result<(), GroupError> {
    let before = format!("{storage:?}");
    let buffers = buffer_identity(storage);
    let result = storage.activate(
        attempt.mode,
        attempt.group,
        attempt.total_items,
        attempt.declared_cap,
    );
    assert_eq!(result, Err(error));
    assert_eq!(format!("{storage:?}"), before);
    assert_eq!(buffer_identity(storage), buffers);
    Ok(())
}

fn finish_three(g: &mut ActiveGroup<'_>) -> Result<LaneToken, GroupError> {
    let t0 = admit(g)?;
    assert_eq!(t0.item_ordinal(), 0);
    let t1 = admit(g)?;
    assert_eq!(t1.item_ordinal(), 1);
    complete(g, t1, SlotValue::I64(20), Taint::Secret)?;
    let t2 = admit(g)?;
    assert_eq!(t2.item_ordinal(), 2);
    complete(g, t0, SlotValue::I64(10), Taint::Clean)?;
    complete(g, t2, SlotValue::I64(30), Taint::DerivedFromSecret)?;
    assert_eq!(g.outcome(), GroupOutcome::Ordered(&ordered_cells()));
    assert_eq!(g.live_count(), 0);
    assert_eq!(g.completed, 3);
    Ok(t0)
}

fn finish_null(g: &mut ActiveGroup<'_>) -> Result<(), GroupError> {
    let id = id(2)?;
    let t = admit(g)?;
    assert_eq!(t.item_ordinal(), 0);
    assert_eq!(t.group(), id);
    complete(g, t, SlotValue::Null, Taint::Clean)?;
    assert_eq!(
        g.outcome(),
        GroupOutcome::Ordered(&[ResultCell::Value {
            value: SlotValue::Null,
            taint: Taint::Clean
        }])
    );
    assert_eq!(g.live_count(), 0);
    Ok(())
}

fn reject_old_token(g: &mut ActiveGroup<'_>, old: LaneToken) -> Result<(), GroupError> {
    let before = GroupControl::clone(g);
    let outcome = LaneCompletion::Succeeded {
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let rejection = g.prepare_completion(old, outcome).err();
    assert_eq!(rejection, Some(GroupError::Lane(LaneError::Foreign)));
    assert_eq!(**g, before);
    Ok(())
}

include!("lifecycle_tests.rs");
include!("policy_tests.rs");
