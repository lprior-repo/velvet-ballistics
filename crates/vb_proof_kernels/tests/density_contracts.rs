use vb_proof_kernels::envelope_header::{
    EnvelopeHeader, HEADER_LEN, ValidationError, ValidationResult, compute_header_crc,
    validate_header_before_alloc, validate_header_crc,
};
use vb_proof_kernels::resource_budget::{
    Budget, Policy, branch_compose, loop_compose, sequential_compose,
};
use vb_proof_kernels::step_state::{
    StepState, all_transitions_exhaustive, is_valid_transition, next_states, non_terminal_states,
    terminal_cannot_transition_to_non_terminal, terminal_states, validate_transition,
};
use vb_proof_kernels::taint::{
    Taint, all_lattice_laws, derived_never_downgrades, has_identity, is_associative,
    is_commutative, is_idempotent, join_many, join_taint, secret_never_downgrades,
};

macro_rules! ktest {
    ($(#[$attr:meta])* $name:ident, $body:block) => {
        $(#[$attr])*
        fn $name() $body
    };
}

fn budget_with_steps_actions(steps: u64, actions: u64) -> Budget {
    let mut budget = Budget::new();
    budget.steps = steps;
    budget.actions = actions;
    budget
}

ktest!(
    #[test]
    taint_clean_rank_is_zero,
    {
        assert_eq!(Taint::Clean.rank(), 0);
    }
);

ktest!(
    #[test]
    taint_derived_rank_is_one,
    {
        assert_eq!(Taint::DerivedFromSecret.rank(), 1);
    }
);

ktest!(
    #[test]
    taint_secret_rank_is_two,
    {
        assert_eq!(Taint::Secret.rank(), 2);
    }
);

ktest!(
    #[test]
    taint_join_clean_clean_is_clean,
    {
        assert_eq!(join_taint(Taint::Clean, Taint::Clean), Taint::Clean);
    }
);

ktest!(
    #[test]
    taint_join_clean_derived_is_derived,
    {
        assert_eq!(
            join_taint(Taint::Clean, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_clean_secret_is_secret,
    {
        assert_eq!(join_taint(Taint::Clean, Taint::Secret), Taint::Secret);
    }
);

ktest!(
    #[test]
    taint_join_derived_clean_is_derived,
    {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Clean),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_derived_derived_is_derived,
    {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_derived_secret_is_secret,
    {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Secret),
            Taint::Secret
        );
    }
);

ktest!(
    #[test]
    taint_join_secret_clean_is_secret,
    {
        assert_eq!(join_taint(Taint::Secret, Taint::Clean), Taint::Secret);
    }
);

ktest!(
    #[test]
    taint_join_secret_derived_is_secret,
    {
        assert_eq!(
            join_taint(Taint::Secret, Taint::DerivedFromSecret),
            Taint::Secret
        );
    }
);

ktest!(
    #[test]
    taint_join_secret_secret_is_secret,
    {
        assert_eq!(join_taint(Taint::Secret, Taint::Secret), Taint::Secret);
    }
);

ktest!(
    #[test]
    taint_join_many_empty_is_clean,
    {
        assert_eq!(join_many(&[]), Taint::Clean);
    }
);

ktest!(
    #[test]
    taint_join_many_clean_only_is_clean,
    {
        assert_eq!(join_many(&[Taint::Clean, Taint::Clean]), Taint::Clean);
    }
);

ktest!(
    #[test]
    taint_join_many_finds_derived,
    {
        assert_eq!(
            join_many(&[Taint::Clean, Taint::DerivedFromSecret]),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_many_finds_secret,
    {
        assert_eq!(
            join_many(&[Taint::Clean, Taint::Secret, Taint::DerivedFromSecret]),
            Taint::Secret
        );
    }
);

ktest!(
    #[test]
    taint_commutative_clean_secret,
    {
        assert!(is_commutative(Taint::Clean, Taint::Secret));
    }
);

ktest!(
    #[test]
    taint_commutative_derived_secret,
    {
        assert!(is_commutative(Taint::DerivedFromSecret, Taint::Secret));
    }
);

ktest!(
    #[test]
    taint_associative_clean_derived_secret,
    {
        assert!(is_associative(
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret
        ));
    }
);

ktest!(
    #[test]
    taint_idempotent_secret,
    {
        assert!(is_idempotent(Taint::Secret));
    }
);

ktest!(
    #[test]
    taint_identity_derived,
    {
        assert!(has_identity(Taint::DerivedFromSecret));
    }
);

ktest!(
    #[test]
    taint_secret_never_downgrades_contract,
    {
        assert!(secret_never_downgrades());
    }
);

ktest!(
    #[test]
    taint_derived_never_downgrades_contract,
    {
        assert!(derived_never_downgrades());
    }
);

ktest!(
    #[test]
    taint_all_laws_for_clean_derived_secret,
    {
        assert!(all_lattice_laws(
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret
        ));
    }
);

ktest!(
    #[test]
    taint_all_laws_for_secret_clean_derived,
    {
        assert!(all_lattice_laws(
            Taint::Secret,
            Taint::Clean,
            Taint::DerivedFromSecret
        ));
    }
);

ktest!(
    #[test]
    step_pending_is_not_terminal,
    {
        assert!(!StepState::Pending.is_terminal());
    }
);

ktest!(
    #[test]
    step_running_is_not_terminal,
    {
        assert!(!StepState::Running.is_terminal());
    }
);

ktest!(
    #[test]
    step_waiting_is_not_terminal,
    {
        assert!(!StepState::Waiting.is_terminal());
    }
);

ktest!(
    #[test]
    step_asking_is_not_terminal,
    {
        assert!(!StepState::Asking.is_terminal());
    }
);

ktest!(
    #[test]
    step_succeeded_is_terminal,
    {
        assert!(StepState::Succeeded.is_terminal());
    }
);

ktest!(
    #[test]
    step_failed_is_terminal,
    {
        assert!(StepState::Failed.is_terminal());
    }
);

ktest!(
    #[test]
    step_cancelled_is_terminal,
    {
        assert!(StepState::Cancelled.is_terminal());
    }
);

ktest!(
    #[test]
    step_skipped_is_terminal,
    {
        assert!(StepState::Skipped.is_terminal());
    }
);

ktest!(
    #[test]
    step_pending_to_running_valid,
    {
        assert!(is_valid_transition(StepState::Pending, StepState::Running));
    }
);

ktest!(
    #[test]
    step_pending_to_succeeded_valid,
    {
        assert!(is_valid_transition(
            StepState::Pending,
            StepState::Succeeded
        ));
    }
);

ktest!(
    #[test]
    step_running_to_waiting_valid,
    {
        assert!(is_valid_transition(StepState::Running, StepState::Waiting));
    }
);

ktest!(
    #[test]
    step_running_to_asking_valid,
    {
        assert!(is_valid_transition(StepState::Running, StepState::Asking));
    }
);

ktest!(
    #[test]
    step_waiting_to_running_valid,
    {
        assert!(is_valid_transition(StepState::Waiting, StepState::Running));
    }
);

ktest!(
    #[test]
    step_asking_to_running_valid,
    {
        assert!(is_valid_transition(StepState::Asking, StepState::Running));
    }
);

ktest!(
    #[test]
    step_succeeded_self_valid,
    {
        assert!(is_valid_transition(
            StepState::Succeeded,
            StepState::Succeeded
        ));
    }
);

ktest!(
    #[test]
    step_failed_self_valid,
    {
        assert!(is_valid_transition(StepState::Failed, StepState::Failed));
    }
);

ktest!(
    #[test]
    step_cancelled_self_valid,
    {
        assert!(is_valid_transition(
            StepState::Cancelled,
            StepState::Cancelled
        ));
    }
);

ktest!(
    #[test]
    step_skipped_self_valid,
    {
        assert!(is_valid_transition(StepState::Skipped, StepState::Skipped));
    }
);

ktest!(
    #[test]
    step_running_to_pending_invalid,
    {
        assert!(!is_valid_transition(StepState::Running, StepState::Pending));
    }
);

ktest!(
    #[test]
    step_succeeded_to_running_invalid,
    {
        assert!(!is_valid_transition(
            StepState::Succeeded,
            StepState::Running
        ));
    }
);

ktest!(
    #[test]
    step_failed_to_pending_invalid,
    {
        assert!(!is_valid_transition(StepState::Failed, StepState::Pending));
    }
);

ktest!(
    #[test]
    step_waiting_to_succeeded_invalid,
    {
        assert!(!is_valid_transition(
            StepState::Waiting,
            StepState::Succeeded
        ));
    }
);

ktest!(
    #[test]
    step_validate_transition_returns_target_on_valid_edge,
    {
        assert_eq!(
            validate_transition(StepState::Pending, StepState::Running),
            Ok(StepState::Running)
        );
    }
);

ktest!(
    #[test]
    step_validate_transition_returns_error_on_invalid_edge,
    {
        assert_eq!(
            validate_transition(StepState::Failed, StepState::Running),
            Err("invalid_state_transition")
        );
    }
);

ktest!(
    #[test]
    step_next_states_pending_has_six_entries,
    {
        assert_eq!(next_states(StepState::Pending).len(), 6);
    }
);

ktest!(
    #[test]
    step_next_states_running_has_seven_entries,
    {
        assert_eq!(next_states(StepState::Running).len(), 7);
    }
);

ktest!(
    #[test]
    step_next_states_waiting_returns_self_and_running,
    {
        assert_eq!(
            next_states(StepState::Waiting),
            vec![StepState::Waiting, StepState::Running]
        );
    }
);

ktest!(
    #[test]
    step_terminal_states_has_four_entries,
    {
        assert_eq!(terminal_states().len(), 4);
    }
);

ktest!(
    #[test]
    step_non_terminal_states_has_four_entries,
    {
        assert_eq!(non_terminal_states().len(), 4);
    }
);

ktest!(
    #[test]
    step_terminal_cannot_transition_to_non_terminal_contract,
    {
        assert!(terminal_cannot_transition_to_non_terminal());
    }
);

ktest!(
    #[test]
    step_all_transitions_exhaustive_contract,
    {
        assert!(all_transitions_exhaustive());
    }
);

ktest!(
    #[test]
    budget_new_starts_with_zero_steps,
    {
        assert_eq!(Budget::new().steps, 0);
    }
);

ktest!(
    #[test]
    budget_new_starts_with_zero_actions,
    {
        assert_eq!(Budget::new().actions, 0);
    }
);

ktest!(
    #[test]
    budget_sequential_compose_adds_steps,
    {
        let result = sequential_compose(
            &budget_with_steps_actions(2, 3),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.steps, 7);
    }
);

ktest!(
    #[test]
    budget_sequential_compose_adds_actions,
    {
        let result = sequential_compose(
            &budget_with_steps_actions(2, 3),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.actions, 10);
    }
);

ktest!(
    #[test]
    budget_sequential_compose_saturates_steps,
    {
        let result = sequential_compose(
            &budget_with_steps_actions(u64::MAX, 0),
            &budget_with_steps_actions(1, 0),
        );
        assert_eq!(result.steps, u64::MAX);
    }
);

ktest!(
    #[test]
    budget_branch_compose_maxes_steps,
    {
        let result = branch_compose(
            &budget_with_steps_actions(2, 9),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.steps, 5);
    }
);

ktest!(
    #[test]
    budget_branch_compose_maxes_actions,
    {
        let result = branch_compose(
            &budget_with_steps_actions(2, 9),
            &budget_with_steps_actions(5, 7),
        );
        assert_eq!(result.actions, 9);
    }
);

ktest!(
    #[test]
    budget_loop_compose_multiplies_steps,
    {
        let result = loop_compose(&budget_with_steps_actions(2, 3), 4);
        assert_eq!(result.steps, 8);
    }
);

ktest!(
    #[test]
    budget_loop_compose_multiplies_actions,
    {
        let result = loop_compose(&budget_with_steps_actions(2, 3), 4);
        assert_eq!(result.actions, 12);
    }
);

ktest!(
    #[test]
    budget_loop_compose_saturates_actions,
    {
        let result = loop_compose(&budget_with_steps_actions(0, u64::MAX), 2);
        assert_eq!(result.actions, u64::MAX);
    }
);

ktest!(
    #[test]
    budget_policy_default_max_actions,
    {
        assert_eq!(Policy::default_policy().max_actions, 100_000);
    }
);

ktest!(
    #[test]
    budget_policy_default_max_parallel,
    {
        assert_eq!(Policy::default_policy().max_parallel, 256);
    }
);

ktest!(
    #[test]
    budget_policy_default_max_steps,
    {
        assert_eq!(Policy::default_policy().max_steps, 1_000_000);
    }
);

ktest!(
    #[test]
    budget_policy_accepts_empty_budget,
    {
        assert!(Policy::default_policy().within(&Budget::new()).is_empty());
    }
);

ktest!(
    #[test]
    budget_policy_rejects_actions_over_limit,
    {
        let mut budget = Budget::new();
        budget.actions = 100_001;
        assert_eq!(Policy::default_policy().within(&budget), vec!["actions"]);
    }
);

ktest!(
    #[test]
    budget_policy_rejects_parallel_over_limit,
    {
        let mut budget = Budget::new();
        budget.parallel = 257;
        assert_eq!(Policy::default_policy().within(&budget), vec!["parallel"]);
    }
);

ktest!(
    #[test]
    budget_policy_rejects_runtime_over_limit,
    {
        let mut budget = Budget::new();
        budget.run_time_secs = Policy::default_policy().max_run_time + 1;
        assert_eq!(Policy::default_policy().within(&budget), vec!["run_time"]);
    }
);

ktest!(
    #[test]
    budget_policy_rejects_result_bytes_over_limit,
    {
        let mut budget = Budget::new();
        budget.result_bytes = Policy::default_policy().max_result_bytes + 1;
        assert_eq!(
            Policy::default_policy().within(&budget),
            vec!["result_bytes"]
        );
    }
);

ktest!(
    #[test]
    budget_policy_rejects_steps_over_limit,
    {
        let mut budget = Budget::new();
        budget.steps = 1_000_001;
        assert_eq!(Policy::default_policy().within(&budget), vec!["steps"]);
    }
);

ktest!(
    #[test]
    budget_policy_reports_multiple_violations_in_order,
    {
        let mut budget = Budget::new();
        budget.actions = 100_001;
        budget.steps = 1_000_001;
        assert_eq!(
            Policy::default_policy().within(&budget),
            vec!["actions", "steps"]
        );
    }
);

ktest!(
    #[test]
    envelope_header_len_constant_is_sixty,
    {
        assert_eq!(HEADER_LEN, 60);
    }
);

ktest!(
    #[test]
    envelope_new_uses_magic_value,
    {
        assert_eq!(EnvelopeHeader::new().magic, EnvelopeHeader::MAGIC_VALUE);
    }
);

ktest!(
    #[test]
    envelope_new_uses_version_one,
    {
        assert_eq!(EnvelopeHeader::new().version, 1);
    }
);

ktest!(
    #[test]
    envelope_new_has_zero_payload_len,
    {
        assert_eq!(EnvelopeHeader::new().payload_len(), 0);
    }
);

ktest!(
    #[test]
    envelope_default_matches_new,
    {
        assert_eq!(EnvelopeHeader::default(), EnvelopeHeader::new());
    }
);

ktest!(
    #[test]
    envelope_validate_magic_accepts_new,
    {
        assert!(EnvelopeHeader::new().validate_magic());
    }
);

ktest!(
    #[test]
    envelope_validate_magic_rejects_changed_magic,
    {
        let mut header = EnvelopeHeader::new();
        header.magic = 0;
        assert!(!header.validate_magic());
    }
);

ktest!(
    #[test]
    envelope_validate_header_len_returns_true,
    {
        assert!(EnvelopeHeader::new().validate_header_len());
    }
);

ktest!(
    #[test]
    envelope_payload_len_combines_high_and_low,
    {
        let mut header = EnvelopeHeader::new();
        header.payload_len_hi = 1;
        header.payload_len_u32 = 2;
        assert_eq!(header.payload_len(), 4_294_967_298);
    }
);

ktest!(
    #[test]
    envelope_validate_payload_len_accepts_equal_max,
    {
        let header = EnvelopeHeader::new();
        assert!(header.validate_payload_len(0));
    }
);

ktest!(
    #[test]
    envelope_validate_payload_len_rejects_over_max,
    {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = 2;
        assert!(!header.validate_payload_len(1));
    }
);

ktest!(
    #[test]
    envelope_validate_before_alloc_accepts_new_header,
    {
        assert_eq!(
            EnvelopeHeader::new().validate_before_alloc(0),
            ValidationResult::Ok
        );
    }
);

ktest!(
    #[test]
    envelope_validate_before_alloc_rejects_bad_magic,
    {
        let mut header = EnvelopeHeader::new();
        header.magic = 0;
        assert_eq!(
            header.validate_before_alloc(0),
            ValidationResult::Err(ValidationError::InvalidMagic)
        );
    }
);

ktest!(
    #[test]
    envelope_validate_before_alloc_rejects_large_payload,
    {
        let mut header = EnvelopeHeader::new();
        header.payload_len_u32 = 2;
        assert_eq!(
            header.validate_before_alloc(1),
            ValidationResult::Err(ValidationError::PayloadTooLarge)
        );
    }
);

ktest!(
    #[test]
    envelope_free_function_delegates_validation,
    {
        assert_eq!(
            validate_header_before_alloc(&EnvelopeHeader::new(), 0),
            ValidationResult::Ok
        );
    }
);

ktest!(
    #[test]
    envelope_crc_stub_returns_zero,
    {
        assert_eq!(compute_header_crc(&EnvelopeHeader::new()), 0);
    }
);

ktest!(
    #[test]
    envelope_crc_stub_validates_header,
    {
        assert!(validate_header_crc(&EnvelopeHeader::new()));
    }
);
