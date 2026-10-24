use vb_core::errors::CoreError;
use vb_core::ids::{BlobId, ListId, ObjectId, SlotIdx, SymbolId};
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError, ValueKind};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn any_root_invalid_references() -> TestResult<()> {
    let values = [
        SlotValue::Symbol(SymbolId::new(99_u32)),
        SlotValue::List(ListId::new(99_u32)),
        SlotValue::Object(ObjectId::new(99_u32)),
        SlotValue::Blob(BlobId::new(99_u64)),
    ];

    let mut checked = 0_u32;
    for value in values {
        let roots = Box::from([(SlotIdx::new(0), value, Taint::Clean)]);
        let cp = checkpoint(8);
        let wf = fixture(
            1,
            vec![CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::Any,
            }],
            8,
            4,
        )?;

        let result = OwnedRootBundle::new(roots, cp, &wf);
        let err = result.err().ok_or("expected error")?;

        match value {
            SlotValue::Symbol(id) => match err {
                OwnedRootBundleError::Checkpoint(CoreError::SymbolOutOfBounds { symbol })
                    if symbol == id => {}
                _ => return Err(format!("unexpected error for symbol: {:?}", err).into()),
            },
            SlotValue::List(id) => match err {
                OwnedRootBundleError::Checkpoint(CoreError::ListOutOfBounds { list })
                    if list == id => {}
                _ => return Err(format!("unexpected error for list: {:?}", err).into()),
            },
            SlotValue::Object(id) => match err {
                OwnedRootBundleError::Checkpoint(CoreError::ObjectOutOfBounds { object })
                    if object == id => {}
                _ => return Err(format!("unexpected error for object: {:?}", err).into()),
            },
            SlotValue::Blob(id) => match err {
                OwnedRootBundleError::Checkpoint(CoreError::BlobOutOfBounds { blob })
                    if blob == id => {}
                _ => return Err(format!("unexpected error for blob: {:?}", err).into()),
            },
            _ => return Err(format!("unexpected value kind: {:?}", value).into()),
        }

        checked = checked.checked_add(1).ok_or("checked overflow")?;
    }

    if checked != 4 {
        return Err(format!("expected 4 checked, got {}", checked).into());
    }

    Ok(())
}

#[test]
fn i64_root_kind_mismatch_before_reference() -> TestResult<()> {
    let roots = Box::from([(
        SlotIdx::new(0),
        SlotValue::Symbol(SymbolId::new(99_u32)),
        Taint::Clean,
    )]);
    let cp = checkpoint(8);
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        4,
    )?;

    let result = OwnedRootBundle::new(roots, cp, &wf);
    let err = result.err().ok_or("expected error")?;

    match err {
        OwnedRootBundleError::RootKindMismatch {
            slot,
            expected,
            actual,
        } if slot == SlotIdx::new(0)
            && expected == InputSlotKind::I64
            && actual == ValueKind::Symbol => {}
        _ => return Err(format!("unexpected error: {:?}", err).into()),
    }

    Ok(())
}
