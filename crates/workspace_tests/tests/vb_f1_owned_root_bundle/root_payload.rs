use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::SlotIdx;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

#[test]
fn root_payload_varies() -> TestResult<()> {
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        4,
    )?;
    let cp = checkpoint(8);

    let roots_a: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::new([(SlotIdx::new(0), SlotValue::I64(11), Taint::Clean)]);
    let roots_b: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::new([(SlotIdx::new(0), SlotValue::I64(12), Taint::Clean)]);

    let a = OwnedRootBundle::new(roots_a.clone(), cp.clone(), &wf)?;
    let b = OwnedRootBundle::new(roots_b.clone(), cp.clone(), &wf)?;

    assert_eq!(a.roots(), roots_a.as_ref());
    assert_eq!(b.roots(), roots_b.as_ref());
    assert_ne!(a.roots(), b.roots());

    assert_eq!(a.checkpoint(), &cp);
    assert_eq!(b.checkpoint(), &cp);

    assert_eq!(a.digest(), b.digest());
    assert_eq!(a.input_slots(), wf.input_slots());
    assert_eq!(b.input_slots(), wf.input_slots());
    assert_eq!(a.resource_contract(), &wf.resource_contract());
    assert_eq!(b.resource_contract(), &wf.resource_contract());

    assert!(a.matches_workflow(&wf));
    assert!(b.matches_workflow(&wf));

    Ok(())
}
