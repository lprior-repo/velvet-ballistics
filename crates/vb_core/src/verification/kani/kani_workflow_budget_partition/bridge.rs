use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};

use super::super::budget_workflow_inputs;
use super::witness::Witness;
use super::{domain, sampling};

fn observations(nodes: &[CompiledNode]) -> (usize, bool, bool, bool, bool) {
    (
        nodes.len(),
        nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::Nop)),
        nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::Do { .. })),
        nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::WaitUntil { .. })),
        nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::Finish { .. })),
    )
}

#[kani::proof]
#[kani::unwind(6)]
fn kani_harness_whole_workflow_budget_compute_domain_original_words_covered() {
    let inputs = budget_workflow_inputs();
    let original_word = super::word::index(inputs.nodes());
    assert!(original_word.is_some(), "every original input needs a word");
    if let Some(index) = original_word {
        assert!(index <= 42);
        assert!(super::word::matches(inputs.nodes(), index));
    }
    let mut visited = false;
    inputs.with_nodes(|nodes, entry, _| {
        assert!(!visited, "original visitor must execute once");
        visited = true;
        assert!(domain::original_projection(nodes, entry));
        let classified = super::word::index(nodes);
        assert_eq!(classified, original_word);
        assert!(classified.is_some(), "every original input needs a word");
        if let Some(index) = classified {
            assert!(index <= 42);
            assert!(super::word::matches(nodes, index));
        }
    });
    assert!(visited, "original visitor must execute once");
}

fn check_original(expected_word: u8) -> bool {
    let inputs = budget_workflow_inputs();
    kani::assume(super::word::matches(inputs.nodes(), expected_word));
    let original = Witness::decode(inputs.nodes(), inputs.entry(), inputs.contract());
    let before = (
        inputs.node_count(),
        inputs.covers_nop(),
        inputs.covers_do(),
        inputs.covers_wait_until(),
        inputs.covers_finish(),
    );
    let mut visited = false;
    inputs.with_nodes(|nodes, entry, contract| {
        assert!(!visited, "original visitor must execute once");
        visited = true;
        assert!(domain::original_projection(nodes, entry));
        assert_eq!(observations(nodes), before);
        let decoded = Witness::decode(nodes, entry, contract);
        assert_eq!(
            decoded, original,
            "original visitor must preserve every field"
        );
        match decoded {
            Some(witness) => witness.with_nodes(|rebuilt, rebuilt_entry, rebuilt_contract| {
                assert!(super::node_equality::focused_nodes_equal(rebuilt, nodes));
                assert_eq!(rebuilt_entry, entry);
                assert_eq!(rebuilt_contract, contract);
                assert_eq!(observations(rebuilt), before);
            }),
            None => assert!(false, "original producer must decode completely"),
        }
    });
    assert!(visited, "original visitor must execute once");
    visited
}

macro_rules! original_case {
    ($name:ident, $index:literal, $label:literal) => {
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name() {
            let visited = super::check_original($index);
            kani::cover!(visited, $label);
        }
    };
}

#[path = "bridge_old_cases.rs"]
mod old_cases;

#[kani::proof]
#[kani::unwind(6)]
fn kani_harness_whole_workflow_budget_compute_domain_case_to_original() {
    let selected: u8 = kani::any();
    let witness = sampling::case_witness(selected);
    witness.with_nodes(|nodes, entry, contract| {
        assert!(domain::original_projection(nodes, entry));
        assert_eq!(Witness::decode(nodes, entry, contract), Some(witness));
        cover_cases(selected);
        cover_fields(nodes, entry);
    });
}

fn cover_fields(nodes: &[CompiledNode], entry: StepIdx) {
    kani::cover!(
        nodes.iter().any(|node| node.output.slot().is_none()),
        "bridge preserves Discard"
    );
    kani::cover!(
        nodes.iter().any(|node| node.output.slot().is_some()),
        "bridge preserves output slots"
    );
    kani::cover!(
        nodes.iter().any(|node| node.routing.next.step().is_none()),
        "bridge preserves absent next"
    );
    kani::cover!(
        nodes
            .iter()
            .any(|node| node.routing.error.handler().is_some()),
        "bridge preserves handlers"
    );
    kani::cover!(
        nodes
            .iter()
            .any(|node| node.routing.error.error_slot().is_some()),
        "bridge preserves error slots"
    );
    kani::cover!(
        nodes.len() == 2 && entry == StepIdx::new(1),
        "bridge preserves second entry"
    );
}

fn cover_cases(selected: u8) {
    macro_rules! cover {
        ($value:literal, $label:literal) => {
            kani::cover!(selected == $value, $label);
        };
    }
    cover!(0, "bridge case 0");
    cover!(1, "bridge case 1");
    cover!(2, "bridge case 2");
    cover!(3, "bridge case 3");
    cover!(4, "bridge case 4");
    cover!(5, "bridge case 5");
    cover!(6, "bridge case 6");
    cover!(7, "bridge case 7");
    cover!(8, "bridge case 8");
    cover!(9, "bridge case 9");
    cover!(10, "bridge case 10");
    cover!(11, "bridge case 11");
    cover!(12, "bridge case 12");
    cover!(13, "bridge case 13");
    cover!(14, "bridge case 14");
    cover!(15, "bridge case 15");
    cover!(16, "bridge case 16");
    cover!(17, "bridge case 17");
    cover!(18, "bridge case 18");
    cover!(19, "bridge case 19");
    cover!(20, "bridge case 20");
    cover!(21, "bridge case 21");
    cover!(22, "bridge case 22");
    cover!(23, "bridge case 23");
    cover!(24, "bridge case 24");
    cover!(25, "bridge case 25");
    cover!(26, "bridge case 26");
    cover!(27, "bridge case 27");
    cover!(28, "bridge case 28");
    cover!(29, "bridge case 29");
    cover!(30, "bridge case 30");
    cover!(31, "bridge case 31");
    cover!(32, "bridge case 32");
    cover!(33, "bridge case 33");
    cover!(34, "bridge case 34");
    cover!(35, "bridge case 35");
    cover!(36, "bridge case 36");
    cover!(37, "bridge case 37");
    cover!(38, "bridge case 38");
    cover!(39, "bridge case 39");
    cover!(40, "bridge case 40");
    cover!(41, "bridge case 41");
    kani::cover!(selected >= 42, "bridge case 42");
}
