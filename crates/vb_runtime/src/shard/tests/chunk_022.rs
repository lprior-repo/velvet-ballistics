
// =========================================================================
// Additional unit tests -- variant equality/inequality, debug format,
// PendingTimer edge cases, AskAnswer, and boundary RunId values
// =========================================================================

#[test]
fn shard_command_submit_inequality_different_run_id() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let a = ShardCommand::Submit {
        run: super::RunId::new(10),
        workflow: wf.clone(),
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    let b = ShardCommand::Submit {
        run: super::RunId::new(20),
        workflow: wf,
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_submit_with_inputs_equality() {
    let Some(wf) = finished_workflow() else {
        return;
    };
    let inputs: Box<[(SlotIdx, vb_core::value::SlotValue)]> =
        Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::I64(7))]);
    let a = ShardCommand::SubmitWithInputs {
        run: super::RunId::new(5),
        workflow: wf.clone(),
        inputs: inputs.clone(),
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    let b = ShardCommand::SubmitWithInputs {
        run: super::RunId::new(5),
        workflow: wf,
        inputs,
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    assert_eq!(a, b);
}

#[test]
fn shard_command_submit_with_inputs_inequality_different_inputs() {
    let Some(wf) = finished_workflow() else {
        return;
    };
    let a = ShardCommand::SubmitWithInputs {
        run: super::RunId::new(5),
        workflow: wf.clone(),
        inputs: Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::I64(1))]),
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    let b = ShardCommand::SubmitWithInputs {
        run: super::RunId::new(5),
        workflow: wf,
        inputs: Box::from([(SlotIdx::new(0), vb_core::value::SlotValue::I64(2))]),
        caps: vb_core::capability::CapabilitySet::empty(),
    };
    assert_ne!(a, b);
}

#[test]
fn shard_command_action_failed_equality() {
    let ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(3),
        step: vb_core::ids::StepIdx::new(1),
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let failure = vb_core::action::ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let a = ShardCommand::ActionFailed {
        ticket: ticket.clone(),
        failure: failure.clone(),
    };
    let b = ShardCommand::ActionFailed { ticket, failure };
    assert_eq!(a, b);
}

#[test]
fn shard_command_ask_answered_equality() {
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(4),
            ask_step: vb_core::ids::StepIdx::new(2),
            resume_step: vb_core::ids::StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    let a = ShardCommand::AskAnswered { answer };
    let answer2 = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(4),
            ask_step: vb_core::ids::StepIdx::new(2),
            resume_step: vb_core::ids::StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    let b = ShardCommand::AskAnswered { answer: answer2 };
    assert_eq!(a, b);
}

#[test]
fn shard_config_debug_format_contains_policy_field() {
    let config = ShardConfig::default();
    let debug_str = format!("{config:?}");
    assert!(
        debug_str.contains("policy"),
        "Debug output should contain policy: {debug_str}"
    );
}

#[test]
fn pending_timer_copy_trait_produces_independent_value() {
    let original = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(7),
        kind: super::types::PendingTimerKind::Ask,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let copy = original;
    assert_eq!(copy, original);
    assert_eq!(copy.step, vb_core::ids::StepIdx::new(7));
    assert_eq!(copy.kind, super::types::PendingTimerKind::Ask);
}

#[test]
fn pending_timer_debug_format() {
    let timer = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(4),
        kind: super::types::PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let debug_str = format!("{timer:?}");
    assert!(
        debug_str.contains("PendingTimer"),
        "Debug should contain PendingTimer: {debug_str}"
    );
}

#[test]
fn pending_timer_with_zero_step_index() {
    let timer = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::ZERO,
        kind: super::types::PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    assert_eq!(timer.step, vb_core::ids::StepIdx::ZERO);
    assert_eq!(timer.kind, super::types::PendingTimerKind::Wait);
}

#[test]
fn ask_answer_equality_same_fields() {
    let a = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(8),
            ask_step: vb_core::ids::StepIdx::new(1),
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        answer_slot: SlotIdx::new(3),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    let b = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(8),
            ask_step: vb_core::ids::StepIdx::new(1),
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        answer_slot: SlotIdx::new(3),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_eq!(a, b);
}

#[test]
fn ask_answer_inequality_different_taint() {
    let a = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(8),
            ask_step: vb_core::ids::StepIdx::new(1),
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        answer_slot: SlotIdx::new(3),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    let b = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(8),
            ask_step: vb_core::ids::StepIdx::new(1),
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        answer_slot: SlotIdx::new(3),
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Secret,
        encoded_len: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn ask_answer_inequality_different_value() {
    let a = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(8),
            ask_step: vb_core::ids::StepIdx::new(1),
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::Bool(true),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    let b = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(8),
            ask_step: vb_core::ids::StepIdx::new(1),
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::Bool(false),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn ask_answer_debug_format() {
    let answer = AskAnswer {
        ticket: AskTicket {
            run: super::RunId::new(9),
            ask_step: vb_core::ids::StepIdx::new(0),
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: vb_core::value::SlotValue::I64(0),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    let debug_str = format!("{answer:?}");
    assert!(
        debug_str.contains("AskAnswer"),
        "Debug should contain AskAnswer: {debug_str}"
    );
}

#[test]
fn shard_submit_with_run_id_zero_accepted() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(0);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
}
