use proptest::prelude::*;
use vb_core::action::{
    ActionContract, ActionTicket, Idempotency, IdempotencyViolation, RetrySafety, SideEffect,
    validate_idempotency_key_ingredients, verify_idempotency,
};
use vb_core::capability::Capability;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::idempotency_contract::{
    IdempotencyContractError, IdempotencyContractErrors, IdempotencyContractViolation,
    collect_idempotency_contract_violations, is_statically_idempotent_contract,
    validate_action_idempotency_contract, validate_workflow_idempotency_contracts,
};

fn action(value: u16) -> ActionId {
    ActionId::new(value)
}

fn slot(value: u16) -> SlotIdx {
    SlotIdx::new(value)
}

fn step(value: u16) -> StepIdx {
    StepIdx::new(value)
}

fn contract(
    id: ActionId,
    side_effect: SideEffect,
    idempotency: Idempotency,
    retry_safety: RetrySafety,
) -> ActionContract {
    ActionContract {
        id,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1_024,
        max_output_bytes: 1_024,
        timeout_ms: 1_000,
        idempotency,
        side_effect,
        retry_safety,
        required_capabilities: Box::<[Capability]>::from([]),
    }
}

fn do_node(index: u16, action_id: ActionId) -> CompiledNode {
    CompiledNode {
        id: step(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: action_id,
            input: slot(0),
        },
    }
}

fn nop_node(index: u16) -> CompiledNode {
    CompiledNode {
        id: step(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn workflow(nodes: Box<[CompiledNode]>) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("idempotency-contract-red"),
        digest: WorkflowDigest::from_bytes([0x51; 32]),
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn retry_unsafe_violation(action_id: ActionId) -> IdempotencyContractViolation {
    IdempotencyContractViolation::SideEffectingRetryUnsafe {
        action: action_id,
        side_effect: SideEffect::Destroys,
        idempotency: Idempotency::IdempotentExternal,
        retry_safety: RetrySafety::Unsafe,
    }
}

fn at_least_once_violation(action_id: ActionId) -> IdempotencyContractViolation {
    IdempotencyContractViolation::SideEffectingAtLeastOnceExternal {
        action: action_id,
        side_effect: SideEffect::Creates,
        idempotency: Idempotency::AtLeastOnceExternal,
        retry_safety: RetrySafety::Safe,
    }
}

fn deterministic_pure_violation(action_id: ActionId) -> IdempotencyContractViolation {
    IdempotencyContractViolation::SideEffectingDeterministicPure {
        action: action_id,
        side_effect: SideEffect::Writes,
        idempotency: Idempotency::DeterministicPure,
        retry_safety: RetrySafety::Safe,
    }
}

#[test]
fn validate_action_returns_unit_for_pure_deterministic_safe_contract() {
    let candidate = contract(
        action(1),
        SideEffect::None,
        Idempotency::DeterministicPure,
        RetrySafety::Safe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_unit_for_pure_at_least_once_unsafe_contract() {
    let candidate = contract(
        action(2),
        SideEffect::None,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Unsafe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_unit_for_side_effecting_idempotent_external_safe_contract() {
    let candidate = contract(
        action(3),
        SideEffect::Writes,
        Idempotency::IdempotentExternal,
        RetrySafety::Safe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_unit_for_side_effecting_idempotent_external_key_required_contract() {
    let candidate = contract(
        action(4),
        SideEffect::Sends,
        Idempotency::IdempotentExternal,
        RetrySafety::KeyRequired,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe() {
    let action_id = action(5);
    let candidate = contract(
        action_id,
        SideEffect::Destroys,
        Idempotency::IdempotentExternal,
        RetrySafety::Unsafe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Err(retry_unsafe_violation(action_id)));
}

#[test]
fn validate_action_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once()
 {
    let action_id = action(6);
    let candidate = contract(
        action_id,
        SideEffect::Creates,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Safe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Err(at_least_once_violation(action_id)));
}

#[test]
fn validate_action_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()
 {
    let action_id = action(7);
    let candidate = contract(
        action_id,
        SideEffect::Writes,
        Idempotency::DeterministicPure,
        RetrySafety::Safe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Err(deterministic_pure_violation(action_id)));
}

#[test]
fn collect_returns_unit_for_empty_contract_slice() {
    let contracts: [ActionContract; 0] = [];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn collect_returns_unit_for_all_legal_contracts() {
    let contracts = [
        contract(
            action(8),
            SideEffect::None,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Unsafe,
        ),
        contract(
            action(9),
            SideEffect::Writes,
            Idempotency::IdempotentExternal,
            RetrySafety::Safe,
        ),
        contract(
            action(10),
            SideEffect::Sends,
            Idempotency::IdempotentExternal,
            RetrySafety::KeyRequired,
        ),
    ];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn collect_returns_one_boxed_retry_unsafe_violation_for_single_illegal_contract() {
    let action_id = action(11);
    let contracts = [contract(
        action_id,
        SideEffect::Destroys,
        Idempotency::IdempotentExternal,
        RetrySafety::Unsafe,
    )];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            retry_unsafe_violation(action_id,)
        ])))
    );
}

#[test]
fn collect_returns_one_boxed_at_least_once_violation_for_single_illegal_contract() {
    let action_id = action(12);
    let contracts = [contract(
        action_id,
        SideEffect::Creates,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Safe,
    )];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            at_least_once_violation(action_id),
        ])))
    );
}

#[test]
fn collect_returns_one_boxed_deterministic_pure_violation_for_single_illegal_contract() {
    let action_id = action(13);
    let contracts = [contract(
        action_id,
        SideEffect::Writes,
        Idempotency::DeterministicPure,
        RetrySafety::Safe,
    )];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            deterministic_pure_violation(action_id),
        ])))
    );
}

#[test]
fn collect_returns_all_boxed_violations_in_input_order_for_multiple_illegal_contracts() {
    let first = action(14);
    let legal = action(15);
    let second = action(16);
    let third = action(17);
    let contracts = [
        contract(
            first,
            SideEffect::Destroys,
            Idempotency::IdempotentExternal,
            RetrySafety::Unsafe,
        ),
        contract(
            legal,
            SideEffect::Sends,
            Idempotency::IdempotentExternal,
            RetrySafety::KeyRequired,
        ),
        contract(
            second,
            SideEffect::Creates,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Safe,
        ),
        contract(
            third,
            SideEffect::Writes,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        ),
    ];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            retry_unsafe_violation(first),
            at_least_once_violation(second),
            deterministic_pure_violation(third),
        ])))
    );
}

#[test]
fn collect_returns_same_boxed_violations_when_called_twice_with_same_input() {
    let contracts = [
        contract(
            action(18),
            SideEffect::Destroys,
            Idempotency::IdempotentExternal,
            RetrySafety::Unsafe,
        ),
        contract(
            action(19),
            SideEffect::Creates,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Safe,
        ),
        contract(
            action(20),
            SideEffect::Writes,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        ),
    ];

    let first = collect_idempotency_contract_violations(&contracts);
    let second = collect_idempotency_contract_violations(&contracts);

    assert_eq!(first, second);
}

#[test]
fn is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values() {
    let deterministic_safe = contract(
        action(21),
        SideEffect::None,
        Idempotency::DeterministicPure,
        RetrySafety::Safe,
    );
    let deterministic_key = contract(
        action(22),
        SideEffect::None,
        Idempotency::DeterministicPure,
        RetrySafety::KeyRequired,
    );
    let deterministic_unsafe = contract(
        action(23),
        SideEffect::None,
        Idempotency::DeterministicPure,
        RetrySafety::Unsafe,
    );
    let external_safe = contract(
        action(24),
        SideEffect::None,
        Idempotency::IdempotentExternal,
        RetrySafety::Safe,
    );
    let external_key = contract(
        action(25),
        SideEffect::None,
        Idempotency::IdempotentExternal,
        RetrySafety::KeyRequired,
    );
    let external_unsafe = contract(
        action(26),
        SideEffect::None,
        Idempotency::IdempotentExternal,
        RetrySafety::Unsafe,
    );
    let at_least_once_safe = contract(
        action(27),
        SideEffect::None,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Safe,
    );
    let at_least_once_key = contract(
        action(28),
        SideEffect::None,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::KeyRequired,
    );
    let at_least_once_unsafe = contract(
        action(29),
        SideEffect::None,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Unsafe,
    );

    let result = [
        is_statically_idempotent_contract(&deterministic_safe),
        is_statically_idempotent_contract(&deterministic_key),
        is_statically_idempotent_contract(&deterministic_unsafe),
        is_statically_idempotent_contract(&external_safe),
        is_statically_idempotent_contract(&external_key),
        is_statically_idempotent_contract(&external_unsafe),
        is_statically_idempotent_contract(&at_least_once_safe),
        is_statically_idempotent_contract(&at_least_once_key),
        is_statically_idempotent_contract(&at_least_once_unsafe),
    ];

    assert_eq!(
        result,
        [
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(())
        ]
    );
}

#[test]
fn is_static_returns_unit_for_side_effecting_idempotent_external_safe_contract() {
    let candidate = contract(
        action(30),
        SideEffect::Writes,
        Idempotency::IdempotentExternal,
        RetrySafety::Safe,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn is_static_returns_unit_for_side_effecting_idempotent_external_key_required_contract() {
    let candidate = contract(
        action(31),
        SideEffect::Sends,
        Idempotency::IdempotentExternal,
        RetrySafety::KeyRequired,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn is_static_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe() {
    let action_id = action(32);
    let candidate = contract(
        action_id,
        SideEffect::Destroys,
        Idempotency::IdempotentExternal,
        RetrySafety::Unsafe,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Err(retry_unsafe_violation(action_id)));
}

#[test]
fn is_static_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once() {
    let action_id = action(33);
    let candidate = contract(
        action_id,
        SideEffect::Creates,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Safe,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Err(at_least_once_violation(action_id)));
}

#[test]
fn is_static_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()
 {
    let action_id = action(34);
    let candidate = contract(
        action_id,
        SideEffect::Writes,
        Idempotency::DeterministicPure,
        RetrySafety::Safe,
    );

    let result = is_statically_idempotent_contract(&candidate);
    assert_eq!(result, Err(deterministic_pure_violation(action_id)));
}

#[test]
fn runtime_returns_missing_key_when_key_required_action_has_empty_key_slots()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = contract(
        action(35),
        SideEffect::Sends,
        Idempotency::IdempotentExternal,
        RetrySafety::KeyRequired,
    );
    let frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;

    let result = verify_idempotency(&candidate, &[], &frame);

    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::Sends))
    );
    Ok(())
}

#[test]
fn runtime_returns_secret_in_key_when_key_slot_taint_is_secret()
-> Result<(), Box<dyn std::error::Error>> {
    let mut frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;
    frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::Bool(true), Taint::Secret)?;

    let result = validate_idempotency_key_ingredients(&[SlotIdx::ZERO], &frame);

    assert_eq!(result, Err(IdempotencyViolation::SecretInKey(0)));
    Ok(())
}

#[test]
fn runtime_returns_secret_in_key_when_key_slot_taint_is_derived_from_secret()
-> Result<(), Box<dyn std::error::Error>> {
    let mut frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;
    frame.write_slot_with_taint(
        SlotIdx::ZERO,
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )?;

    let result = validate_idempotency_key_ingredients(&[SlotIdx::ZERO], &frame);

    assert_eq!(result, Err(IdempotencyViolation::SecretInKey(0)));
    Ok(())
}

#[test]
fn runtime_returns_unit_when_key_required_action_has_non_empty_clean_key_slots()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = contract(
        action(36),
        SideEffect::Writes,
        Idempotency::IdempotentExternal,
        RetrySafety::KeyRequired,
    );
    let mut frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;
    frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(true))?;

    let result = verify_idempotency(&candidate, &[SlotIdx::ZERO], &frame);

    assert_eq!(result, Ok(()));
    Ok(())
}

#[test]
fn static_verifier_ignores_zero_numeric_ticket_key_when_contract_is_key_required() {
    let candidate = contract(
        action(37),
        SideEffect::Sends,
        Idempotency::IdempotentExternal,
        RetrySafety::KeyRequired,
    );
    let ticket = ActionTicket {
        run: RunId::ZERO,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: action(37),
        attempt: 1,
        idempotency_key: 0,
        capacity: 3,
    };

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(ticket.idempotency_key, 0);
    assert_eq!(result, Ok(()));
}

#[test]
fn direct_decision_table_has_no_uncovered_enum_combination() {
    let accepted = [
        is_statically_idempotent_contract(&contract(
            action(38),
            SideEffect::None,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )),
        is_statically_idempotent_contract(&contract(
            action(39),
            SideEffect::Writes,
            Idempotency::IdempotentExternal,
            RetrySafety::Safe,
        )),
        is_statically_idempotent_contract(&contract(
            action(40),
            SideEffect::Sends,
            Idempotency::IdempotentExternal,
            RetrySafety::KeyRequired,
        )),
    ];
    let rejected = [
        is_statically_idempotent_contract(&contract(
            action(41),
            SideEffect::Destroys,
            Idempotency::IdempotentExternal,
            RetrySafety::Unsafe,
        )),
        is_statically_idempotent_contract(&contract(
            action(42),
            SideEffect::Creates,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Safe,
        )),
        is_statically_idempotent_contract(&contract(
            action(43),
            SideEffect::Writes,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )),
    ];

    assert_eq!(accepted, [Ok(()), Ok(()), Ok(())]);
    assert_eq!(
        rejected,
        [
            Err(retry_unsafe_violation(action(41))),
            Err(at_least_once_violation(action(42))),
            Err(deterministic_pure_violation(action(43))),
        ]
    );
}

#[test]
fn verifier_unit_functions_do_not_mutate_contract_values() {
    let candidate = contract(
        action(44),
        SideEffect::Writes,
        Idempotency::IdempotentExternal,
        RetrySafety::Safe,
    );
    let original = candidate.clone();

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
    assert_eq!(candidate, original);
}

#[test]
fn validate_workflow_returns_unit_when_workflow_has_no_do_nodes_and_registry_is_empty() {
    let parts = workflow(Box::from([nop_node(0)]));
    let contracts: [ActionContract; 0] = [];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_retry_safe() {
    let action_id = action(45);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts = [contract(
        action_id,
        SideEffect::Writes,
        Idempotency::IdempotentExternal,
        RetrySafety::Safe,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_key_required() {
    let action_id = action(46);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts = [contract(
        action_id,
        SideEffect::Sends,
        Idempotency::IdempotentExternal,
        RetrySafety::KeyRequired,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_returns_retry_unsafe_error_when_side_effecting_contract_is_retry_unsafe() {
    let action_id = action(47);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts = [contract(
        action_id,
        SideEffect::Destroys,
        Idempotency::IdempotentExternal,
        RetrySafety::Unsafe,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractError::IdempotencyViolations(
            IdempotencyContractErrors(Box::from([retry_unsafe_violation(action_id)]))
        ))
    );
}

#[test]
fn validate_workflow_returns_action_contract_missing_when_do_node_has_no_matching_contract() {
    let action_id = action(48);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts: [ActionContract; 0] = [];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractError::ActionContractMissing {
            action_id,
            node_index: 0,
        })
    );
}

#[test]
fn validate_workflow_returns_action_contract_orphan_when_registry_contract_has_no_do_node() {
    let action_id = action(49);
    let parts = workflow(Box::from([nop_node(0)]));
    let contracts = [contract(
        action_id,
        SideEffect::Writes,
        Idempotency::IdempotentExternal,
        RetrySafety::Safe,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractError::ActionContractOrphan { action_id })
    );
}

proptest! {
    #[test]
    fn proptest_pure_action_acceptance_holds_for_representative_action_ids(action_raw in 0u16..128u16) {
        let candidate = contract(
            ActionId::new(action_raw),
            SideEffect::None,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Unsafe,
        );

        let result = is_statically_idempotent_contract(&candidate);

        prop_assert_eq!(result, Ok(()));
    }

    #[test]
    fn proptest_retry_unsafe_side_effecting_contracts_report_original_action(action_raw in 0u16..128u16) {
        let action_id = ActionId::new(action_raw);
        let candidate = contract(
            action_id,
            SideEffect::Destroys,
            Idempotency::IdempotentExternal,
            RetrySafety::Unsafe,
        );

        let result = is_statically_idempotent_contract(&candidate);

        prop_assert_eq!(result, Err(retry_unsafe_violation(action_id)));
    }
}

proptest! {
    #[test]
    fn proptest_001_decision_table_confluence_10k(
        side_effect in prop_oneof![
            Just(SideEffect::None),
            Just(SideEffect::Writes),
            Just(SideEffect::Sends),
            Just(SideEffect::Creates),
            Just(SideEffect::Destroys),
        ],
        retry_safety in prop_oneof![
            Just(RetrySafety::Safe),
            Just(RetrySafety::KeyRequired),
            Just(RetrySafety::Unsafe),
        ],
        idempotency in prop_oneof![
            Just(Idempotency::DeterministicPure),
            Just(Idempotency::IdempotentExternal),
            Just(Idempotency::AtLeastOnceExternal),
        ],
    ) {
        let candidate = contract(ActionId::new(0), side_effect, idempotency, retry_safety);
        let result1 = is_statically_idempotent_contract(&candidate);
        let result2 = is_statically_idempotent_contract(&candidate);
        prop_assert_eq!(
            result1.is_ok(), result2.is_ok(),
            "Decision table must be confluent"
        );
    }
}

proptest! {
    #[test]
    fn proptest_002_runtime_gate_determinism_10k(
        side_effect in prop_oneof![
            Just(SideEffect::None),
            Just(SideEffect::Writes),
            Just(SideEffect::Sends),
        ],
        idempotency in prop_oneof![
            Just(Idempotency::IdempotentExternal),
            Just(Idempotency::AtLeastOnceExternal),
        ],
        key_count in 0..8u8,
        taint_pattern in 0..=255u8,
    ) {
        let candidate = contract(ActionId::new(0), side_effect, idempotency, RetrySafety::KeyRequired);
        let key_slots: Vec<SlotIdx> = (0..key_count as u16).map(SlotIdx::new).collect();

        let mut frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 8, 8)
            .unwrap();
        for (i, &slot_idx) in key_slots.iter().enumerate() {
            let taint_bit = (taint_pattern >> i) & 1;
            let taint = if taint_bit == 0 { Taint::Clean } else { Taint::Secret };
            let _ = frame.write_slot_with_taint(slot_idx, SlotValue::I64(i as i64), taint);
        }

        let result1 = verify_idempotency(&candidate, &key_slots, &frame);
        let result2 = verify_idempotency(&candidate, &key_slots, &frame);

        prop_assert_eq!(
            result1.is_ok(), result2.is_ok(),
            "Runtime gate must be deterministic"
        );
    }
}
