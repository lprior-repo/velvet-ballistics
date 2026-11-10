use super::support::{Probe, TestResult};
use vb_core::{FiniteF64, SlotValue, Taint};

fn source(first: bool, nested: bool) -> String {
    let mut value = "set: {output: payload, value: '$input.payload'}".to_string();
    if nested {
        value = format!(
            "together:\n                branches:\n                  - label: nested\n                    steps:\n                      - id: payload\n                        {value}"
        );
    }
    let payload = format!(
        "        - label: payload\n          steps:\n            - id: payload_branch\n              {value}\n"
    );
    let other = "        - label: numeric\n          steps:\n            - id: marker\n              set: {output: marker, value: '9'}\n";
    let branches = if first {
        format!("{payload}{other}")
    } else {
        format!("{other}{payload}")
    };
    format!(
        "version: velvet-ballistics/v1\nname: together_results\nwhen: {{manual: {{}}}}\ninputs: {{payload: any}}\nsteps:\n  - id: outer\n    together:\n      branches:\n{branches}  - id: done\n    finish: {{result: '$step.outer'}}\n"
    )
}

fn value(probe: &mut Probe, case: u8) -> TestResult<SlotValue> {
    Ok(match case {
        0 => SlotValue::Null,
        1 => SlotValue::I64(i64::MIN),
        2 => SlotValue::I64(i64::MAX),
        3 => SlotValue::F64(FiniteF64::new(3.5)?),
        4 => SlotValue::List(probe.store.insert_list(Box::from([]))?),
        5 => {
            let inner = probe.store.insert_list_with_taint(
                Box::from([SlotValue::I64(7), SlotValue::F64(FiniteF64::new(2.5)?)]),
                Box::from([Taint::Secret, Taint::DerivedFromSecret]),
            )?;
            SlotValue::List(probe.store.insert_list_with_taint(
                Box::from([SlotValue::List(inner)]),
                Box::from([Taint::Secret]),
            )?)
        }
        _ => return Err("unknown value case".into()),
    })
}

#[test]
fn together_preserves_null_empty_nested_lists_numeric_types_and_taints_in_each_position()
-> TestResult {
    for first in [true, false] {
        for nested in [false, true] {
            for case in 0..6 {
                for taint in [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret] {
                    for quantum in [1, 64] {
                        let mut probe = Probe::new(&source(first, nested))?;
                        let supplied = value(&mut probe, case)?;
                        probe.seed(supplied, taint)?;
                        let saved_input = match supplied {
                            SlotValue::List(id) => Some(probe.store.list(id)?.to_vec()),
                            _ => None,
                        };
                        let result = probe.finish(quantum)?;
                        let values = probe.list(result)?;
                        assert_eq!(values.len(), 2);
                        let position = if first { 0 } else { 1 };
                        let other = if first { 1 } else { 0 };
                        let produced = *values.get(position).ok_or("missing value")?;
                        assert_eq!(values.get(other), Some(&SlotValue::I64(9)));
                        if nested {
                            assert_eq!(probe.list(produced)?, &[supplied]);
                        } else {
                            assert_eq!(produced, supplied);
                        }
                        probe.assert_result_evidence(result, taint)?;
                        if let (SlotValue::List(id), Some(saved)) = (supplied, saved_input) {
                            assert_eq!(probe.store.list(id)?, saved.as_slice());
                        }
                        if case == 5 {
                            let root = probe.list(supplied)?;
                            let child = *root.first().ok_or("missing child")?;
                            let SlotValue::List(child_id) = child else {
                                return Err("missing nested input list".into());
                            };
                            assert_eq!(
                                probe.store.list_item_with_taint(child_id, 0)?,
                                (SlotValue::I64(7), Taint::Secret)
                            );
                            assert_eq!(
                                probe.store.list_item_with_taint(child_id, 1)?,
                                (
                                    SlotValue::F64(FiniteF64::new(2.5)?),
                                    Taint::DerivedFromSecret
                                )
                            );
                        }
                        let SlotValue::List(result_id) = result else {
                            return Err("result is not a list".into());
                        };
                        assert_eq!(
                            probe
                                .store
                                .list_item_with_taint(result_id, u32::try_from(position)?)?,
                            (produced, taint)
                        );
                        if nested {
                            let SlotValue::List(nested_id) = produced else {
                                return Err("nested result is not a list".into());
                            };
                            assert_eq!(
                                probe.store.list_item_with_taint(nested_id, 0)?,
                                (supplied, taint)
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
