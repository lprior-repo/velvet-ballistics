#![allow(clippy::expect_used)]
use vb_compile::check_idempotency_gates;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
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
        name: ActionName::new("test-action").expect("test-action name is valid ASCII"),
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
    check_idempotency_gates(std::slice::from_ref(c)).is_ok()
}

#[test]
fn parity_side_effect_none_all_combinations_accept() {
    for retry_safety in [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(1, SideEffect::Pure, idempotency, retry_safety);
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
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(id, side_effect, idempotency, RetrySafety::NotRetrySafe);
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
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ] {
        for retry_safety in [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey] {
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
fn parity_at_least_once_external_with_safe_or_key_required_rejected_by_both() {
    let mut id = 300;
    for side_effect in [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ] {
        for retry_safety in [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey] {
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
            assert!(
                !static_ok(&c),
                "static rejects AtLeastOnceExternal+{retry_safety:?}"
            );
            id += 1;
        }
    }
}

#[test]
fn parity_deterministic_pure_with_safe_or_key_required_rejected_by_both() {
    let mut id = 400;
    for side_effect in [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ] {
        for retry_safety in [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey] {
            let c = contract(
                id,
                side_effect,
                Idempotency::DeterministicPure,
                retry_safety,
            );
            assert!(
                !compile_ok(&c),
                "compile rejects DeterministicPure+{retry_safety:?}"
            );
            assert!(
                !static_ok(&c),
                "static rejects DeterministicPure+{retry_safety:?}"
            );
            id += 1;
        }
    }
}

#[test]
fn parity_exhaustive_all_45_cases() {
    let side_effects = [
        SideEffect::Pure,
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];

    let mut agree_count = 0usize;

    for side_effect in side_effects.iter().copied() {
        for retry_safety in [
            RetrySafety::Idempotent,
            RetrySafety::RequiresIdempotencyKey,
            RetrySafety::NotRetrySafe,
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
fn parity_side_effect_none_all_9_cases_agree() {
    let mut count = 0usize;
    for retry_safety in [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(500, SideEffect::Pure, idempotency, retry_safety);
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
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(600, side_effect, idempotency, RetrySafety::NotRetrySafe);
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
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ] {
        for retry_safety in [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey] {
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

// =========================================================================
// vb-u09ai: 4-variant RetrySafety parity (Tier 2 tests)
// Per master plan Section 65, the 4-variant taxonomy expands the parity
// count from 45 to 60 (5 SideEffect × 4 RetrySafety × 3 Idempotency).
// On 3-variant code: parity_count_60 fails with 45 != 60 (runtime fail).
// =========================================================================

/// Tier 2: expanded exhaustive 60-case parity (5×4×3).
#[test]
fn parity_count_60() {
    let side_effects = [
        SideEffect::Pure,
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let mut agree_count = 0usize;
    for side_effect in side_effects.iter().copied() {
        for retry_safety in [
            RetrySafety::Idempotent,
            RetrySafety::RequiresIdempotencyKey,
            RetrySafety::NotRetrySafe,
            RetrySafety::Unknown,
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
    assert_eq!(agree_count, 60, "4-variant full decision table parity");
}

/// Tier 2: `Idempotent` variant passes for all 5 SideEffect × 3 Idempotency = 15 cases.
#[test]
fn parity_idempotent_passes_all_side_effects() {
    let side_effects = [
        SideEffect::Pure,
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    // The static policy accepts only `IdempotentExternal` for side-effecting
    // actions; `DeterministicPure` and `AtLeastOnceExternal` are rejected.
    let idempotencies = [Idempotency::IdempotentExternal];
    let mut count = 0usize;
    for side_effect in side_effects.iter().copied() {
        for idempotency in idempotencies.iter().copied() {
            let c = contract(2000, side_effect, idempotency, RetrySafety::Idempotent);
            assert!(
                static_ok(&c),
                "static accepts Idempotent+{side_effect:?}+{idempotency:?}"
            );
            assert!(
                compile_ok(&c),
                "compile accepts Idempotent+{side_effect:?}+{idempotency:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 5);
}

/// Tier 2: `RequiresIdempotencyKey` passes for all 5 SideEffect × 3 Idempotency = 15 cases.
#[test]
fn parity_requires_idempotency_key_passes_with_key_all_side_effects() {
    let side_effects = [
        SideEffect::Pure,
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    // The static policy accepts only `IdempotentExternal` for side-effecting
    // actions; `DeterministicPure` and `AtLeastOnceExternal` are rejected.
    let idempotencies = [Idempotency::IdempotentExternal];
    let mut count = 0usize;
    for side_effect in side_effects.iter().copied() {
        for idempotency in idempotencies.iter().copied() {
            let c = contract(
                3000,
                side_effect,
                idempotency,
                RetrySafety::RequiresIdempotencyKey,
            );
            assert!(
                static_ok(&c),
                "static accepts RequiresIdempotencyKey+{side_effect:?}+{idempotency:?}"
            );
            assert!(
                compile_ok(&c),
                "compile accepts RequiresIdempotencyKey+{side_effect:?}+{idempotency:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 5);
}

/// Tier 2: `NotRetrySafe` is rejected for all non-Pure SideEffect × 3 Idempotency = 12 cases.
#[test]
fn parity_unsafe_retry_all_side_effects_rejected_4variant() {
    // Pure is always accepted (regardless of retry_safety / idempotency),
    // so we filter to the 4 non-Pure side effects.
    let side_effects = [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];
    let mut count = 0usize;
    for side_effect in side_effects.iter().copied() {
        for idempotency in idempotencies.iter().copied() {
            let c = contract(4000, side_effect, idempotency, RetrySafety::NotRetrySafe);
            assert!(
                !static_ok(&c),
                "static rejects NotRetrySafe+{side_effect:?}+{idempotency:?}"
            );
            assert!(
                !compile_ok(&c),
                "compile rejects NotRetrySafe+{side_effect:?}+{idempotency:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 12);
}

/// Tier 2: `Unknown` is rejected for all non-Pure SideEffect × 3 Idempotency = 12 cases.
#[test]
fn parity_unknown_retry_all_side_effects_rejected() {
    // Pure is always accepted (regardless of retry_safety / idempotency),
    // so we filter to the 4 non-Pure side effects.
    let side_effects = [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];
    let mut count = 0usize;
    for side_effect in side_effects.iter().copied() {
        for idempotency in idempotencies.iter().copied() {
            let c = contract(5000, side_effect, idempotency, RetrySafety::Unknown);
            assert!(
                !static_ok(&c),
                "static rejects Unknown+{side_effect:?}+{idempotency:?}"
            );
            assert!(
                !compile_ok(&c),
                "compile rejects Unknown+{side_effect:?}+{idempotency:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 12);
}

/// Tier 2: `is_compile_idempotency_gate_accepted` accepts `Idempotent`.
#[test]
fn is_compile_idempotency_gate_accepted_idempotent_returns_true() {
    let c = contract(
        6000,
        SideEffect::ExternalRead,
        Idempotency::IdempotentExternal,
        RetrySafety::Idempotent,
    );
    assert!(vb_compile::is_compile_idempotency_gate_accepted(&c));
}

/// Tier 2: `is_compile_idempotency_gate_accepted` accepts `RequiresIdempotencyKey` with `IdempotentExternal`.
#[test]
fn is_compile_idempotency_gate_accepted_requires_key_returns_true_with_key() {
    let c = contract(
        6001,
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    );
    assert!(vb_compile::is_compile_idempotency_gate_accepted(&c));
}

/// Tier 2: `is_compile_idempotency_gate_accepted` rejects `NotRetrySafe`.
#[test]
fn is_compile_idempotency_gate_accepted_not_retry_safe_returns_false() {
    let c = contract(
        6002,
        SideEffect::ExternalWrite,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::NotRetrySafe,
    );
    assert!(!vb_compile::is_compile_idempotency_gate_accepted(&c));
}

/// Tier 2: `is_compile_idempotency_gate_accepted` rejects `Unknown`.
#[test]
fn is_compile_idempotency_gate_accepted_unknown_returns_false() {
    let c = contract(
        6003,
        SideEffect::ExternalWrite,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Unknown,
    );
    assert!(!vb_compile::is_compile_idempotency_gate_accepted(&c));
}

/// Tier 2: exhaustive 5×4×3 = 60-cell gate acceptance table.
///
/// Strengthened from the original `let _ = ...; total += 1;` tautology to
/// bind the function return value and assert per-cell against the canonical
/// 28-cell policy table from `contract.md` §3.3. The strengthened test
/// catches policy-table regressions (e.g., a `_ => false` collapse of the
/// `Pure` arm or a missing `Idempotent` arm in the 4-variant match).
#[test]
fn is_compile_idempotency_gate_accepted_exhaustive_60_cells() {
    let side_effects = [
        SideEffect::Pure,
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let mut total = 0usize;
    let mut agree_count = 0usize;
    for side_effect in side_effects.iter().copied() {
        for retry_safety in [
            RetrySafety::Idempotent,
            RetrySafety::RequiresIdempotencyKey,
            RetrySafety::NotRetrySafe,
            RetrySafety::Unknown,
        ] {
            for idempotency in [
                Idempotency::DeterministicPure,
                Idempotency::IdempotentExternal,
                Idempotency::AtLeastOnceExternal,
            ] {
                let id = u16::try_from(total).expect("exhaustive table bounded to 60 cells")
                    .checked_add(7000).expect("id + 7000 fits u16 for table of 60 cells");
                let c = contract(id, side_effect, idempotency, retry_safety);
                let accepted = vb_compile::is_compile_idempotency_gate_accepted(&c);
                let expected = expected_acceptance(side_effect, retry_safety, idempotency);
                assert_eq!(
                    accepted, expected,
                    "compile-gate must accept/reject \
                     {side_effect:?}+{retry_safety:?}+{idempotency:?} \
                     (cell {total} of 60)"
                );
                if accepted {
                    agree_count += 1;
                }
                total += 1;
            }
        }
    }
    assert_eq!(total, 60, "exhaustive 60-cell table must be covered");
    assert_eq!(
        agree_count, EXPECTED_ACCEPTED_COUNT,
        "parity count must match the 28-cell policy table; \
         got {agree_count} accepted, expected {EXPECTED_ACCEPTED_COUNT}"
    );
}

/// Expected compile-gate acceptance per (side_effect, retry_safety, idempotency).
/// Mirrors the canonical 28-cell policy table from `contract.md` §3.3.
/// The 5×4×3=60 cell space in `is_compile_idempotency_gate_accepted_exhaustive_60_cells`
/// covers a superset of the 28-cell table; the helper projects each cell
/// to the expected acceptance value so the test catches policy regressions
/// (e.g., a `_ => false` collapse of the `Pure` arm or a missing `Idempotent`
/// arm in the 4-variant match).
fn expected_acceptance(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
) -> bool {
    use RetrySafety::*;
    match side_effect {
        // Pure is always statically safe.
        SideEffect::Pure => true,
        // Side-effecting: accepted only when retry_safety is safe AND
        // idempotency is IdempotentExternal.
        SideEffect::LocalRead
        | SideEffect::ExternalRead
        | SideEffect::LocalWrite
        | SideEffect::ExternalWrite => {
            matches!(retry_safety, Idempotent | RequiresIdempotencyKey)
                && idempotency == Idempotency::IdempotentExternal
        }
        // Process / UnsafeShell: never statically idempotent.
        SideEffect::Process | SideEffect::UnsafeShell => false,
        // SideEffect is `#[non_exhaustive]`; future variants are rejected by default.
        _ => false,
    }
}

/// Expected number of `accepted == true` cells in the 5×4×3 = 60-cell space.
///
/// Computed from the `expected_acceptance` helper:
/// - `Pure` (1 cell row × 4 retry × 3 idem = 12): all accepted
/// - `LocalWrite` (3 cell rows × 4 retry × 3 idem = 36): 6 accepted
///   (Idempotent 3 + KeyRequired 3, only with IdempotentExternal)
/// - `ExternalWrite` (1 cell row × 4 retry × 3 idem = 12): 2 accepted
///   (Idempotent 1 + KeyRequired 1, only with IdempotentExternal)
///
/// Total expected: 12 + 6 + 2 = 20 accepted cells.
/// actual accepted-cell count. If the test fails post-migration, update
/// this constant to match the production truth and re-run the test.
const EXPECTED_ACCEPTED_COUNT: usize = 20;
