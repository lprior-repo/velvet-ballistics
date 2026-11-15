#![forbid(unsafe_code)]

use vb_core::{CompiledNode, CompiledNodeKind as K, CompiledWorkflow, SlotValue, StepIdx};

pub(super) fn supports(node: &CompiledNode) -> bool {
    let handler = node.routing.error.handler().is_some();
    let error_slot = node.routing.error.error_slot().is_some();
    (!handler && !error_slot) || (handler && matches!(node.kind, K::Do { .. } | K::EvalExpr { .. }))
}

pub(super) fn has_computed_composites(plan: &CompiledWorkflow) -> bool {
    (0..plan.node_count()).any(|raw| {
        plan.node(StepIdx::new(raw)).is_some_and(|node| {
            node.output.slot().is_some()
                && matches!(node.kind, K::BuildList { .. } | K::BuildObject { .. })
        })
    })
}

pub(super) fn has_selected_immediate_retry(
    events: &[crate::JournalEvent],
    plan: &CompiledWorkflow,
) -> bool {
    if super::eligibility(plan) != Some(false)
        || matches!(
            events
                .iter()
                .rev()
                .find(|event| !matches!(event, crate::JournalEvent::RunResumed { .. })),
            Some(crate::JournalEvent::ActionFailedEvent { .. })
        )
    {
        return false;
    }
    events.iter().any(|event| match event {
        crate::JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.attempt > 1 => {
            plan.node(ticket.step).is_some_and(|node| {
                matches!(node.kind, K::Do { .. })
                    && node.retry.is_some_and(|policy| policy.delay_ms == 0)
            })
        }
        _ => false,
    })
}

pub(super) fn retained_roots_need_checkpoint(
    plan: &CompiledWorkflow,
    roots: &[(vb_core::SlotIdx, SlotValue, vb_core::Taint)],
) -> bool {
    roots.iter().any(|(_, value, _)| match value {
        SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_) => true,
        SlotValue::Symbol(symbol) => plan
            .source_metadata()
            .and_then(|metadata| metadata.symbol(*symbol))
            .is_none(),
        SlotValue::Null | SlotValue::Bool(_) | SlotValue::I64(_) | SlotValue::F64(_) => false,
        _ => true,
    })
}
