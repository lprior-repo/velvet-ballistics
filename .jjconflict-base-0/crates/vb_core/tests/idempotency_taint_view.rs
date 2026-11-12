use vb_core::action::{
    ActionContract, ActionName, Idempotency, IdempotencyViolation, RetrySafety, SideEffect,
    verify_idempotency,
};
use vb_core::{ActionId, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(7),
        name: ActionName::new("idempotency-view")?,
        input_slot_count: 5,
        output_slot_count: 1,
        max_input_bytes: 128,
        max_output_bytes: 128,
        timeout_ms: 1000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::new([]),
    })
}

fn frame() -> TestResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(98), StepIdx::new(1), 3, 5)?;
    frame.mark_succeeded(StepIdx::ZERO)?;
    frame.mark_running(StepIdx::new(1))?;
    frame.increment_executed()?;
    frame.set_max_parallel_in_flight(7);
    frame.add_parallel_in_flight(2)?;
    [
        Taint::Clean,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ]
    .into_iter()
    .zip(0_u16..)
    .try_for_each(|(taint, raw)| {
        frame.write_slot_with_taint(SlotIdx::new(raw), SlotValue::I64(42), taint)
    })?;
    Ok(frame)
}

fn check(
    frame: &RunFrame,
    contract: &ActionContract,
    slots: &[u16],
    expected: Result<(), IdempotencyViolation>,
) {
    let before = frame.checkpoint_v1();
    let keys: Vec<_> = slots.iter().copied().map(SlotIdx::new).collect();
    assert_eq!(verify_idempotency(contract, &keys, frame), expected);
    assert_eq!(frame.checkpoint_v1(), before);
}

#[test]
fn public_idempotency_preserves_all_taint_outcomes() -> TestResult {
    let contract = contract()?;
    let mut frame = frame()?;
    [
        (Taint::Clean, Ok(())),
        (Taint::Secret, Err(IdempotencyViolation::SecretInKey(0))),
        (
            Taint::DerivedFromSecret,
            Err(IdempotencyViolation::SecretInKey(0)),
        ),
        (Taint::Random, Err(IdempotencyViolation::RandomInKey(0))),
        (
            Taint::TimeDependent,
            Err(IdempotencyViolation::TimeInKey(0)),
        ),
    ]
    .into_iter()
    .try_for_each(|(taint, expected)| -> TestResult {
        frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::Null, taint)?;
        check(&frame, &contract, &[0], expected);
        Ok(())
    })
}

#[test]
fn public_idempotency_reports_first_invalid_ingredient_and_exact_index() -> TestResult {
    let contract = contract()?;
    let frame = frame()?;
    [
        (vec![0, 0], Ok(())),
        (vec![0, 1, 2], Err(IdempotencyViolation::SecretInKey(1))),
        (vec![2, 1], Err(IdempotencyViolation::RandomInKey(2))),
        (vec![3, 2], Err(IdempotencyViolation::TimeInKey(3))),
        (vec![4, 1], Err(IdempotencyViolation::UnavailableKeySlot(4))),
        (vec![1, 4], Err(IdempotencyViolation::SecretInKey(1))),
        (
            vec![u16::MAX, 1],
            Err(IdempotencyViolation::UnavailableKeySlot(u32::from(
                u16::MAX,
            ))),
        ),
        (vec![1, u16::MAX], Err(IdempotencyViolation::SecretInKey(1))),
    ]
    .into_iter()
    .for_each(|(slots, expected)| check(&frame, &contract, &slots, expected));
    Ok(())
}

#[test]
fn public_idempotency_preserves_policy_precedence_and_empty_key_errors() -> TestResult {
    let frame = frame()?;
    let mut contract = contract()?;
    let missing = Err(IdempotencyViolation::MissingKey(SideEffect::Writes));
    check(&frame, &contract, &[], missing.clone());
    contract.retry_safety = RetrySafety::Unsafe;
    check(&frame, &contract, &[0], missing.clone());
    check(&frame, &contract, &[u16::MAX], missing);
    contract.retry_safety = RetrySafety::Safe;
    check(&frame, &contract, &[], Ok(()));
    check(&frame, &contract, &[1, u16::MAX], Ok(()));
    contract.side_effect = SideEffect::None;
    contract.retry_safety = RetrySafety::Unsafe;
    check(&frame, &contract, &[], Ok(()));
    check(&frame, &contract, &[1, u16::MAX], Ok(()));
    Ok(())
}
