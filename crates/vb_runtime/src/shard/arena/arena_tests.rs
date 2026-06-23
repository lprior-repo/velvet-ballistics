#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]

//! Tests for the arena allocator.

use super::types::{ArenaError, Generation, SlotHandle, SlotId};
use super::{Arena, SlotSet};

#[test]
fn slot_id_constants() {
    assert!(SlotId::INVALID.is_invalid());
    assert!(!SlotId::new(0).is_invalid());
}

#[test]
fn generation_successor() {
    let g = Generation::INITIAL;
    assert_eq!(g.successor(), Generation(1));
    assert_eq!(Generation::TERMINAL.successor(), Generation::TERMINAL);
    assert!(!g.is_terminal());
}

#[test]
fn arena_allocate_deallocate() {
    let mut arena: Arena<String> = Arena::new();

    let handle1 = arena.allocate("test".to_string()).unwrap();
    assert_eq!(arena.get(handle1).unwrap(), "test");
    assert_eq!(handle1.generation(), Generation::INITIAL);

    arena.deallocate(handle1).unwrap();
    assert!(matches!(
        arena.get(handle1),
        Err(ArenaError::GenerationMismatch)
    ));

    // Reuse slot
    let handle2 = arena.allocate("test2".to_string()).unwrap();
    assert_eq!(handle2.slot_id(), handle1.slot_id());
    assert_eq!(handle2.generation(), handle1.generation().successor());
    assert_eq!(arena.get(handle2).unwrap(), "test2");
    assert!(matches!(
        arena.get(handle1),
        Err(ArenaError::GenerationMismatch)
    ));
}

#[test]
fn arena_contains() {
    let mut arena: Arena<i32> = Arena::new();
    let handle = arena.allocate(42).unwrap();

    assert!(arena.contains(handle));
    assert!(!Arena::<i32>::new().contains(handle));

    arena.deallocate(handle).unwrap();
    assert!(!arena.contains(handle));
}

#[test]
fn slot_set_basic() {
    let mut set = SlotSet::new();
    let handle = set.arena.allocate(()).unwrap();

    assert!(set.contains(handle));
    assert_eq!(set.len(), 1);

    set.insert(handle).unwrap();
    assert_eq!(set.len(), 1);

    set.remove(handle).unwrap();
    assert!(!set.contains(handle));
    assert!(set.is_empty());
}

#[test]
fn slot_set_rejects_invalid_or_gapped_handle() {
    let mut set = SlotSet::new();
    let invalid = SlotHandle::new(SlotId::INVALID, Generation::INITIAL);
    let gapped = SlotHandle::new(SlotId::new(8), Generation::INITIAL);

    assert_eq!(set.insert(invalid), Err(ArenaError::InvalidSlotId));
    assert_eq!(set.insert(gapped), Err(ArenaError::InvalidSlotId));
    assert!(set.is_empty());
}

#[test]
fn slot_set_rejects_stale_reinsert() {
    let mut set = SlotSet::new();
    let handle = SlotHandle::new(SlotId::new(0), Generation::INITIAL);
    let successor = SlotHandle::new(handle.slot_id(), handle.generation().successor());

    set.insert(handle).unwrap();
    set.remove(handle).unwrap();

    assert_eq!(set.insert(handle), Err(ArenaError::GenerationMismatch));
    set.insert(successor).unwrap();
    assert!(set.contains(successor));
}

// RS-026 regression: when SlotSet::ensure_insert_slot grows the arena for a
// previously-unseen slot index, the new slot's generation must be initialized
// to Generation::INITIAL (matching Arena::push_new_slot). The pre-fix code
// blindly stored the caller-provided handle generation, coupling the set's
// internal generation state to external caller state.
#[test]
fn slot_set_ensures_new_slot_initializes_generation_to_initial() {
    let mut set = SlotSet::new();
    // Fabricate a handle with a non-initial generation for slot index 0.
    let fabricated_generation = Generation(99);
    assert!(!fabricated_generation.is_terminal());
    let handle = SlotHandle::new(SlotId::new(0), fabricated_generation);

    // Inserting into a fresh set forces the `idx == slots.len()` growth branch.
    set.insert(handle).unwrap();

    // The arena's stored generation for slot 0 must be Generation::INITIAL,
    // NOT the fabricated caller-provided value (99).
    let stored = set
        .arena
        .generations
        .get(0)
        .copied()
        .expect("test slot must exist");
    assert_eq!(
        stored,
        Generation::INITIAL,
        "new arena slots must start at Generation::INITIAL, not the caller-provided generation"
    );
}
