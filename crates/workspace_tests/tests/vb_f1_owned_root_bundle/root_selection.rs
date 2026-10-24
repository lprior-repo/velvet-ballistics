use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::SlotIdx;
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

fn make_roots(indices: &[u16]) -> Vec<(SlotIdx, SlotValue, Taint)> {
    indices
        .iter()
        .map(|&i| (SlotIdx::new(i), SlotValue::I64(7), Taint::Clean))
        .collect()
}

#[test]
fn root_slot_out_of_range() -> TestResult<()> {
    let wf = fixture(
        5,
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

    let roots = make_roots(&[6, 5]);
    let err = OwnedRootBundle::new(roots.into_boxed_slice(), checkpoint(8), &wf)
        .err()
        .ok_or("expected error")?;
    match err {
        OwnedRootBundleError::RootSlotOutOfRange { slot, max } => {
            assert_eq!(slot, SlotIdx::new(6));
            assert_eq!(max, 5);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }

    let roots = make_roots(&[5, 6]);
    let err = OwnedRootBundle::new(roots.into_boxed_slice(), checkpoint(8), &wf)
        .err()
        .ok_or("expected error")?;
    match err {
        OwnedRootBundleError::RootSlotOutOfRange { slot, max } => {
            assert_eq!(slot, SlotIdx::new(5));
            assert_eq!(max, 5);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }

    Ok(())
}

#[test]
fn root_slot_duplicate() -> TestResult<()> {
    let wf = fixture(
        5,
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

    let roots = make_roots(&[1, 1, 0, 0]);
    let err = OwnedRootBundle::new(roots.into_boxed_slice(), checkpoint(8), &wf)
        .err()
        .ok_or("expected error")?;
    match err {
        OwnedRootBundleError::RootSlotDuplicate { slot } => {
            assert_eq!(slot, SlotIdx::new(1));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }

    let roots = make_roots(&[0, 0, 1, 1]);
    let err = OwnedRootBundle::new(roots.into_boxed_slice(), checkpoint(8), &wf)
        .err()
        .ok_or("expected error")?;
    match err {
        OwnedRootBundleError::RootSlotDuplicate { slot } => {
            assert_eq!(slot, SlotIdx::new(0));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }

    Ok(())
}

#[test]
fn root_slot_extra() -> TestResult<()> {
    let wf = fixture(
        5,
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

    let roots = make_roots(&[3, 1]);
    let err = OwnedRootBundle::new(roots.into_boxed_slice(), checkpoint(8), &wf)
        .err()
        .ok_or("expected error")?;
    match err {
        OwnedRootBundleError::RootSlotExtra { slot } => {
            assert_eq!(slot, SlotIdx::new(3));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }

    let roots = make_roots(&[1, 3]);
    let err = OwnedRootBundle::new(roots.into_boxed_slice(), checkpoint(8), &wf)
        .err()
        .ok_or("expected error")?;
    match err {
        OwnedRootBundleError::RootSlotExtra { slot } => {
            assert_eq!(slot, SlotIdx::new(1));
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }

    Ok(())
}
