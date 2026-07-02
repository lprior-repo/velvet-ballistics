    /// K-F2: validate_transition never panics for any of 64 pairs.
    #[kani::proof]
    fn validate_transition_no_panic_random() {
        let current_u8: u8 = kani::any();
        let new_u8: u8 = kani::any();
        let current = step_state_from_u8(current_u8);
        let new = step_state_from_u8(new_u8);
        let _result = validate_transition_inline(current, new);
    }

    /// K-F3: Idempotency — same-state transitions always return true.
    #[kani::proof]
    fn validate_transition_idempotent() {
        let state_u8 = kani::any::<u8>();
        let state = step_state_from_u8(state_u8 % 8);
        let result = validate_transition_inline(state, state);
        kani::assert(result, "self-transition always valid");
    }

    /// K-F4: Running can reach any terminal or suspend state.
    /// Uses kani::any() to symbolically explore valid target states.
    #[kani::proof]
    fn validate_transition_running_to_all_valid_targets() {
        let c = StepState::Running;
        let target: StepState = kani::any();
        // Running can transition to: Running, Succeeded, Failed, Waiting, Asking, Skipped, Cancelled
        // Not valid: Pending
        let result = validate_transition_inline(c, target);
        // If target is not Pending, transition should be valid
        if target != StepState::Pending {
            kani::assert(result, "Running can transition to non-Pending state");
        } else {
            kani::assert(!result, "Running cannot transition to Pending");
        }
    }

    /// K-F5: Terminal states block all non-self transitions EXCEPT Succeeded->Pending.
    /// Uses kani::any() to symbolically verify terminal blocking property.
    /// NOTE: vb_proof_kernels/src/step_state.rs:48 explicitly allows Succeeded->Pending,
    /// so this harness reflects that design decision.
    #[kani::proof]
    fn validate_transition_terminal_blocks_all() {
        let terminal: StepState = kani::any();
        let target: StepState = kani::any();
        // Succeeded, Failed, Skipped, Cancelled are terminal states
        let is_terminal = matches!(
            terminal,
            StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled
        );
        kani::assume(is_terminal);
        let result = validate_transition_inline(terminal, target);
        // Terminal states can transition to themselves (idempotent re-mark)
        if terminal == target {
            kani::assert(result, "terminal->self allowed");
        // Succeeded->Pending is explicitly allowed by proof kernel (step_state.rs:48)
        } else if terminal == StepState::Succeeded && target == StepState::Pending {
            kani::assert(result, "Succeeded->Pending allowed by proof kernel");
        } else {
            kani::assert(!result, "terminal->other blocked");
        }
    }

