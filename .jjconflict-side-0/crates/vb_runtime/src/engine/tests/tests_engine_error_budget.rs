// =====================================================================
// RuntimeEngineError tests
// =====================================================================

#[test]
fn runtime_engine_error_from_engine_error() {
    let core = EngineError::UnsupportedPrimitive {
        primitive: "test".into(),
    };
    let engine_err: RuntimeEngineError = core.into();
    assert_eq!(
        engine_err,
        RuntimeEngineError::Core(EngineError::UnsupportedPrimitive {
            primitive: "test".into()
        })
    );
}

#[test]
fn runtime_engine_error_from_action_error() {
    let action = vb_core::action::ActionError::UnknownAction {
        action: ActionId::new(5),
    };
    let engine_err: RuntimeEngineError = action.into();
    assert_eq!(
        engine_err,
        RuntimeEngineError::Action(vb_core::action::ActionError::UnknownAction {
            action: ActionId::new(5)
        })
    );
}

#[test]
fn runtime_engine_error_core_wraps_core_error() {
    let core_err = EngineError::UnsupportedPrimitive {
        primitive: "test_prim".into(),
    };
    let engine_err = RuntimeEngineError::Core(core_err.clone());
    assert_eq!(
        engine_err,
        RuntimeEngineError::Core(EngineError::UnsupportedPrimitive {
            primitive: "test_prim".into(),
        })
    );
}

#[test]
fn runtime_engine_error_action_wraps_action_error() {
    let action_err = vb_core::action::ActionError::UnknownAction {
        action: ActionId::new(7),
    };
    let engine_err = RuntimeEngineError::Action(action_err.clone());
    assert_eq!(
        engine_err,
        RuntimeEngineError::Action(vb_core::action::ActionError::UnknownAction {
            action: ActionId::new(7),
        })
    );
}

#[test]
fn runtime_engine_error_retry_exhausted_reports_action_and_attempts() {
    let err = RuntimeEngineError::RetryExhausted {
        action: ActionId::new(3),
        attempts: 5,
    };
    match err {
        RuntimeEngineError::RetryExhausted { action, attempts } => {
            assert_eq!(action, ActionId::new(3));
            assert_eq!(attempts, 5);
        }
        other => assert_eq!(
            other,
            RuntimeEngineError::RetryExhausted {
                action: ActionId::new(0),
                attempts: 0,
            }
        ),
    }
}

#[test]
fn runtime_engine_error_taint_violation_reports_step() {
    let err = RuntimeEngineError::TaintViolation {
        step: StepIdx::new(42),
    };
    match err {
        RuntimeEngineError::TaintViolation { step } => {
            assert_eq!(step, StepIdx::new(42));
        }
        other => assert_eq!(
            other,
            RuntimeEngineError::TaintViolation {
                step: StepIdx::new(0),
            }
        ),
    }
}

// =====================================================================
// Step budget tests
// =====================================================================

#[test]
fn step_budget_new_with_zero_allows_no_steps() {
    let mut budget = vb_core::engine::StepBudget::new(0);
    assert_eq!(budget.try_take(), Ok(false));
}

#[test]
fn step_budget_remaining_decreases_after_each_step() {
    let mut budget = vb_core::engine::StepBudget::new(3);
    assert_eq!(budget.try_take(), Ok(true));
    assert_eq!(budget.try_take(), Ok(true));
    assert_eq!(budget.try_take(), Ok(true));
    assert_eq!(budget.try_take(), Ok(false));
}

