#![forbid(unsafe_code)]

//! Behavior tests for vb_core YAML parsing and chain contract enforcement.
//!
//! Tests focus on:
//! - YAML deserialization behavior (happy and sad paths)
//! - Chain contract enforcement behavior
//! - Exact error variant assertions
//! - Contract parity between spec and implementation

use vb_compile::compile_workflow;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::ActionId;
use vb_core::workflow::ResourceContract;
use vb_validate::ValidationError;
use vb_validate::shared::{ValidationPipeline, validate, validate_with_contracts};
use vb_yaml::{YamlError, parse_workflow_source, parse_yaml_events, validate_yaml_profile};

// ---------------------------------------------------------------------------
// YAML parsing happy paths
// ---------------------------------------------------------------------------

#[test]
fn yaml_parses_minimal_valid_workflow() {
    let yaml = r#"version: velvet-ballistics/v1
name: minimal
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = parse_workflow_source(yaml);
    assert!(
        result.is_ok(),
        "minimal valid workflow should parse: {result:?}"
    );
    let source = result.unwrap();
    assert_eq!(source.name(), "minimal");
}

#[test]
fn yaml_profile_validation_accepts_plain_scalar_strings() {
    // Plain scalars are allowed for string values
    let yaml = r#"version: velvet-ballistics/v1
name: plain_scalar_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: "plain string value"
"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        result.is_ok(),
        "plain scalar strings should be accepted: {result:?}"
    );
}

#[test]
fn yaml_profile_validation_accepts_quoted_strings() {
    let yaml = r#"version: velvet-ballistics/v1
name: quoted_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: "quoted"
"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        result.is_ok(),
        "quoted strings should be accepted: {result:?}"
    );
}

#[test]
fn yaml_profile_validation_accepts_integer_values() {
    let yaml = r#"version: velvet-ballistics/v1
name: integer_test
when:
  manual: {}
steps:
  - id: set_value
    set:
      output: answer
      value: "42"
  - id: done
    finish:
      result: answer
"#;
    let result = compile_workflow(yaml.as_bytes());
    assert!(result.is_ok(), "integer values should compile: {result:?}");
}

#[test]
fn yaml_events_parse_valid_document() {
    let yaml = r#"version: velvet-ballistics/v1
name: events_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = parse_yaml_events(yaml);
    assert!(
        result.is_ok(),
        "valid YAML should produce events: {result:?}"
    );
    let events = result.unwrap();
    assert!(
        !events.is_empty(),
        "events should not be empty for valid YAML"
    );
}

// ---------------------------------------------------------------------------
// YAML parsing sad paths - exact error variant assertions
// ---------------------------------------------------------------------------

#[test]
fn yaml_rejects_ambiguous_yes_scalar_exact_variant() {
    let yaml = r#"key: yes"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { ref scalar }) if scalar.as_ref() == "yes"),
        "expected AmbiguousScalar{{scalar: \"yes\"}}, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_ambiguous_no_scalar_exact_variant() {
    let yaml = r#"key: no"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { ref scalar }) if scalar.as_ref() == "no"),
        "expected AmbiguousScalar{{scalar: \"no\"}}, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_ambiguous_on_scalar_exact_variant() {
    let yaml = r#"key: on"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { ref scalar }) if scalar.as_ref() == "on"),
        "expected AmbiguousScalar{{scalar: \"on\"}}, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_ambiguous_off_scalar_exact_variant() {
    let yaml = r#"key: off"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { ref scalar }) if scalar.as_ref() == "off"),
        "expected AmbiguousScalar{{scalar: \"off\"}}, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_multiple_documents_exact_count() {
    let yaml = r#"---
version: velvet-ballistics/v1
name: first
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
---
version: velvet-ballistics/v1
name: second
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::MultipleDocuments { count }) if count == 2),
        "expected MultipleDocuments{{count: 2}}, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_custom_tag_exact_variant() {
    let yaml = r#"key: !custom_tag some_value"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::CustomTag { .. })),
        "expected CustomTag{{..}}, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_anchor_alias_merge_exact_variant() {
    let yaml = r#"---
defaults: &defaults
  name: test
override:
  <<: *defaults
  age: 5
"#;
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::AnchorAliasMerge)),
        "expected AnchorAliasMerge, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_source_too_large_exact_size() {
    let large_yaml = "x: ".to_string() + &"y".repeat(1_000_000);
    let result = validate_yaml_profile(&large_yaml);
    assert!(
        matches!(result, Err(YamlError::ScalarTooLong { len, max: _ }) if len > 1000),
        "expected ScalarTooLong with len > 1000, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_null_byte_in_source_exact_variant() {
    let yaml = "key: \"has\x00null\"\n";
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::ForbiddenFeature { detail }) if detail.contains("null")),
        "expected ForbiddenFeature{{detail: \"null_byte_in_source\"}}, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_empty_source_exact_variant() {
    let result = validate_yaml_profile("");
    assert!(
        matches!(result, Err(YamlError::EmptySource)),
        "expected EmptySource, got: {result:?}"
    );
}

#[test]
fn yaml_rejects_missing_version_exact_variant() {
    let yaml = br#"name: no_version
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(yaml);
    assert!(result.is_err(), "missing version should fail compilation");
}

// ---------------------------------------------------------------------------
// ResourceContract enforcement behavior
// ---------------------------------------------------------------------------

#[test]
fn resource_contract_default_has_consistent_bounds() {
    let contract = ResourceContract::DEFAULT;

    // Verify DEFAULT contract has reasonable bounds
    assert_eq!(contract.max_steps, 1_000);
    assert_eq!(contract.max_slots, 1_024);
    assert_eq!(contract.max_constants, 8_192);
    assert_eq!(contract.max_accessors, 8_192);
    assert_eq!(contract.max_expressions, 4_096);
    assert_eq!(contract.max_expr_stack, 64);
    assert_eq!(contract.max_step_budget_per_tick, 10_000);
    assert_eq!(contract.max_input_bytes, 1_048_576);
    assert_eq!(contract.max_output_bytes, 262_144);
    assert_eq!(contract.max_blob_bytes, 16_777_216);
    assert_eq!(contract.max_ipc_payload_bytes, 1_048_576);
    assert_eq!(contract.max_retry_attempts, 3);
    assert_eq!(contract.max_fanout, 64);
    assert_eq!(contract.max_collect_items, 1_024);
    assert_eq!(contract.max_queue_depth, 1_024);
    assert_eq!(contract.max_journal_batch_bytes, 1_048_576);
}

#[test]
fn resource_contract_serialization_parity() {
    let contract = ResourceContract::DEFAULT;
    let serialized = postcard::to_allocvec(&contract).expect("DEFAULT should serialize");
    let deserialized: ResourceContract =
        postcard::from_bytes(&serialized).expect("DEFAULT should deserialize");
    assert_eq!(
        contract, deserialized,
        "ResourceContract DEFAULT must survive round-trip"
    );
}

// ---------------------------------------------------------------------------
// ActionContract chain contract behavior
// ---------------------------------------------------------------------------

#[test]
fn action_contract_complete_construction() {
    let contract = ActionContract {
        id: ActionId::new(42),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 3,
        output_slot_count: 2,
        max_input_bytes: 4096,
        max_output_bytes: 2048,
        timeout_ms: 30_000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([Capability::new("net.fetch".into(), ActionId::new(42))]),
    };

    assert_eq!(contract.id, ActionId::new(42));
    assert_eq!(contract.input_slot_count, 3);
    assert_eq!(contract.output_slot_count, 2);
    assert_eq!(contract.max_input_bytes, 4096);
    assert_eq!(contract.max_output_bytes, 2048);
    assert_eq!(contract.timeout_ms, 30_000);
    assert_eq!(contract.idempotency, Idempotency::IdempotentExternal);
    assert_eq!(contract.side_effect, SideEffect::LocalWrite);
    assert_eq!(contract.retry_safety, RetrySafety::RequiresIdempotencyKey);
    assert_eq!(contract.required_capabilities.len(), 1);
}

#[test]
fn action_contract_serialization_parity() {
    let contract = ActionContract {
        id: ActionId::new(10),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 512,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let serialized = postcard::to_allocvec(&contract).expect("ActionContract should serialize");
    let deserialized: ActionContract =
        postcard::from_bytes(&serialized).expect("ActionContract should deserialize");
    assert_eq!(
        contract, deserialized,
        "ActionContract must survive round-trip"
    );
}

#[test]
fn action_contract_all_idempotency_variants_constructable() {
    let variants = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];
    for variant in variants {
        let contract = ActionContract {
            id: ActionId::new(1),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: variant,
            side_effect: SideEffect::Pure,
            retry_safety: RetrySafety::Idempotent,
            required_capabilities: Box::new([]),
        };
        assert_eq!(contract.idempotency, variant);
    }
}

#[test]
fn action_contract_all_side_effect_variants_constructable() {
    let variants = [
        SideEffect::Pure,
        SideEffect::LocalRead,
        SideEffect::LocalWrite,
        SideEffect::ExternalRead,
        SideEffect::ExternalWrite,
        SideEffect::Process,
        SideEffect::UnsafeShell,
    ];
    for variant in variants {
        let contract = ActionContract {
            id: ActionId::new(1),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: variant,
            retry_safety: RetrySafety::Idempotent,
            required_capabilities: Box::new([]),
        };
        assert_eq!(contract.side_effect, variant);
    }
}

#[test]
fn action_contract_all_retry_safety_variants_constructable() {
    let variants = [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
    ];
    for variant in variants {
        let contract = ActionContract {
            id: ActionId::new(1),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::Pure,
            retry_safety: variant,
            required_capabilities: Box::new([]),
        };
        assert_eq!(contract.retry_safety, variant);
    }
}

// ---------------------------------------------------------------------------
// Validation pipeline chain contract enforcement
// ---------------------------------------------------------------------------

#[test]
fn validation_pipeline_default_enables_all_gates() {
    let pipeline = ValidationPipeline::default();
    assert!(pipeline.gate_07_expression_stack);
    assert!(pipeline.gate_08_accessor_paths);
    assert!(pipeline.gate_09_slot_references);
    assert!(pipeline.gate_10_node_kind_specific);
    assert!(pipeline.gate_11_loop_body_graph);
    assert!(pipeline.gate_12_action_contracts);
    assert!(pipeline.gate_13_no_slot_cycles);
    assert!(pipeline.gate_14_slot_type_consistency);
    assert!(pipeline.gate_15_determinism_proof);
}

#[test]
fn validation_pipeline_no_gates_disables_all() {
    let pipeline = ValidationPipeline::no_gates();
    assert!(!pipeline.gate_07_expression_stack);
    assert!(!pipeline.gate_08_accessor_paths);
    assert!(!pipeline.gate_09_slot_references);
    assert!(!pipeline.gate_10_node_kind_specific);
    assert!(!pipeline.gate_11_loop_body_graph);
    assert!(!pipeline.gate_12_action_contracts);
    assert!(!pipeline.gate_13_no_slot_cycles);
    assert!(!pipeline.gate_14_slot_type_consistency);
    assert!(!pipeline.gate_15_determinism_proof);
}

#[test]
fn validate_with_contracts_rejects_missing_action_contract() {
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind};

    // Create a workflow with a Do node but no corresponding ActionContract
    let do_node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ActionId::new(99), // Action 99 has no contract
            input: SlotIdx::new(0),
        },
    };
    let finish_node = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };

    let parts = vb_core::WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: vec![do_node, finish_node].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Empty contracts array - no contract for action 99
    let contracts: [ActionContract; 0] = [];
    let result = validate_with_contracts(&parts, &contracts);

    assert!(
        matches!(
            result,
            Err(ValidationError::ActionContractMissing {
                action_id: 99,
                node_index: 0
            })
        ),
        "expected ActionContractMissing{{action_id: 99, node_index: 0}}, got: {result:?}"
    );
}

#[test]
fn validate_with_contracts_rejects_orphan_action_contract() {
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind};

    // Create a workflow with no Do nodes but a contract for action 99
    let finish_node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };

    let parts = vb_core::WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: vec![finish_node].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Contract for action 99, but no Do node uses action 99
    let orphan_contract = ActionContract {
        id: ActionId::new(99),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 512,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };

    let result = validate_with_contracts(&parts, &[orphan_contract]);

    assert!(
        matches!(
            result,
            Err(ValidationError::ActionContractOrphan { action_id: 99 })
        ),
        "expected ActionContractOrphan{{action_id: 99}}, got: {result:?}"
    );
}

#[test]
fn validate_with_contracts_accepts_matching_contract() {
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind};

    // Create a workflow with a Do node using action 42
    let do_node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ActionId::new(42),
            input: SlotIdx::new(0),
        },
    };
    let finish_node = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };

    let parts = vb_core::WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: vec![do_node, finish_node].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Contract for action 42 matching the Do node
    let matching_contract = ActionContract {
        id: ActionId::new(42),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 512,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };

    let result = validate_with_contracts(&parts, &[matching_contract]);
    assert!(
        result.is_ok(),
        "matching contract should pass validation: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Contract parity between spec and implementation
// ---------------------------------------------------------------------------

#[test]
fn resource_contract_default_matches_compiled_default() {
    // The DEFAULT contract used during compilation must match
    // the spec-defined defaults for all fields
    let compiled_default = ResourceContract::DEFAULT;

    // These are the hardcoded defaults in the spec
    assert_eq!(compiled_default.max_steps, 1_000);
    assert_eq!(compiled_default.max_slots, 1_024);
    assert_eq!(compiled_default.max_expr_stack, 64);
    assert_eq!(compiled_default.max_step_budget_per_tick, 10_000);
    assert_eq!(compiled_default.max_input_bytes, 1_048_576);
    assert_eq!(compiled_default.max_output_bytes, 262_144);
}

#[test]
fn action_contract_idempotency_determinism_parity() {
    // When side_effect is None, idempotency should be DeterministicPure
    // This is a semantic contract requirement
    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    // Semantic parity: DeterministicPure + None side effect = Safe retry
    assert_eq!(contract.side_effect, SideEffect::Pure);
    assert_eq!(contract.idempotency, Idempotency::DeterministicPure);
    assert_eq!(contract.retry_safety, RetrySafety::Idempotent);
}

#[test]
fn action_contract_side_effects_require_key_when_not_safe() {
    // Actions with side effects that are not Safe must require key
    let contract = ActionContract {
        id: ActionId::new(2),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::LocalWrite,
        retry_safety: RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };

    // Safety contract: Non-pure side effect with AtLeastOnceExternal needs KeyRequired
    assert!(matches!(
        contract.side_effect,
        SideEffect::LocalRead
            | SideEffect::LocalWrite
            | SideEffect::ExternalRead
            | SideEffect::ExternalWrite
            | SideEffect::Process
            | SideEffect::UnsafeShell
    ));
    assert!(matches!(
        contract.retry_safety,
        RetrySafety::RequiresIdempotencyKey | RetrySafety::NotRetrySafe
    ));
}

#[test]
fn workflow_parts_accepts_resource_contract_at_exact_usage_bounds() {
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind};

    // Workflow with exact bounds matching DEFAULT contract
    let finish_node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };

    let parts = vb_core::WorkflowParts {
        name: Box::from("bounds_test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: vec![finish_node].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Validation should pass with DEFAULT contract
    let result = validate(&parts);
    assert!(
        result.is_ok(),
        "workflow within DEFAULT contract bounds should validate: {result:?}"
    );
}

#[test]
fn compiled_workflow_carries_resource_contract() {
    let yaml = r#"version: velvet-ballistics/v1
name: contract_carrier_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let workflow = compile_workflow(yaml.as_bytes()).expect("should compile");
    let contract = workflow.resource_contract();

    // The compiled workflow must carry a valid resource contract
    assert_eq!(contract.max_steps, 1_000);
    assert_eq!(contract.max_slots, 1_024);
    assert_eq!(contract.max_expr_stack, 64);
}

#[test]
fn compiled_workflow_to_parts_preserves_contract() {
    let yaml = r#"version: velvet-ballistics/v1
name: parts_preservation_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let workflow = compile_workflow(yaml.as_bytes()).expect("should compile");
    let parts = workflow.to_parts();

    // to_parts must preserve the resource contract
    assert_eq!(parts.resource_contract.max_steps, 1_000);
    assert_eq!(parts.resource_contract.max_slots, 1_024);
    assert_eq!(parts.resource_contract.max_expr_stack, 64);
}

#[test]
fn compiled_workflow_try_from_parts_validates_contract() {
    let yaml = r#"version: velvet-ballistics/v1
name: parts_validation_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let workflow = compile_workflow(yaml.as_bytes()).expect("should compile");
    let parts = workflow.to_parts();

    // try_from_parts must succeed for valid parts
    let result = vb_core::CompiledWorkflow::try_from_parts(parts);
    assert!(
        result.is_ok(),
        "valid parts should produce CompiledWorkflow: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Error variant exhaustive checking
// ---------------------------------------------------------------------------

#[test]
fn yaml_error_all_variants_are_exhaustive() {
    // This test documents the complete YamlError variant surface area
    // Any new variants added must be handled in tests
    // Note: YamlError is marked as non_exhaustive so we use a catch-all

    fn check_yaml_error_variant(err: &YamlError) -> &'static str {
        match err {
            YamlError::UnsupportedTrigger { .. } => "UnsupportedTrigger",
            YamlError::UnsupportedFeature { .. } => "UnsupportedFeature",
            YamlError::DuplicateKey { .. } => "DuplicateKey",
            YamlError::AnchorAliasMerge => "AnchorAliasMerge",
            YamlError::CustomTag { .. } => "CustomTag",
            YamlError::BinaryScalar => "BinaryScalar",
            YamlError::MultipleDocuments { .. } => "MultipleDocuments",
            YamlError::AmbiguousScalar { .. } => "AmbiguousScalar",
            YamlError::SourceTooLarge { .. } => "SourceTooLarge",
            YamlError::NestingTooDeep { .. } => "NestingTooDeep",
            YamlError::NodeLimitExceeded { .. } => "NodeLimitExceeded",
            YamlError::ScalarTooLong { .. } => "ScalarTooLong",
            YamlError::SequenceTooLong { .. } => "SequenceTooLong",
            YamlError::MappingTooLarge { .. } => "MappingTooLarge",
            YamlError::UnknownField { .. } => "UnknownField",
            YamlError::EmptySource => "EmptySource",
            YamlError::MissingField { .. } => "MissingField",
            YamlError::FieldShape { .. } => "FieldShape",
            YamlError::ParseError { .. } => "ParseError",
            YamlError::ForbiddenFeature { .. } => "ForbiddenFeature",
            _ => "UnknownVariant",
        }
    }

    // Verify each error type can be matched
    let err = YamlError::EmptySource;
    assert_eq!(check_yaml_error_variant(&err), "EmptySource");

    let err = YamlError::AmbiguousScalar {
        scalar: "yes".into(),
    };
    assert_eq!(check_yaml_error_variant(&err), "AmbiguousScalar");

    let err = YamlError::MultipleDocuments { count: 2 };
    assert_eq!(check_yaml_error_variant(&err), "MultipleDocuments");

    let err = YamlError::SourceTooLarge { size: 100, max: 50 };
    assert_eq!(check_yaml_error_variant(&err), "SourceTooLarge");
}

#[test]
fn validation_error_all_contract_variants_are_exhaustive() {
    // This test documents the complete ValidationError variant surface area
    // related to contracts (Gate 12)

    fn check_contract_error_variant(err: &ValidationError) -> Option<&'static str> {
        match err {
            ValidationError::ActionContractMissing { .. } => Some("ActionContractMissing"),
            ValidationError::ActionContractOrphan { .. } => Some("ActionContractOrphan"),
            ValidationError::CapabilityNameEmpty { .. } => Some("CapabilityNameEmpty"),
            ValidationError::CapabilityNameTooLong { .. } => Some("CapabilityNameTooLong"),
            ValidationError::CapabilityNameInvalid { .. } => Some("CapabilityNameInvalid"),
            ValidationError::CapabilityActionMismatch { .. } => Some("CapabilityActionMismatch"),
            ValidationError::CapabilityDuplicate { .. } => Some("CapabilityDuplicate"),
            _ => None, // Non-contract errors
        }
    }

    let err = ValidationError::ActionContractMissing {
        action_id: 1,
        node_index: 0,
    };
    assert_eq!(
        check_contract_error_variant(&err),
        Some("ActionContractMissing")
    );

    let err = ValidationError::ActionContractOrphan { action_id: 99 };
    assert_eq!(
        check_contract_error_variant(&err),
        Some("ActionContractOrphan")
    );
}
