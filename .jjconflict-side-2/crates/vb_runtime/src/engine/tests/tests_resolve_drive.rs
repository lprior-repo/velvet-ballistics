// =====================================================================
// Resolve contract tests
// =====================================================================

#[test]
fn resolve_contract_returns_unknown_action_for_empty_contracts() {
    let contracts: Vec<vb_core::action::ActionContract> = Vec::new();
    let result = resolve_contract(ActionId::new(0), &contracts);
    assert_eq!(
        result,
        Err(RuntimeEngineError::Action(
            vb_core::action::ActionError::UnknownAction {
                action: ActionId::new(0),
            }
        ))
    );
}

// =====================================================================
// Drive loop tests
// =====================================================================

#[test]
fn drive_deterministic_budget_zero_returns_step_budget_exhausted() {
    let node = vb_core::workflow::node(StepIdx::ZERO, CompiledNodeKind::Nop, None, None);
    let parts = vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("nop"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
        step_names: Box::from([]),
    };
    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(_) => return,
    };
    let mut run = match RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 2) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut store = ValueStore::new();
    let mut budget = vb_core::engine::StepBudget::new(0);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = crate::primitives::collect::CollectStates::new();
    let result = drive_deterministic_full(
        &workflow,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::StepBudgetExhausted));
}

