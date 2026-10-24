use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::SlotIdx;
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

#[test]
fn out_of_range_before_duplicate_and_missing() -> TestResult<()> {
    // Given: workflow with input slots 0 and 2, roots include out-of-range slot 3
    let wf = fixture(
        3,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::I64,
            },
        ],
        8,
        4,
    )?;
    let cp = checkpoint(8);
    let roots = Box::from([
        (SlotIdx::new(0), SlotValue::I64(1), Taint::Clean),
        (SlotIdx::new(0), SlotValue::I64(2), Taint::Clean),
        (SlotIdx::new(3), SlotValue::I64(3), Taint::Clean),
    ]);

    // When: constructing root bundle
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected error")?;

    // Then: out-of-range error is reported first
    match err {
        OwnedRootBundleError::RootSlotOutOfRange { slot, max } => {
            assert_eq!(slot, SlotIdx::new(3));
            assert_eq!(max, 3_u16);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn duplicate_before_extra_and_missing() -> TestResult<()> {
    // Given: workflow with input slots 0 and 2, roots have duplicate slot 1
    let wf = fixture(
        3,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::I64,
            },
        ],
        8,
        4,
    )?;
    let cp = checkpoint(8);
    let roots = Box::from([
        (SlotIdx::new(1), SlotValue::I64(1), Taint::Clean),
        (SlotIdx::new(1), SlotValue::I64(2), Taint::Clean),
        (SlotIdx::new(0), SlotValue::I64(3), Taint::Clean),
    ]);

    // When: constructing root bundle
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected error")?;

    // Then: duplicate error is reported before extra/missing
    match err {
        OwnedRootBundleError::RootSlotDuplicate { slot } => {
            assert_eq!(slot, SlotIdx::new(1));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn extra_before_missing() -> TestResult<()> {
    // Given: workflow with input slots 0 and 2, roots have extra slot 1
    let wf = fixture(
        3,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::I64,
            },
        ],
        8,
        4,
    )?;
    let cp = checkpoint(8);
    let roots = Box::from([
        (SlotIdx::new(1), SlotValue::I64(1), Taint::Clean),
        (SlotIdx::new(0), SlotValue::I64(2), Taint::Clean),
    ]);

    // When: constructing root bundle
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected error")?;

    // Then: extra slot error is reported before missing
    match err {
        OwnedRootBundleError::RootSlotExtra { slot } => {
            assert_eq!(slot, SlotIdx::new(1));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn missing_slot_detected() -> TestResult<()> {
    // Given: workflow with input slots 0 and 2, no root values provided
    let wf = fixture(
        3,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::I64,
            },
        ],
        8,
        4,
    )?;
    let cp = checkpoint(8);
    let roots = Box::from([]);

    // When: constructing root bundle
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected error")?;

    // Then: missing slot 0 is reported (first in declaration order)
    match err {
        OwnedRootBundleError::RootSlotMissing { slot } => {
            assert_eq!(slot, SlotIdx::new(0));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}
