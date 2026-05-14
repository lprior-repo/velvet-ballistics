#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
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
    ActionContract {
        id: ActionId::new(action),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
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
    assert_eq!(diagnostic.code.code(), 0x050D);
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
    assert_eq!(diagnostic.code.code(), 0x050E);
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
    assert_eq!(diagnostic.code.code(), 0x050F);
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
    assert_eq!(diagnostic.code.code(), 0x0510);
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
    assert_eq!(diagnostic.code.code(), 0x0511);
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

    #[test]
    fn proptest_grammar_valid(name in "[a-z][a-z0-9_]*(\\.[a-z][a-z0-9_]*){0,2}") {
        prop_assume!(name.len() <= 128, "generated name must be within byte limit");
        let contracts = [action_contract(1, Box::new([capability(&name, 1)]))];
        let result = validate_contracts(&[1], &contracts);
        prop_assert!(result.is_ok(), "valid grammar name {:?} must pass", name);
    }

    #[test]
    fn proptest_grammar_invalid(name in prop_oneof![
        Just("Network:github".to_string()),
        Just("network-github".to_string()),
        Just(".network".to_string()),
        Just("network.".to_string()),
        Just(" network".to_string()),
        Just("network ".to_string()),
    ]) {
        let contracts = [action_contract(1, Box::new([capability(&name, 1)]))];
        let result = validate_contracts(&[1], &contracts);
        prop_assert!(
            matches!(result, Err(ValidationError::CapabilityNameInvalid { .. })),
            "invalid grammar name {:?} must be rejected", name
        );
    }

    #[test]
    fn proptest_duplicate(
        cap1_name in "[a-z][a-z0-9_]*",
        cap1_action in 1u16..=8u16,
        is_duplicate in proptest::bool::ANY,
    ) {
        let cap1_name_owned = cap1_name.to_string();
        let cap1 = capability(&cap1_name_owned, cap1_action);
        let cap2_name_final = if is_duplicate {
            cap1_name_owned.clone()
        } else {
            if cap1_name == "a" { "b".to_string() } else { "a".to_string() }
        };
        let cap2 = capability(&cap2_name_final, cap1_action);
        let caps_vec = vec![cap1, cap2];
        let contracts = [action_contract(cap1_action, caps_vec.into_boxed_slice())];
        let result = validate_contracts(&[cap1_action], &contracts);
        if is_duplicate {
            prop_assert!(
                matches!(result, Err(ValidationError::CapabilityDuplicate { .. })),
                "duplicate capabilities must be detected"
            );
        } else {
            prop_assert!(result.is_ok(), "non-duplicate capabilities must pass");
        }
    }

    #[test]
    fn proptest_determinism(name in "[a-z][a-z0-9_]*(\\.[a-z][a-z0-9_]*){0,2}") {
        let contracts1 = [action_contract(1, Box::new([capability(&name, 1)]))];
        let contracts2 = [action_contract(1, Box::new([capability(&name, 1)]))];
        let result1 = validate_contracts(&[1], &contracts1);
        let result2 = validate_contracts(&[1], &contracts2);
        prop_assert_eq!(result1, result2, "same input must produce same output");
    }
}

// ============================================================================
// Capability contract schema acceptance tests
// ============================================================================

#[test]
fn test_declared_capability_passes_verification() {
    // A well-formed capability with valid lowercase name passes schema validation.
    let contracts = [action_contract(
        1,
        Box::new([capability("network", 1)]),
    )];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(result, Ok(()));
}

#[test]
fn test_multiple_declared_capabilities_are_preserved() {
    // Multiple distinct capabilities all pass validation and are preserved.
    let contracts = [
        action_contract(
            1,
            Box::new([
                capability("secrets.read", 1),
                capability("storage.write", 1),
            ]),
        ),
        action_contract(2, Box::new([capability("network", 2)])),
    ];
    let result = validate_contracts(&[1, 2], &contracts);
    assert_eq!(result, Ok(()));
}

#[test]
fn test_missing_capability_fails_verification() {
    // A capability with empty name fails schema validation (CAPABILITY_NAME_EMPTY).
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
fn test_unknown_capability_kind_fails_verification() {
    // A capability with invalid name format (uppercase, special chars) fails
    // schema validation (CAPABILITY_NAME_INVALID).
    let contracts = [action_contract(
        1,
        Box::new([capability("RESOURCE.storage", 1)]),
    )];
    let result = validate_contracts(&[1], &contracts);
    assert_eq!(
        result,
        Err(ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: "RESOURCE.storage".to_owned(),
        })
    );
}
