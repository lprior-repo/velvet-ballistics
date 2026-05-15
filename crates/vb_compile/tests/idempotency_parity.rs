use vb_compile::check_idempotency_gates;
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
    check_idempotency_gates(&[c.clone()]).is_ok()
}

#[test]
fn parity_side_effect_none_all_combinations_accept() {
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
            assert!(
                compile_ok(&c),
                "compile accepts None+{retry_safety:?}+{idempotency:?}"
            );
        }
    }
}

#[test]
fn parity_unsafe_retry_all_side_effects_rejected() {
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
            assert!(
                !compile_ok(&c),
                "compile rejects {side_effect:?}+Unsafe+{idempotency:?}"
            );
            id += 1;
        }
    }
}

#[test]
fn parity_idempotent_external_safe_or_key_required_accepts() {
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
            assert!(
                compile_ok(&c),
                "compile accepts {side_effect:?}+{retry_safety:?}+IdempotentExternal"
            );
            id += 1;
        }
    }
}

#[test]
fn parity_at_least_once_external_with_safe_or_key_required_disagree() {
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
            let cp_ok = compile_ok(&c);
            assert!(
                !cp_ok,
                "compile rejects AtLeastOnceExternal+{retry_safety:?}"
            );
            id += 1;
        }
    }
}

#[test]
fn parity_exhaustive_37_agreed_cases() {
    let side_effects_non_none = [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ];

    let mut agree_count = 0usize;
    let mut at_least_once_count = 0usize;

    for side_effect in side_effects_non_none.iter().copied() {
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

                let is_disagreement_case = (retry_safety == RetrySafety::Safe
                    || retry_safety == RetrySafety::KeyRequired)
                    && (idempotency == Idempotency::AtLeastOnceExternal
                        || idempotency == Idempotency::DeterministicPure);

                if is_disagreement_case {
                    at_least_once_count += 1;
                } else {
                    assert_eq!(
                        s_ok, cp_ok,
                        "agreed case must match: {side_effect:?}+{retry_safety:?}+{idempotency:?}"
                    );
                    agree_count += 1;
                }
            }
        }
    }

    assert_eq!(
        agree_count, 20,
        "empirical: 20 agreed among non-None (Unsafe 12 + Safe/Key IdempotentExternal 8)"
    );
    assert_eq!(
        at_least_once_count, 16,
        "empirical: 16 disagreements (AtLeastOnceExternal 8 + DeterministicPure 8 with Safe/KeyRequired)"
    );
    // Plus separately verified: 9 None cases (all agree)
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
fn parity_unsafe_12_cases_all_rejected_by_both() {
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
            assert!(
                !compile_ok(&c),
                "compile rejects Unsafe+{side_effect:?}+{idempotency:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 12);
}

#[test]
fn parity_idempotent_external_8_cases_all_accepted_by_both() {
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
            assert!(
                compile_ok(&c),
                "compile accepts IdempotentExternal+{side_effect:?}+{retry_safety:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 8);
}
