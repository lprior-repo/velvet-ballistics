use vb_compile::{CompileError, CompileErrors, check_idempotency_gates};
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_validate::idempotency_contract::is_statically_idempotent_contract;

fn contract(
    id: u16,
    side_effect: SideEffect,
    idempotency: Idempotency,
    retry_safety: RetrySafety,
) -> ActionContract {
    ActionContract {
        id: ActionId::new(id),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency,
        side_effect,
        retry_safety,
        required_capabilities: Box::from([]),
    }
}

fn static_ok(c: &ActionContract) -> bool {
    is_statically_idempotent_contract(c).is_ok()
}

fn compile_ok(c: &ActionContract) -> bool {
    compile_result(c).is_ok()
}

fn compile_result(c: &ActionContract) -> Result<(), CompileErrors> {
    check_idempotency_gates(&[c.clone()])
}

fn assert_compile_accepts(c: &ActionContract, context: &str) -> Result<(), String> {
    compile_result(c).map_err(|errors| format!("compile should accept {context}, got {errors:?}"))
}

fn assert_compile_rejects_with_idempotency_violation(
    c: &ActionContract,
    expected_reason: &'static str,
    context: &str,
) -> Result<(), String> {
    match compile_result(c) {
        Err(CompileErrors(errors)) => match errors.as_slice() {
            [
                CompileError::IdempotencyViolation {
                    action,
                    side_effect,
                    reason,
                },
            ] => {
                assert_eq!(
                    (*action, *side_effect, reason.as_ref()),
                    (c.id, c.side_effect, expected_reason),
                    "compile IdempotencyViolation payload for {context}"
                );
                Ok(())
            }
            other => Err(format!(
                "compile should reject {context} with one IdempotencyViolation, got {other:?}"
            )),
        },
        Ok(()) => Err(format!(
            "compile should reject {context} with IdempotencyViolation"
        )),
    }
}

#[test]
fn parity_side_effect_none_all_combinations_accept() -> Result<(), String> {
    for retry_safety in [
        RetrySafety::Safe,
        RetrySafety::KeyRequired,
        RetrySafety::Unsafe,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(1, SideEffect::None, idempotency, retry_safety);
            assert!(
                static_ok(&c),
                "static accepts None+{retry_safety:?}+{idempotency:?}"
            );
            assert_compile_accepts(&c, &format!("None+{retry_safety:?}+{idempotency:?}"))?;
        }
    }
    Ok(())
}

#[test]
fn parity_unsafe_retry_all_side_effects_rejected() -> Result<(), String> {
    let mut id = 100;
    for side_effect in [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(id, side_effect, idempotency, RetrySafety::Unsafe);
            assert!(
                !static_ok(&c),
                "static rejects {side_effect:?}+Unsafe+{idempotency:?}"
            );
            assert_compile_rejects_with_idempotency_violation(
                &c,
                "side-effecting action declares RetrySafety::Unsafe",
                &format!("{side_effect:?}+Unsafe+{idempotency:?}"),
            )?;
            id += 1;
        }
    }
    Ok(())
}

#[test]
fn parity_idempotent_external_safe_or_key_required_accepts() -> Result<(), String> {
    let mut id = 200;
    for side_effect in [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for retry_safety in [RetrySafety::Safe, RetrySafety::KeyRequired] {
            let c = contract(
                id,
                side_effect,
                Idempotency::IdempotentExternal,
                retry_safety,
            );
            assert!(
                static_ok(&c),
                "static accepts {side_effect:?}+{retry_safety:?}+IdempotentExternal"
            );
            assert_compile_accepts(
                &c,
                &format!("{side_effect:?}+{retry_safety:?}+IdempotentExternal"),
            )?;
            id += 1;
        }
    }
    Ok(())
}

#[test]
fn parity_at_least_once_external_with_safe_or_key_required_rejected_by_both() -> Result<(), String>
{
    let mut id = 300;
    for side_effect in [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for retry_safety in [RetrySafety::Safe, RetrySafety::KeyRequired] {
            let c = contract(
                id,
                side_effect,
                Idempotency::AtLeastOnceExternal,
                retry_safety,
            );
            assert_compile_rejects_with_idempotency_violation(
                &c,
                "side-effecting action declares Idempotency::AtLeastOnceExternal without guaranteed idempotent retry",
                &format!("{side_effect:?}+{retry_safety:?}+AtLeastOnceExternal"),
            )?;
            assert!(
                !static_ok(&c),
                "static rejects AtLeastOnceExternal+{retry_safety:?}"
            );
            id += 1;
        }
    }
    Ok(())
}

#[test]
fn parity_deterministic_pure_with_safe_or_key_required_rejected_by_both() -> Result<(), String> {
    let mut id = 400;
    for side_effect in [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for retry_safety in [RetrySafety::Safe, RetrySafety::KeyRequired] {
            let c = contract(
                id,
                side_effect,
                Idempotency::DeterministicPure,
                retry_safety,
            );
            assert_compile_rejects_with_idempotency_violation(
                &c,
                "side-effecting action declares Idempotency::DeterministicPure",
                &format!("{side_effect:?}+{retry_safety:?}+DeterministicPure"),
            )?;
            assert!(
                !static_ok(&c),
                "static rejects DeterministicPure+{retry_safety:?}"
            );
            id += 1;
        }
    }
    Ok(())
}

#[test]
fn parity_exhaustive_all_45_cases() {
    let side_effects = [
        SideEffect::None,
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ];

    let mut agree_count = 0usize;

    for side_effect in side_effects.iter().copied() {
        for retry_safety in [
            RetrySafety::Safe,
            RetrySafety::KeyRequired,
            RetrySafety::Unsafe,
        ] {
            for idempotency in [
                Idempotency::DeterministicPure,
                Idempotency::IdempotentExternal,
                Idempotency::AtLeastOnceExternal,
            ] {
                let c = contract(1000, side_effect, idempotency, retry_safety);
                let s_ok = static_ok(&c);
                let cp_ok = compile_ok(&c);

                assert_eq!(
                    s_ok, cp_ok,
                    "all cases must match: {side_effect:?}+{retry_safety:?}+{idempotency:?}"
                );
                agree_count += 1;
            }
        }
    }

    assert_eq!(agree_count, 45, "full decision table parity");
}

#[test]
fn multi_contract_error_accumulation_ordering() {
    // Three contracts with distinct violations: Unsafe retry,
    // AtLeastOnceExternal, and DeterministicPure side-effects.
    // Each must produce an IdempotencyViolation, and they must
    // appear in the exact order of the input slice.
    let c_unsafe = contract(
        1001,
        SideEffect::Writes,
        Idempotency::DeterministicPure,
        RetrySafety::Unsafe,
    );
    let c_at_least_once = contract(
        2001,
        SideEffect::Sends,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Safe,
    );
    let c_deterministic = contract(
        3001,
        SideEffect::Destroys,
        Idempotency::DeterministicPure,
        RetrySafety::Safe,
    );

    let result = check_idempotency_gates(&[
        c_unsafe.clone(),
        c_at_least_once.clone(),
        c_deterministic.clone(),
    ]);

    let Err(CompileErrors(errors)) = result else {
        panic!("expected Err(CompileErrors([...])) with 3 violations");
    };

    assert_eq!(errors.len(), 3, "must produce exactly 3 errors in input order");

    // First error: Unsafe retry for writes action.
    match &errors[0] {
        CompileError::IdempotencyViolation {
            action,
            side_effect,
            reason,
        } => {
            assert_eq!(*action, c_unsafe.id);
            assert_eq!(*side_effect, SideEffect::Writes);
            assert_eq!(
                reason.as_ref(),
                "side-effecting action declares RetrySafety::Unsafe"
            );
        }
        other => panic!(
            "expected IdempotencyViolation for contract 1 (Unsafe), got {other:?}"
        ),
    }

    // Second error: AtLeastOnceExternal without guaranteed idempotent retry.
    match &errors[1] {
        CompileError::IdempotencyViolation {
            action,
            side_effect,
            reason,
        } => {
            assert_eq!(*action, c_at_least_once.id);
            assert_eq!(*side_effect, SideEffect::Sends);
            assert_eq!(
                reason.as_ref(),
                "side-effecting action declares Idempotency::AtLeastOnceExternal \
                 without guaranteed idempotent retry"
            );
        }
        other => panic!(
            "expected IdempotencyViolation for contract 2 (AtLeastOnceExternal), got {other:?}"
        ),
    }

    // Third error: DeterministicPure with side effects.
    match &errors[2] {
        CompileError::IdempotencyViolation {
            action,
            side_effect,
            reason,
        } => {
            assert_eq!(*action, c_deterministic.id);
            assert_eq!(*side_effect, SideEffect::Destroys);
            assert_eq!(
                reason.as_ref(),
                "side-effecting action declares Idempotency::DeterministicPure"
            );
        }
        other => panic!(
            "expected IdempotencyViolation for contract 3 (DeterministicPure), got {other:?}"
        ),
    }
}

#[test]
fn parity_side_effect_none_all_9_cases_agree() {
    let mut count = 0usize;
    for retry_safety in [
        RetrySafety::Safe,
        RetrySafety::KeyRequired,
        RetrySafety::Unsafe,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(500, SideEffect::None, idempotency, retry_safety);
            assert_eq!(
                static_ok(&c),
                compile_ok(&c),
                "None+{retry_safety:?}+{idempotency:?} must agree"
            );
            count += 1;
        }
    }
    assert_eq!(count, 9);
}

#[test]
fn parity_unsafe_12_cases_all_rejected_by_both() -> Result<(), String> {
    let mut count = 0usize;
    for side_effect in [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(600, side_effect, idempotency, RetrySafety::Unsafe);
            assert!(
                !static_ok(&c),
                "static rejects Unsafe+{side_effect:?}+{idempotency:?}"
            );
            assert_compile_rejects_with_idempotency_violation(
                &c,
                "side-effecting action declares RetrySafety::Unsafe",
                &format!("Unsafe+{side_effect:?}+{idempotency:?}"),
            )?;
            count += 1;
        }
    }
    assert_eq!(count, 12);
    Ok(())
}

#[test]
fn parity_idempotent_external_8_cases_all_accepted_by_both() -> Result<(), String> {
    let mut count = 0usize;
    for side_effect in [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for retry_safety in [RetrySafety::Safe, RetrySafety::KeyRequired] {
            let c = contract(
                700,
                side_effect,
                Idempotency::IdempotentExternal,
                retry_safety,
            );
            assert!(
                static_ok(&c),
                "static accepts IdempotentExternal+{side_effect:?}+{retry_safety:?}"
            );
            assert_compile_accepts(
                &c,
                &format!("IdempotentExternal+{side_effect:?}+{retry_safety:?}"),
            )?;
            count += 1;
        }
    }
    assert_eq!(count, 8);
    Ok(())
}
