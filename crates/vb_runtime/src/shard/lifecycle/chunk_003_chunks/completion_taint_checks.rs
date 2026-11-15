use super::validate_completion_taint;
use crate::RuntimeError;
use vb_core::action::Idempotency;
use vb_core::value::Taint;

const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];

fn required_rows() -> [(Idempotency, [Taint; 5]); 3] {
    [
        (Idempotency::DeterministicPure, TAINTS),
        (Idempotency::IdempotentExternal, TAINTS),
        (
            Idempotency::AtLeastOnceExternal,
            [
                Taint::Clean,
                Taint::DerivedFromSecret,
                Taint::DerivedFromSecret,
                Taint::DerivedFromSecret,
                Taint::DerivedFromSecret,
            ],
        ),
    ]
}

#[cfg(test)]
#[test]
fn completion_taint_guard_matches_all_75_current_tuples() {
    let mut checked = 0;
    for (idempotency, required) in required_rows() {
        for (input, (required_rank, required)) in
            TAINTS.into_iter().zip(required.into_iter().map(|required| {
                let rank = TAINTS.iter().position(|taint| *taint == required);
                (rank, required)
            }))
        {
            for (supplied_rank, supplied) in TAINTS.into_iter().enumerate() {
                let expected = if required_rank.is_some_and(|rank| supplied_rank >= rank) {
                    Ok(())
                } else {
                    Err(RuntimeError::ActionTaintDowngrade { required, supplied })
                };
                assert_eq!(
                    validate_completion_taint(idempotency, input, supplied),
                    expected,
                    "{idempotency:?}/{input:?}/{supplied:?}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 75);
}

#[cfg(kani)]
fn any_taint() -> Taint {
    match kani::any::<u8>() % 5 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        3 => Taint::Random,
        _ => Taint::TimeDependent,
    }
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(7)]
fn completion_taint_guard_exact_typed_domain() {
    let selected = kani::any::<u8>() % 3;
    let input = any_taint();
    let supplied = any_taint();
    let row = required_rows().into_iter().nth(usize::from(selected));
    let Some((idempotency, required)) = row else {
        kani::assert(false, "every generated idempotency has a required row");
        return;
    };
    let input_rank = TAINTS.iter().position(|taint| *taint == input);
    let expected = input_rank.and_then(|rank| required.get(rank).copied());
    let Some(required) = expected else {
        kani::assert(false, "every generated input has a required taint");
        return;
    };
    let supplied_rank = TAINTS.iter().position(|taint| *taint == supplied);
    let required_rank = TAINTS.iter().position(|taint| *taint == required);
    let permitted = supplied_rank
        .zip(required_rank)
        .is_some_and(|(s, r)| s >= r);
    let result = validate_completion_taint(idempotency, input, supplied);
    match &result {
        Ok(()) => kani::assert(permitted, "accept exactly permitted output taints"),
        Err(RuntimeError::ActionTaintDowngrade {
            required: actual_required,
            supplied: actual_supplied,
        }) => {
            kani::assert(!permitted, "reject exactly lower supplied taints");
            kani::assert(*actual_required == required, "exact required taint");
            kani::assert(*actual_supplied == supplied, "exact supplied taint");
        }
        Err(_) => kani::assert(false, "no other error variant belongs to the guard"),
    }
    kani::cover(result.is_ok(), "accepted tuple is reachable");
    kani::cover(result.is_err(), "rejected tuple is reachable");
    kani::cover(
        input == Taint::Random && supplied == Taint::Random && permitted,
        "Random is generated and accepted",
    );
    kani::cover(
        input == Taint::TimeDependent && supplied == Taint::Clean && !permitted,
        "TimeDependent is generated and can refuse downgrade",
    );
    kani::cover(selected == 0 && permitted, "DeterministicPure accepts");
    kani::cover(selected == 1 && !permitted, "IdempotentExternal refuses");
    kani::cover(selected == 2 && permitted, "AtLeastOnceExternal accepts");
    // Other RuntimeError variants own allocations; their destruction is outside this guard.
    std::mem::forget(result);
}
