use vb_core::{ids, workflow};

#[path = "budget_focused_visit/fixtures.rs"]
mod fixtures;
#[path = "../src/verification/kani/kani_workflow_budget_focused_visit.rs"]
mod focused_visit;
#[path = "budget_focused_visit/legacy.rs"]
mod legacy;

use ids::StepIdx;
use workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

type Observation = (Vec<CompiledNode>, StepIdx, ResourceContract);

fn observe(
    original: bool,
    nodes: &[CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Vec<Observation> {
    let mut observations = Vec::new();
    let visit = |nodes: &[CompiledNode], entry, contract: &ResourceContract| {
        observations.push((nodes.to_vec(), entry, *contract));
    };
    if original {
        legacy::with_nodes(nodes, entry, contract, visit);
    } else {
        focused_visit::with_nodes(nodes, entry, contract, visit);
    }
    observations
}

#[test]
fn all_words_preserve_every_field_entry_contract_and_single_callback() {
    for malformed in [false, true] {
        let words = fixtures::words(malformed);
        assert_eq!(words.len(), 43);
        for nodes in words {
            for entry in [StepIdx::new(0), StepIdx::new(1), StepIdx::new(u16::MAX)] {
                for high in [false, true] {
                    let contract = fixtures::contract(high);
                    let expected = vec![(nodes.clone(), entry, contract)];
                    assert_eq!(observe(true, &nodes, entry, &contract), expected);
                    assert_eq!(observe(false, &nodes, entry, &contract), expected);
                }
            }
        }
    }
}

fn rejected_before_callback(original: bool, nodes: &[CompiledNode]) -> (Option<String>, usize) {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let mut observations = Vec::new();
    let contract = fixtures::contract(false);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let visit = |nodes: &[CompiledNode], entry, contract: &ResourceContract| {
            observations.push((nodes.to_vec(), entry, *contract));
        };
        if original {
            legacy::with_nodes(nodes, StepIdx::new(0), &contract, visit);
        } else {
            focused_visit::with_nodes(nodes, StepIdx::new(0), &contract, visit);
        }
    }));
    let diagnostic = match outcome {
        Ok(()) => None,
        Err(payload) => {
            let message = match payload.downcast_ref::<&str>() {
                Some(message) => (*message).to_owned(),
                None => match payload.downcast_ref::<String>() {
                    Some(message) => message.clone(),
                    None => "unexpected panic payload".to_owned(),
                },
            };
            Some(message)
        }
    };
    (diagnostic, observations.len())
}

#[test]
fn unsupported_first_or_second_kind_rejects_before_any_callback() {
    let supported = fixtures::node(CompiledNodeKind::Nop, 0, true);
    let unsupported = fixtures::node(
        CompiledNodeKind::Copy {
            source: ids::SlotIdx::new(u16::MAX),
        },
        1,
        true,
    );
    let words = [
        vec![unsupported.clone()],
        vec![unsupported.clone(), supported.clone()],
        vec![supported, unsupported.clone()],
        vec![unsupported.clone(), unsupported],
    ];
    let expected = (
        Some("budget generator produced an unsupported node kind".to_owned()),
        0,
    );
    for nodes in words {
        assert_eq!(rejected_before_callback(true, &nodes), expected);
        assert_eq!(rejected_before_callback(false, &nodes), expected);
    }
}
