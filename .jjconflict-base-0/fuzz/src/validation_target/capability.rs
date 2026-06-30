//! Capability schema fuzz target bodies.

use vb_core::WorkflowParts;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};
use vb_validate::ValidationError;

fn bounded_capability_name(name: &str) -> &str {
    let mut end = name.len().min(128);
    while !name.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let Some(prefix) = name.get(..end) else {
        return "";
    };
    prefix
}

fn capability_name_is_valid(name: &str) -> bool {
    name.split('.').all(capability_segment_is_valid)
}

fn capability_segment_is_valid(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn fuzz_action_contract(
    action: u16,
    required_capabilities: Box<[Capability]>,
) -> Option<ActionContract> {
    let name = ActionName::new(format!("fuzz_action_{action}")).ok()?;
    Some(ActionContract {
        id: ActionId::new(action),
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities,
    })
}

fn fuzz_parts_with_actions(actions: &[u16]) -> WorkflowParts {
    let mut nodes = Vec::new();
    let mut index = 0u16;
    for action in actions {
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(*action),
                input: SlotIdx::new(0),
            },
        });
        index = index.saturating_add(1);
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    WorkflowParts {
        name: Box::from("capability-schema-fuzz"),
        digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

pub fn fuzz_capability_name_schema(data: &[u8]) {
    let Ok(name) = std::str::from_utf8(data) else {
        return;
    };
    let bounded_name = bounded_capability_name(name);
    let parts = fuzz_parts_with_actions(&[1]);
    let Some(contract) = fuzz_action_contract(
        1,
        Box::new([Capability::new(Box::from(bounded_name), ActionId::new(1))]),
    ) else {
        return;
    };
    let result = vb_validate::shared::validate_with_contracts(&parts, &[contract]);
    if bounded_name.is_empty() {
        assert!(matches!(result, Err(ValidationError::CapabilityNameEmpty { .. })));
    } else if !capability_name_is_valid(bounded_name) {
        assert!(matches!(result, Err(ValidationError::CapabilityNameInvalid { .. })));
    } else {
        assert!(result.is_ok());
    }
}

pub fn fuzz_capability_contract_schema(data: &[u8]) {
    let first = data.first().copied().map_or(1, u16::from);
    let second = data.get(1).copied().map_or(first, u16::from);
    let tail = match data.get(2..) {
        Some(bytes) => bytes,
        None => &[],
    };
    let name = std::str::from_utf8(tail).map_or("network", bounded_capability_name);
    let parts = fuzz_parts_with_actions(&[first]);
    let Some(contract) = fuzz_action_contract(
        first,
        Box::new([
            Capability::new(Box::from(name), ActionId::new(second)),
            Capability::new(Box::from(name), ActionId::new(second)),
        ]),
    ) else {
        return;
    };
    let result = vb_validate::shared::validate_with_contracts(&parts, &[contract]);
    if name.is_empty() {
        assert!(matches!(result, Err(ValidationError::CapabilityNameEmpty { .. })));
    } else if !capability_name_is_valid(name) {
        assert!(matches!(result, Err(ValidationError::CapabilityNameInvalid { .. })));
    } else if first != second {
        assert!(matches!(result, Err(ValidationError::CapabilityActionMismatch { .. })));
    } else {
        assert!(matches!(result, Err(ValidationError::CapabilityDuplicate { .. })));
    }
}
