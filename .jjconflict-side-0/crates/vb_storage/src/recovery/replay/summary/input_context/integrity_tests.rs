use super::*;
use vb_core::ListId;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn entry(value: SlotValue, taint: Taint) -> RecoveredSlotEntry {
    RecoveredSlotEntry {
        slot: SlotIdx::ZERO,
        value,
        taint,
    }
}

fn assert_entry(entries: &[RecoveredSlotEntry], value: SlotValue, taint: Taint) {
    assert_eq!(
        entries,
        &[RecoveredSlotEntry {
            slot: SlotIdx::ZERO,
            value,
            taint
        }]
    );
}

fn validate_and_take(
    store: ValueStore,
    slots: RecoveredSlots,
) -> TestResult<(RecoveredSlots, ValueStore)> {
    let cell = std::cell::Cell::new(Some(store));
    let context = ReplayInputContext {
        inputs: &[],
        store: None,
        mode: ReplayStoreMode::OwnReplay(&cell),
    };
    let mut validated = slots;
    context.validate_final_slots(&mut validated)?;
    let owned = cell.take().ok_or("recovery store lost")?;
    Ok((validated, owned))
}

#[test]
fn given_missing_list_arena_when_recovery_slots_validated_then_reject_without_mutation()
-> TestResult {
    let store = ValueStore::with_max_slots(8);
    let entries = vec![entry(SlotValue::List(ListId::new(0)), Taint::Secret)];
    let before = store.checkpoint_v1();
    let (validated, after) = validate_and_take(store, RecoveredSlots::supported(entries.clone()))?;
    assert_eq!(
        validated.fully_supported, false,
        "missing list must be unsupported"
    );
    assert_entry(
        &validated.entries,
        SlotValue::List(ListId::new(0)),
        Taint::Secret,
    );
    assert_eq!(after.checkpoint_v1(), before, "store unchanged");
    Ok(())
}

#[test]
fn given_initialized_list_when_recovery_slots_validated_then_preserve_store_and_secret_taint()
-> TestResult {
    let mut store = ValueStore::with_max_slots(8);
    let id = store.insert_list(Box::from([SlotValue::I64(7), SlotValue::Bool(false)]))?;
    let before = store.checkpoint_v1();
    let entries = vec![entry(SlotValue::List(id), Taint::Secret)];
    let (validated, after) = validate_and_take(store, RecoveredSlots::supported(entries.clone()))?;
    assert_eq!(
        validated.fully_supported, true,
        "valid list must be supported"
    );
    assert_eq!(after.checkpoint_v1(), before, "store preserved");
    assert_entry(
        &validated.entries,
        SlotValue::List(ListId::new(0)),
        Taint::Secret,
    );
    assert_eq!(
        after.list(id)?,
        &[SlotValue::I64(7), SlotValue::Bool(false)]
    );
    Ok(())
}

#[test]
fn given_valid_reference_but_unsupported_slots_then_never_rebless() -> TestResult {
    let mut store = ValueStore::with_max_slots(8);
    let id = store.insert_list(Box::from([SlotValue::I64(1)]))?;
    let entries = vec![entry(SlotValue::List(id), Taint::Clean)];
    let before = store.checkpoint_v1();
    let mut bad = RecoveredSlots::unsupported();
    bad.entries = entries.clone();
    let (validated, after) = validate_and_take(store, bad)?;
    assert_eq!(
        validated.fully_supported, false,
        "already-unsupported never reblessed"
    );
    assert_entry(
        &validated.entries,
        SlotValue::List(ListId::new(0)),
        Taint::Clean,
    );
    assert_eq!(after.checkpoint_v1(), before);
    Ok(())
}

#[test]
fn given_missing_list_when_reference_validated_then_exact_core_error() -> TestResult {
    let store = ValueStore::with_max_slots(8);
    assert_eq!(
        validate_reference(&store, SlotValue::List(ListId::new(0))),
        Err(CoreError::ListOutOfBounds {
            list: ListId::new(0)
        })
    );
    Ok(())
}
