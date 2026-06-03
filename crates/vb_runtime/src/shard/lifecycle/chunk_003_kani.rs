// =============================================================================
// Kani proof: taint guard core logic (pure-function verification)
// =============================================================================
//
// The full end-to-end Kani harness through reject_taint_downgrade is heavy
// (RunState, CompiledWorkflow, ValueStore construction). We extract the
// guard's core decision into a pure function verified here, and cover the
// integration path via proptest in workspace_tests.
//
// GOD RULE 1: All inputs generated via kani::any() with assume guards.

#[cfg(kani)]
mod kani_taint_guard {
    use vb_core::action::Idempotency;
    use vb_core::value::Taint;

    /// Generates a valid `Taint` variant from an arbitrary u8.
    fn any_taint() -> Taint {
        let raw: u8 = kani::any();
        kani::assume(raw <= 4);
        match raw {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            2 => Taint::Secret,
            3 => Taint::Random,
            4 => Taint::TimeDependent,
            _ => Taint::Clean, // unreachable due to assume
        }
    }

    /// Generates a valid `Idempotency` variant from an arbitrary u8.
    fn any_idempotency() -> Idempotency {
        let raw: u8 = kani::any();
        kani::assume(raw <= 2);
        match raw {
            0 => Idempotency::DeterministicPure,
            1 => Idempotency::IdempotentExternal,
            2 => Idempotency::AtLeastOnceExternal,
            _ => Idempotency::DeterministicPure,
        }
    }

    /// Pure extraction of the guard decision:
    ///   `should_reject(idem, input_taint) -> Option<reason>`
    ///
    /// This mirrors the logic in `reject_taint_downgrade` lines 134-143
    /// without requiring RunState, frame, or workflow construction.
    #[must_use]
    fn guard_decision(idempotency: Idempotency, input_taint: Taint) -> Option<Taint> {
        if idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean {
            Some(Taint::Clean) // guard fires: required = Clean
        } else {
            None // guard does not fire
        }
    }

    /// Panic-freedom: guard_decision must not panic for any valid input.
    #[kani::proof]
    #[kani::unwind(2)]
    fn guard_decision_panic_free() {
        let input_taint = any_taint();
        let idempotency = any_idempotency();
        let _result: Option<Taint> = guard_decision(idempotency, input_taint);
    }

    /// Invariant: guard fires iff idempotency=DeterministicPure AND input_taint!=Clean.
    #[kani::proof]
    #[kani::unwind(2)]
    fn guard_fires_exactly_for_non_clean_deterministicpure() {
        let input_taint = any_taint();
        let idempotency = any_idempotency();
        let result = guard_decision(idempotency, input_taint);

        let expected_fires =
            idempotency == Idempotency::DeterministicPure && input_taint != Taint::Clean;

        if expected_fires {
            assert_eq!(
                result,
                Some(Taint::Clean),
                "guard must fire with required=Clean for DeterministicPure + non-Clean input"
            );
        } else {
            assert_eq!(
                result, None,
                "guard must NOT fire when idempotency≠DeterministicPure or input=Clean"
            );
        }
    }

    /// DeterministicPure guard fires for every non-Clean taint variant.
    #[kani::proof]
    #[kani::unwind(2)]
    fn every_non_clean_taint_triggers_guard_for_deterministicpure() {
        let input_taint = any_taint();
        kani::assume(input_taint != Taint::Clean);
        let result = guard_decision(Idempotency::DeterministicPure, input_taint);
        assert_eq!(
            result,
            Some(Taint::Clean),
            "DeterministicPure + non-Clean input must always fire the guard"
        );
    }

    /// Clean input never triggers the guard, regardless of idempotency.
    #[kani::proof]
    #[kani::unwind(2)]
    fn clean_input_never_triggers_guard() {
        let idempotency = any_idempotency();
        let result = guard_decision(idempotency, Taint::Clean);
        assert_eq!(
            result, None,
            "Clean input must never trigger the guard for any idempotency"
        );
    }

    /// Non-DeterministicPure idempotency levels never trigger the guard.
    #[kani::proof]
    #[kani::unwind(2)]
    fn non_deterministicpure_never_triggers_guard() {
        let input_taint = any_taint();
        let idempotency = any_idempotency();
        kani::assume(idempotency != Idempotency::DeterministicPure);
        let result = guard_decision(idempotency, input_taint);
        assert_eq!(
            result, None,
            "Non-DeterministicPure idempotency must never trigger the DeterministicPure guard"
        );
    }
}
