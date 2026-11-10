use super::*;

fn ask() -> vb_core::CompiledNode {
    node(
        1,
        K::Ask {
            prompt: slot(0),
            timeout_slot: None,
        },
        Some(1),
        Some(2),
    )
}

#[test]
fn successful_ask_continuation_supplies_host_answer() -> TestResult {
    CompiledWorkflow::try_from_parts(parts(
        vec![
            set(0, 0, 0, 1),
            ask(),
            node(2, K::AskResume { answer: slot(1) }, Some(2), Some(3)),
            finish(3, 2),
        ],
        3,
    ))?;
    Ok(())
}

#[test]
fn bypassing_ask_does_not_supply_an_answer() -> TestResult {
    let mut p = parts(
        vec![
            choose(0, 1, 2),
            ask(),
            node(2, K::AskResume { answer: slot(1) }, Some(2), Some(3)),
            finish(3, 2),
        ],
        3,
    );
    declare(&mut p, 0, vb_core::InputSlotKind::Bool);
    // Use a separate numeric prompt; Bool is not an accepted prompt shape.
    if let Some(n) = p.nodes.get_mut(1) {
        n.kind = K::Ask {
            prompt: slot(3),
            timeout_slot: None,
        };
    }
    p.slot_count = 4;
    declare(&mut p, 3, vb_core::InputSlotKind::I64);
    rejected(p, vb_core::workflow::SlotFlowFailure::UninitializedRead)
}

#[test]
fn ask_failure_does_not_supply_an_answer() -> TestResult {
    let mut a = ask();
    a.routing.error = vb_core::workflow::ErrorRouting::Handler {
        handler: step(2),
        error_slot: None,
    };
    rejected(
        parts(
            vec![
                set(0, 0, 0, 1),
                a,
                node(2, K::AskResume { answer: slot(1) }, Some(2), Some(3)),
                finish(3, 2),
            ],
            3,
        ),
        vb_core::workflow::SlotFlowFailure::UninitializedRead,
    )
}
