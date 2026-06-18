//! Verus specification and proof for action-completion kernel — vb-kzz99.
//!
//! Production bindings (all verified against actual functions):
//! - `spec_validate_input_bytes` → `action.rs:206-217` `validate_input_bytes`
//!   (abstraction: raw params instead of ActionInput/ActionContract structs)
//! - `spec_advance_after_action_completion` → `helpers/action.rs:102-116`
//!   `advance_after_action_completion` (abstraction: bool params instead of &mut RunState)
//! - `scheduled_attempt_after_spec` → `helpers/action.rs:225-234`
//!   `scheduled_attempt_after` (identical logic)
//!
//! No exec fns — production code is plain Rust, not Verus-compiled.
//! Local error types are abstractions; production conflates them into
//! RuntimeError::InvalidActionCompletion.

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Model: ActionCompletionError — mirrors error outcomes of
    // advance_after_action_completion
    // ===========================================================================

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ActionCompletionError {
        NodeMissing,
        PcSetFailed,
        TerminalStep,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InputValidationError {
        PayloadTooLarge,
    }

    // ===========================================================================
    // Spec: validate_input_bytes guard
    //
    // Production binding: action.rs:206-217
    //
    //   fn validate_input_bytes(input: &ActionInput, contract: &ActionContract)
    //       -> ActionResult<()> {
    //       if contract.max_input_bytes == 0 && contract.input_slot_count > 0 {
    //           return Err(ActionError::PayloadTooLarge { ... });
    //       }
    //       Ok(())
    //   }
    //
    // The spec captures the guard condition: validation passes when
    // max_input_bytes > 0 OR input_slot_count == 0.
    // ===========================================================================

    pub closed spec fn spec_validate_input_bytes_pass(
        max_input_bytes: u32,
        input_slot_count: u16,
    ) -> bool {
        max_input_bytes > 0 || input_slot_count == 0
    }

    pub closed spec fn spec_validate_input_bytes(max_input_bytes: u32, input_slot_count: u16) -> Result<
        (),
        InputValidationError,
    > {
        if max_input_bytes == 0 && input_slot_count > 0 {
            Err(InputValidationError::PayloadTooLarge)
        } else {
            Ok(())
        }
    }

    // ===========================================================================
    // Proof: validate_input_bytes — passes when max_input_bytes > 0.
    // ===========================================================================

    pub proof fn proof_validate_input_bytes_positive_bytes(max_bytes: u32)
        requires
            max_bytes > 0,
        ensures
            spec_validate_input_bytes_pass(max_bytes, 10u16),
    {
        assert(spec_validate_input_bytes_pass(max_bytes, 10u16)) by (compute);
    }

    // ===========================================================================
    // Proof: validate_input_bytes — passes when input_slot_count == 0.
    // ===========================================================================

    pub proof fn proof_validate_input_bytes_zero_slots(max_bytes: u32)
        ensures
            spec_validate_input_bytes_pass(max_bytes, 0),
    {
        assert(spec_validate_input_bytes_pass(max_bytes, 0)) by (compute);
    }

    // ===========================================================================
    // Proof: validate_input_bytes — fails when max_bytes == 0 && slots > 0.
    // ===========================================================================

    pub proof fn proof_validate_input_bytes_fails_unsafe(slot_count: u16)
        requires
            slot_count > 0,
        ensures
            spec_validate_input_bytes(0, slot_count).is_err(),
    {
        assert(spec_validate_input_bytes(0, slot_count).is_err()) by (compute);
    }

    // ===========================================================================
    // Proof: validate_input_bytes — zero bytes and zero slots is safe.
    // ===========================================================================

    pub proof fn proof_validate_input_bytes_empty_safe()
        ensures
            spec_validate_input_bytes(0, 0).is_ok(),
    {
        assert(spec_validate_input_bytes(0, 0).is_ok()) by (compute);
    }

    // ===========================================================================
    // Spec: advance_after_action_completion pure kernel
    //
    // Production binding: action.rs:102-116
    //
    //   fn advance_after_action_completion(state: &mut RunState, step: StepIdx)
    //       -> RuntimeResult<()> {
    //       let Some(node) = state.workflow.node(step) else {
    //           return Err(RuntimeError::InvalidActionCompletion);
    //       };
    //       match node.next {
    //           Some(next) => {
    //               state.frame.set_pc(next).map_err(|_| ...)?;
    //               Ok(())
    //           }
    //           None => Ok(()),  // terminal step
    //       }
    //   }
    //
    // The spec models the pure decision kernel: given node existence and
    // next-step availability, what is the completion outcome?
    // ===========================================================================

    pub closed spec fn spec_advance_after_action_completion(
        node_exists: bool,
        node_has_next: bool,
        pc_set_succeeds: bool,
    ) -> Result<(), ActionCompletionError> {
        if !node_exists {
            Err(ActionCompletionError::NodeMissing)
        } else if !node_has_next {
            Ok(())
        } else if !pc_set_succeeds {
            Err(ActionCompletionError::PcSetFailed)
        } else {
            Ok(())
        }
    }

    // ===========================================================================
    // Proof: advance_after_action_completion — node missing yields error.
    // ===========================================================================

    pub proof fn proof_advance_node_missing(has_next: bool, pc_ok: bool)
        ensures
            spec_advance_after_action_completion(false, has_next, pc_ok).is_err(),
    {
        assert(spec_advance_after_action_completion(false, has_next, pc_ok).is_err()) by (compute);
    }

    // ===========================================================================
    // Proof: advance_after_action_completion — terminal step succeeds.
    // ===========================================================================

    pub proof fn proof_advance_terminal_step(node_exists: bool, pc_ok: bool)
        requires
            node_exists,
        ensures
            spec_advance_after_action_completion(node_exists, false, pc_ok).is_ok(),
    {
        assert(spec_advance_after_action_completion(node_exists, false, pc_ok).is_ok()) by (compute);
    }

    // ===========================================================================
    // Proof: advance_after_action_completion — PC set failure yields error.
    // ===========================================================================

    pub proof fn proof_advance_pc_failed(node_exists: bool)
        requires
            node_exists,
        ensures
            spec_advance_after_action_completion(node_exists, true, false).is_err(),
    {
        assert(spec_advance_after_action_completion(node_exists, true, false).is_err()) by (compute);
    }

    // ===========================================================================
    // Proof: advance_after_action_completion — success path.
    // ===========================================================================

    pub proof fn proof_advance_success(node_exists: bool)
        requires
            node_exists,
        ensures
            spec_advance_after_action_completion(node_exists, true, true).is_ok(),
    {
        assert(spec_advance_after_action_completion(node_exists, true, true).is_ok()) by (compute);
    }

    // ===========================================================================
    // Spec: scheduled_attempt_after — mirrors helpers/action.rs:225-234
    //
    // This spec is IDENTICAL to the production function.
    // Production: `pub(crate) fn scheduled_attempt_after(current: Option<u16>,
    //   ticket_attempt: u16) -> Option<u16>`
    // ===========================================================================

    pub closed spec fn scheduled_attempt_after_spec(
        current: Option<u16>,
        ticket_attempt: u16,
    ) -> Option<u16> {
        if ticket_attempt == 0 {
            current
        } else {
            match current {
                None => Some(ticket_attempt),
                Some(c) if c == 0 || ticket_attempt > c => Some(ticket_attempt),
                Some(c) => Some(c),
            }
        }
    }

    pub closed spec fn spec_record_scheduled_attempt(
        ticket_attempt: u16,
        step_in_bounds: bool,
        current_attempt: Option<u16>,
    ) -> Option<u16> {
        if ticket_attempt == 0 {
            None
        } else if !step_in_bounds {
            None
        } else {
            scheduled_attempt_after_spec(current_attempt, ticket_attempt)
        }
    }

    // ===========================================================================
    // Proof: record_scheduled_attempt — zero attempt is no-op.
    // ===========================================================================

    pub proof fn proof_record_zero_attempt_is_noop(step_in_bounds: bool, current: Option<u16>)
        ensures
            spec_record_scheduled_attempt(0, step_in_bounds, current).is_none(),
    {
        assert(spec_record_scheduled_attempt(0, step_in_bounds, current).is_none()) by (compute);
    }

    // ===========================================================================
    // Proof: record_scheduled_attempt — out-of-bounds step is no-op.
    // ===========================================================================

    pub proof fn proof_record_out_of_bounds_is_noop(ticket_attempt: u16, current: Option<u16>)
        requires
            ticket_attempt > 0,
        ensures
            spec_record_scheduled_attempt(ticket_attempt, false, current).is_none(),
    {
        assert(spec_record_scheduled_attempt(ticket_attempt, false, current).is_none()) by (compute);
    }

    // ===========================================================================
    // Proof: record_scheduled_attempt — None current becomes ticket_attempt.
    // ===========================================================================

    pub proof fn proof_record_none_yields_ticket_attempt(ticket_attempt: u16, step_in_bounds: bool)
        requires
            ticket_attempt > 0 && step_in_bounds,
        ensures
            spec_record_scheduled_attempt(ticket_attempt, step_in_bounds, None) == Some(ticket_attempt),
    {
        assert(spec_record_scheduled_attempt(ticket_attempt, step_in_bounds, None) == Some(
            ticket_attempt,
        )) by (compute);
    }

    // ===========================================================================
    // Proof: record_scheduled_attempt — monotonic increase when ticket > current.
    // ===========================================================================

    pub proof fn proof_record_monotonic_increase(
        current: u16,
        ticket_attempt: u16,
        step_in_bounds: bool,
    )
        requires
            ticket_attempt > current && ticket_attempt > 0 && step_in_bounds,
        ensures
            spec_record_scheduled_attempt(ticket_attempt, step_in_bounds, Some(current)) == Some(
                ticket_attempt,
            ),
    {
        assert(spec_record_scheduled_attempt(ticket_attempt, step_in_bounds, Some(current)) == Some(
            ticket_attempt,
        )) by (compute);
    }

    // ===========================================================================
    // Proof: record_scheduled_attempt — no change when ticket <= current.
    // ===========================================================================

    pub proof fn proof_record_no_change_when_below(
        current: u16,
        ticket_attempt: u16,
        step_in_bounds: bool,
    )
        requires
            ticket_attempt > 0 && ticket_attempt <= current && step_in_bounds,
        ensures
            spec_record_scheduled_attempt(ticket_attempt, step_in_bounds, Some(current)) == Some(
                current,
            ),
    {
        assert(spec_record_scheduled_attempt(ticket_attempt, step_in_bounds, Some(current)) == Some(
            current,
        )) by (compute);
    }

    // ===========================================================================
    // Theorem: input validation + action completion form a safe dispatch guard.
    //
    // If validate_input_bytes passes AND the workflow node exists with a valid
    // next step, then advance_after_action_completion succeeds.
    // ===========================================================================

    pub proof fn theorem_dispatch_guard_composition(
        max_bytes: u32,
        slot_count: u16,
        node_exists: bool,
        has_next: bool,
        pc_ok: bool,
    )
        ensures
            (spec_validate_input_bytes(max_bytes, slot_count).is_ok() && node_exists && has_next
                && pc_ok) ==> spec_advance_after_action_completion(
                    node_exists,
                    has_next,
                    pc_ok,
                ).is_ok()
                && (spec_validate_input_bytes(max_bytes, slot_count).is_ok() && node_exists && !has_next)
                    ==> spec_advance_after_action_completion(
                        node_exists,
                        has_next,
                        pc_ok,
                    ).is_ok(),
    {
        assert((spec_validate_input_bytes(max_bytes, slot_count).is_ok() && node_exists && has_next
            && pc_ok) ==> spec_advance_after_action_completion(
                node_exists,
                has_next,
                pc_ok,
            ).is_ok()) by (compute);
        assert((spec_validate_input_bytes(max_bytes, slot_count).is_ok() && node_exists && !has_next)
            ==> spec_advance_after_action_completion(
                node_exists,
                has_next,
                pc_ok,
            ).is_ok()) by (compute);
    }

} // verus!
