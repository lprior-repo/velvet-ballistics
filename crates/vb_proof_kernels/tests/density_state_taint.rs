use vb_proof_kernels::step_state::{
    StepState, all_transitions_exhaustive, is_valid_transition, next_states, non_terminal_states,
    terminal_cannot_transition_to_non_terminal, terminal_states, validate_transition,
};
use vb_proof_kernels::taint::{
    Taint, all_lattice_laws, derived_never_downgrades, has_identity, is_associative,
    is_commutative, is_idempotent, join_many, join_taint, secret_never_downgrades,
};

#[test]
fn pending_is_not_terminal() {
    assert!(!StepState::Pending.is_terminal());
}

#[test]
fn running_is_not_terminal() {
    assert!(!StepState::Running.is_terminal());
}

#[test]
fn waiting_is_not_terminal() {
    assert!(!StepState::Waiting.is_terminal());
}

#[test]
fn asking_is_not_terminal() {
    assert!(!StepState::Asking.is_terminal());
}

#[test]
fn succeeded_is_terminal() {
    assert!(StepState::Succeeded.is_terminal());
}

#[test]
fn failed_is_terminal() {
    assert!(StepState::Failed.is_terminal());
}

#[test]
fn cancelled_is_terminal() {
    assert!(StepState::Cancelled.is_terminal());
}

#[test]
fn skipped_is_terminal() {
    assert!(StepState::Skipped.is_terminal());
}

#[test]
fn pending_can_mark_running() {
    assert!(is_valid_transition(StepState::Pending, StepState::Running));
}

#[test]
fn pending_can_mark_succeeded() {
    assert!(is_valid_transition(
        StepState::Pending,
        StepState::Succeeded
    ));
}

#[test]
fn running_can_wait() {
    assert!(is_valid_transition(StepState::Running, StepState::Waiting));
}

#[test]
fn running_can_ask() {
    assert!(is_valid_transition(StepState::Running, StepState::Asking));
}

#[test]
fn waiting_can_resume_running() {
    assert!(is_valid_transition(StepState::Waiting, StepState::Running));
}

#[test]
fn asking_can_resume_running() {
    assert!(is_valid_transition(StepState::Asking, StepState::Running));
}

#[test]
fn terminal_cannot_resume_running() {
    assert!(!is_valid_transition(
        StepState::Succeeded,
        StepState::Running
    ));
}

#[test]
fn running_cannot_return_to_pending() {
    assert!(!is_valid_transition(StepState::Running, StepState::Pending));
}

#[test]
fn validate_transition_returns_target_on_success() {
    assert_eq!(
        validate_transition(StepState::Pending, StepState::Running),
        Ok(StepState::Running)
    );
}

#[test]
fn validate_transition_returns_static_error_on_failure() {
    assert_eq!(
        validate_transition(StepState::Running, StepState::Pending),
        Err("invalid_state_transition")
    );
}

#[test]
fn next_states_for_waiting_are_self_and_running() {
    let next = next_states(StepState::Waiting);
    assert_eq!(next.len(), 2);
    assert!(next.contains(&StepState::Waiting));
    assert!(next.contains(&StepState::Running));
}

#[test]
fn terminal_states_are_exactly_four() {
    assert_eq!(terminal_states().len(), 4);
}

#[test]
fn non_terminal_states_are_exactly_four() {
    assert_eq!(non_terminal_states().len(), 4);
}

#[test]
fn terminal_absorption_property_holds() {
    assert!(terminal_cannot_transition_to_non_terminal());
}

#[test]
fn transition_partition_property_holds() {
    assert!(all_transitions_exhaustive());
}

#[test]
fn taint_clean_rank_is_zero() {
    assert_eq!(Taint::Clean.rank(), 0);
}

#[test]
fn taint_derived_rank_is_one() {
    assert_eq!(Taint::DerivedFromSecret.rank(), 1);
}

#[test]
fn taint_secret_rank_is_two() {
    assert_eq!(Taint::Secret.rank(), 2);
}

#[test]
fn join_clean_with_derived_returns_derived() {
    assert_eq!(
        join_taint(Taint::Clean, Taint::DerivedFromSecret),
        Taint::DerivedFromSecret
    );
}

#[test]
fn join_derived_with_clean_returns_derived() {
    assert_eq!(
        join_taint(Taint::DerivedFromSecret, Taint::Clean),
        Taint::DerivedFromSecret
    );
}

#[test]
fn join_clean_with_secret_returns_secret() {
    assert_eq!(join_taint(Taint::Clean, Taint::Secret), Taint::Secret);
}

#[test]
fn join_secret_with_derived_returns_secret() {
    assert_eq!(
        join_taint(Taint::Secret, Taint::DerivedFromSecret),
        Taint::Secret
    );
}

#[test]
fn join_many_empty_defaults_to_clean() {
    let taints: [Taint; 0] = [];
    assert_eq!(join_many(&taints), Taint::Clean);
}

#[test]
fn join_many_promotes_to_secret() {
    let taints = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
    assert_eq!(join_many(&taints), Taint::Secret);
}

#[test]
fn lattice_commutativity_law_holds_for_secret_and_derived() {
    assert!(is_commutative(Taint::Secret, Taint::DerivedFromSecret));
}

#[test]
fn lattice_associativity_law_holds_for_mixed_taints() {
    assert!(is_associative(
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret
    ));
}

#[test]
fn lattice_idempotence_holds_for_secret() {
    assert!(is_idempotent(Taint::Secret));
}

#[test]
fn lattice_identity_holds_for_derived() {
    assert!(has_identity(Taint::DerivedFromSecret));
}

#[test]
fn secret_never_downgrade_guard_holds() {
    assert!(secret_never_downgrades());
}

#[test]
fn derived_never_downgrade_guard_holds() {
    assert!(derived_never_downgrades());
}

#[test]
fn all_lattice_laws_hold_for_mixed_tuple() {
    assert!(all_lattice_laws(
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret
    ));
}
