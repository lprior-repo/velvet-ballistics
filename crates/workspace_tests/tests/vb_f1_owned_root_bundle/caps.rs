use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::SlotIdx;
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

#[test]
fn cap_mismatch_checkpoint_less_than_workflow() -> TestResult<()> {
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        4,
    )?;
    let mut cp = checkpoint(7);
    cp.lists.push(Box::from([SlotValue::Null]));
    let roots = Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);

    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected error")?;

    match err {
        OwnedRootBundleError::CapMismatch {
            checkpoint_cap,
            workflow_max_slots,
        } => {
            assert_eq!(checkpoint_cap, 7);
            assert_eq!(workflow_max_slots, 8);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn cap_mismatch_checkpoint_zero() -> TestResult<()> {
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        4,
    )?;
    let cp = checkpoint(0);
    let roots = Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);

    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected error")?;

    match err {
        OwnedRootBundleError::CapMismatch {
            checkpoint_cap,
            workflow_max_slots,
        } => {
            assert_eq!(checkpoint_cap, 0);
            assert_eq!(workflow_max_slots, 8);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn arena_limit_exceeded_symbols() -> TestResult<()> {
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        4,
    )?;
    let mut cp = checkpoint(8);
    let mut symbols = Vec::new();
    for _ in 0..9 {
        symbols.push(Box::from("s"));
    }
    cp.symbols = symbols;
    let roots = Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);

    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected error")?;

    match err {
        OwnedRootBundleError::ArenaLimit { count, limit } => {
            assert_eq!(count, 9);
            assert_eq!(limit, 8);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}
