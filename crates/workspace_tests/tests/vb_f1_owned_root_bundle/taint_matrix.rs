use vb_core::ids::{ListId, SlotIdx};
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn taint_matrix_all_pairs() -> TestResult<()> {
    let child_rows = [
        (Taint::Clean, [true, true, true, true, true]),
        (Taint::DerivedFromSecret, [false, true, true, true, true]),
        (Taint::Secret, [false, false, true, true, true]),
        (Taint::Random, [false, false, false, true, true]),
        (Taint::TimeDependent, [false, false, false, false, true]),
    ];
    let root_taints = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];

    let mut checked = 0_u64;

    for (child, allowed) in child_rows {
        for (root, should_allow) in root_taints.into_iter().zip(allowed) {
            checked = checked.checked_add(1).ok_or("pair count overflow")?;

            let wf = fixture(
                1,
                vec![CompiledInputSlot {
                    slot: SlotIdx::new(0),
                    kind: InputSlotKind::List,
                }],
                8,
                4,
            )?;

            let mut cp = checkpoint(8);
            cp.lists.push(Box::from([SlotValue::I64(1)]));
            cp.list_taints.push(Box::from([child]));

            let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
                Box::from([(SlotIdx::new(0), SlotValue::List(ListId::new(0)), root)]);

            let result = OwnedRootBundle::new(roots.clone(), cp.clone(), &wf);

            if should_allow {
                let bundle = result?;
                assert!(bundle.matches_workflow(&wf));
                assert_eq!(bundle.roots(), roots.as_ref());
                assert_eq!(bundle.checkpoint(), &cp);
                assert!(bundle.has_root(SlotIdx::new(0)));
                match bundle.root(SlotIdx::new(0)) {
                    Some((value, taint)) => {
                        assert_eq!(value, SlotValue::List(ListId::new(0)));
                        assert_eq!(taint, root);
                    }
                    None => return Err("root should be Some".into()),
                }
            } else {
                let err = result.err().ok_or("expected taint error")?;
                match err {
                    OwnedRootBundleError::TaintMismatch {
                        slot,
                        expected,
                        actual,
                    } => {
                        assert_eq!(slot, SlotIdx::new(0));
                        assert_eq!(expected, child);
                        assert_eq!(actual, root);
                    }
                    other => {
                        return Err(format!("wrong error variant: {:?}", other).into());
                    }
                }
            }
        }
    }

    assert_eq!(checked, 25);
    Ok(())
}
