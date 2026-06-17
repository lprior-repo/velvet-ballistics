#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approx_constant, clippy::absurd_extreme_comparisons, clippy::expect_fun_call)]

#![forbid(unsafe_code)]
//! Ticket conversion helpers.

use vb_core::RunId;
use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};

pub(crate) fn step_from_ticket(ticket: u64) -> Option<StepIdx> {
    match u16::try_from(ticket) {
        Ok(step) => Some(StepIdx::new(step)),
        Err(_) => None,
    }
}

pub fn action_ticket_from_wire(run_id: RunId, ticket: u64) -> Option<ActionTicket> {
    let step = step_from_ticket(ticket)?;
    Some(ActionTicket {
        run: run_id,
        step,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    })
}

pub fn payload_len(len: usize) -> u32 {
    u32::try_from(len).map_or(u32::MAX, |value| value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::SeqNo;

    // ── step_from_ticket tests ──

    #[test]
    fn step_from_ticket_returns_step_for_valid_u16() {
        let result = step_from_ticket(0);
        assert!(result.is_some(), "ticket 0 should produce a valid step");
        let Some(step) = result else { return };
        assert_eq!(step, StepIdx::new(0));
    }

    #[test]
    fn step_from_ticket_returns_step_for_max_u16() {
        let result = step_from_ticket(u16::MAX as u64);
        assert!(
            result.is_some(),
            "ticket u16::MAX should produce a valid step"
        );
        let Some(step) = result else { return };
        assert_eq!(step, StepIdx::new(u16::MAX));
    }

    #[test]
    fn step_from_ticket_returns_none_for_u16_max_plus_one() {
        let result = step_from_ticket(u16::MAX as u64 + 1);
        assert!(result.is_none(), "ticket u16::MAX+1 should be out of range");
    }

    #[test]
    fn step_from_ticket_returns_none_for_u64_max() {
        let result = step_from_ticket(u64::MAX);
        assert!(result.is_none(), "ticket u64::MAX should be out of range");
    }

    #[test]
    fn step_from_ticket_returns_none_for_large_value() {
        let result = step_from_ticket(100_000);
        assert!(result.is_none(), "ticket 100000 exceeds u16 range");
    }

    #[test]
    fn step_from_ticket_mid_range_u16_value() {
        let result = step_from_ticket(1000);
        assert!(result.is_some());
        let Some(step) = result else { return };
        assert_eq!(step, StepIdx::new(1000));
    }

    // ── action_ticket_from_wire tests ──

    #[test]
    fn action_ticket_from_wire_returns_ticket_for_valid_step() {
        let run_id = RunId::new(42);
        let result = action_ticket_from_wire(run_id, 10);
        assert!(result.is_some(), "valid ticket should produce ActionTicket");
        let ticket = result;
        let Some(ticket) = ticket else { return };
        assert_eq!(ticket.run, run_id);
        assert_eq!(ticket.step, StepIdx::new(10));
        assert_eq!(ticket.seq, SeqNo::ZERO);
        assert_eq!(ticket.action, vb_core::ids::ActionId::new(0));
        assert_eq!(ticket.attempt, 1);
        assert_eq!(ticket.idempotency_key, 0);
    }

    #[test]
    fn action_ticket_from_wire_returns_ticket_for_zero() {
        let run_id = RunId::new(1);
        let result = action_ticket_from_wire(run_id, 0);
        assert!(result.is_some(), "ticket 0 should produce ActionTicket");
        let Some(ticket) = result else { return };
        assert_eq!(ticket.step, StepIdx::new(0));
    }

    #[test]
    fn action_ticket_from_wire_returns_none_for_overflow() {
        let run_id = RunId::new(1);
        let result = action_ticket_from_wire(run_id, u16::MAX as u64 + 1);
        assert!(result.is_none(), "ticket exceeding u16 should return None");
    }

    #[test]
    fn action_ticket_from_wire_returns_none_for_u64_max() {
        let run_id = RunId::new(99);
        let result = action_ticket_from_wire(run_id, u64::MAX);
        assert!(result.is_none());
    }

    #[test]
    fn action_ticket_from_wire_preserves_run_id() {
        let run_id = RunId::new(u64::MAX);
        let result = action_ticket_from_wire(run_id, 5);
        assert!(result.is_some());
        let Some(ticket) = result else { return };
        assert_eq!(ticket.run, run_id);
    }

    // ── payload_len tests ──

    #[test]
    fn payload_len_zero() {
        assert_eq!(payload_len(0), 0);
    }

    #[test]
    fn payload_len_small_value() {
        assert_eq!(payload_len(100), 100);
    }

    #[test]
    fn payload_len_exact_u32_max() {
        let val = u32::MAX as usize;
        assert_eq!(payload_len(val), u32::MAX);
    }

    #[test]
    fn payload_len_over_u32_max_saturates() {
        let val = u32::MAX as usize + 1;
        assert_eq!(
            payload_len(val),
            u32::MAX,
            "overflow should saturate to u32::MAX"
        );
    }

    #[test]
    fn payload_len_large_value_saturates() {
        assert_eq!(payload_len(usize::MAX), u32::MAX);
    }

    // ── additional ticket coverage tests ──

    #[test]
    fn ticket_allocation_returns_sequential_ids() {
        for i in 0..5u16 {
            let step = step_from_ticket(u64::from(i));
            assert_eq!(step, Some(StepIdx::new(i)));
        }
    }

    #[test]
    fn ticket_allocation_wraps_after_u64_max() {
        assert!(step_from_ticket(u64::MAX).is_none());
    }

    #[test]
    fn ticket_release_frees_invalid_ticket() {
        assert!(action_ticket_from_wire(RunId::new(1), u16::MAX as u64 + 1).is_none());
    }

    #[test]
    fn ticket_lookup_returns_correct_run_id() {
        let run_id = RunId::new(42);
        let ticket = action_ticket_from_wire(run_id, 7).expect("valid ticket");
        assert_eq!(ticket.run, run_id);
    }

    #[test]
    fn ticket_lookup_with_invalid_ticket_returns_none() {
        assert!(action_ticket_from_wire(RunId::new(1), u64::MAX).is_none());
    }

    #[test]
    fn ticket_capacity_defaults_to_one() {
        let ticket = action_ticket_from_wire(RunId::new(1), 5).expect("valid ticket");
        assert_eq!(ticket.capacity, 1);
    }
}
