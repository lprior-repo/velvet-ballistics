#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::ValidationError;
use vb_validate::diagnostic::diagnostic_from_error;
use vb_validate::shared::{self, ValidationPipeline};

const MAX_CAPABILITY_NAME_BYTES: usize = 128;

fn workflow_with_do_actions(actions: &[u16]) -> WorkflowParts {
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
        name: Box::from("capability-schema-red"),
        digest: WorkflowDigest::from_bytes([0; 32]),
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

fn capability(name: &str, action: u16) -> Capability {
    Capability::new(Box::from(name), ActionId::new(action))
}

fn action_contract(action: u16, required_capabilities: Box<[Capability]>) -> ActionContract {
    let name = match ActionName::new("test-action") {
        Ok(v) => v,
        Err(e) => panic!("ActionName::new(\"test-action\") should succeed: {e:?}"),
    };
    ActionContract {
        id: ActionId::new(action),
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities,
    }
}

fn validate_contracts(
    actions: &[u16],
    contracts: &[ActionContract],
) -> Result<(), ValidationError> {
    let parts = workflow_with_do_actions(actions);
    ValidationPipeline::default().validate_with_contracts(&parts, contracts)
}

fn long_valid_name(len: usize) -> String {
    "a".repeat(len)
}

#[test]
fn validation_pipeline_returns_capability_name_empty_when_requirement_name_is_empty() {
    let contracts = [action_contract(1, Box::new([capability("", 1)]))];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0,
        })
    );
}

#[test]
fn validation_pipeline_returns_capability_name_too_long_when_name_has_129_bytes() {
    let name = long_valid_name(129);
    let contracts = [action_contract(1, Box::new([capability(&name, 1)]))];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameTooLong {
            action_id: 1,
            capability_index: 0,
            len: 129,
            max: MAX_CAPABILITY_NAME_BYTES,
        })
    );
}

#[test]
fn validation_pipeline_returns_capability_name_invalid_when_name_contains_colon() {
    let contracts = [action_contract(
        1,
        Box::new([capability("network:github", 1)]),
    )];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: "network:github".to_owned(),
        })
    );
}

#[test]
fn validation_pipeline_returns_capability_name_invalid_when_name_has_leading_dot() {
    let contracts = [action_contract(1, Box::new([capability(".network", 1)]))];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: ".network".to_owned(),
        })
    );
}

#[test]
fn validation_pipeline_returns_capability_name_invalid_when_name_has_trailing_dot() {
    let contracts = [action_contract(1, Box::new([capability("network.", 1)]))];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: "network.".to_owned(),
        })
    );
}

#[test]
fn validation_pipeline_returns_capability_name_invalid_when_name_has_uppercase() {
    let contracts = [action_contract(1, Box::new([capability("Network", 1)]))];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: "Network".to_owned(),
        })
    );
}

#[test]
fn validation_pipeline_returns_capability_action_mismatch_when_requirement_action_differs_from_contract()
 {
    let contracts = [action_contract(1, Box::new([capability("network", 2)]))];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityActionMismatch {
            contract_action_id: 1,
            capability_action_id: 2,
            capability_index: 0,
        })
    );
}

#[test]
fn validation_pipeline_returns_capability_duplicate_when_same_name_and_action_repeat_in_one_contract()
 {
    let contracts = [action_contract(
        1,
        Box::new([capability("network", 1), capability("network", 1)]),
    )];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityDuplicate {
            action_id: 1,
            first_index: 0,
            duplicate_index: 1,
            name: "network".to_owned(),
        })
    );
}

#[test]
fn validation_pipeline_returns_earliest_capability_duplicate_when_multiple_duplicates_exist() {
    let contracts = [action_contract(
        1,
        Box::new([
            capability("network", 1),
            capability("fs.read", 1),
            capability("network", 1),
            capability("fs.read", 1),
        ]),
    )];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityDuplicate {
            action_id: 1,
            first_index: 0,
            duplicate_index: 2,
            name: "network".to_owned(),
        })
    );
}

#[test]
fn validation_pipeline_returns_first_schema_error_before_duplicate_and_orphan_checks() {
    let contracts = [
        action_contract(
            1,
            Box::new([
                capability("", 1),
                capability("network:github", 2),
                capability("network", 1),
                capability("network", 1),
            ]),
        ),
        action_contract(9, Box::new([])),
    ];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0,
        })
    );
}

#[test]
fn shared_validate_with_contracts_returns_capability_name_empty_when_live_gate_rejects_empty_name()
{
    let parts = workflow_with_do_actions(&[1]);
    let contracts = [action_contract(1, Box::new([capability("", 1)]))];
    let result = shared::validate_with_contracts(&parts, &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0,
        })
    );
}

#[test]
fn diagnostic_conversion_returns_e050d_when_error_is_capability_name_empty() {
    let error = ValidationError::CapabilityNameEmpty {
        action_id: 1,
        capability_index: 0,
    };
    let diagnostic = diagnostic_from_error(&error);
    assert_eq!(diagnostic.numeric_code.code(), 0x050D);
    assert_eq!(
        diagnostic.message.as_ref(),
        "capability name is empty for action 1 at required_capabilities[0]"
    );
}

#[test]
fn diagnostic_conversion_returns_e050e_when_error_is_capability_name_too_long() {
    let error = ValidationError::CapabilityNameTooLong {
        action_id: 1,
        capability_index: 0,
        len: 129,
        max: 128,
    };
    let diagnostic = diagnostic_from_error(&error);
    assert_eq!(diagnostic.numeric_code.code(), 0x050E);
    assert_eq!(
        diagnostic.message.as_ref(),
        "capability name too long for action 1 at required_capabilities[0]: 129 > 128"
    );
}

#[test]
fn diagnostic_conversion_returns_e050f_when_error_is_capability_name_invalid() {
    let error = ValidationError::CapabilityNameInvalid {
        action_id: 1,
        capability_index: 0,
        name: "network:github".to_owned(),
    };
    let diagnostic = diagnostic_from_error(&error);
    assert_eq!(diagnostic.numeric_code.code(), 0x050F);
    assert_eq!(
        diagnostic.message.as_ref(),
        "invalid capability name for action 1 at required_capabilities[0]: network:github"
    );
}

#[test]
fn diagnostic_conversion_returns_e0510_when_error_is_capability_action_mismatch() {
    let error = ValidationError::CapabilityActionMismatch {
        contract_action_id: 1,
        capability_action_id: 2,
        capability_index: 0,
    };
    let diagnostic = diagnostic_from_error(&error);
    assert_eq!(diagnostic.numeric_code.code(), 0x0510);
    assert_eq!(
        diagnostic.message.as_ref(),
        "capability action 2 does not match contract action 1 at required_capabilities[0]"
    );
}

#[test]
fn diagnostic_conversion_returns_e0511_when_error_is_capability_duplicate() {
    let error = ValidationError::CapabilityDuplicate {
        action_id: 1,
        first_index: 0,
        duplicate_index: 1,
        name: "network".to_owned(),
    };
    let diagnostic = diagnostic_from_error(&error);
    assert_eq!(diagnostic.numeric_code.code(), 0x0511);
    assert_eq!(
        diagnostic.message.as_ref(),
        "duplicate capability requirement for action 1: network at required_capabilities[0] and required_capabilities[1]"
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn proptest_valid_shaped_too_long_names_return_capability_name_too_long(len in 129usize..=256usize) {
        let name = long_valid_name(len);
        let contracts = [action_contract(1, Box::new([capability(&name, 1)]))];
        let result = validate_contracts(&[1], &contracts);
        prop_assert_eq!(
            result,
            Err(ValidationError::CapabilityNameTooLong {
                action_id: 1,
                capability_index: 0,
                len,
                max: MAX_CAPABILITY_NAME_BYTES,
            })
        );
    }

    #[test]
    fn proptest_unequal_capability_action_returns_action_mismatch(capability_action in 2u16..=64u16) {
        let contracts = [action_contract(1, Box::new([capability("network", capability_action)]))];
        let result = validate_contracts(&[1], &contracts);
        prop_assert_eq!(
            result,
            Err(ValidationError::CapabilityActionMismatch {
                contract_action_id: 1,
                capability_action_id: usize::from(capability_action),
                capability_index: 0,
            })
        );
    }
}
