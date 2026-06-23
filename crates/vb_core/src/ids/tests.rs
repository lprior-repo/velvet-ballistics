//! Tests for compact numeric identifiers.
//!
//! Coverage: construction, boundary values, checked arithmetic, ordering,
//! FromStr, Debug, Hash, Copy/Clone, and constant constants (ZERO, MIN, MAX).

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
    unused_imports,
    dead_code,
    unused_variables
)]

use super::{RunId, SeqNo, SlotIdx, StepIdx, WorkflowId};

#[test]
fn workflow_id_get_returns_inner_value() {
    let id = WorkflowId::new(42);
    assert_eq!(id.get(), 42);
}

#[test]
fn run_id_get_returns_inner_value() {
    let id = RunId::new(12345);
    assert_eq!(id.get(), 12345);
}

#[test]
fn step_idx_as_usize_returns_inner_value() {
    let idx = StepIdx::new(7);
    assert_eq!(idx.as_usize(), 7);
}

#[test]
fn slot_idx_as_usize_returns_inner_value() {
    let idx = SlotIdx::new(15);
    assert_eq!(idx.as_usize(), 15);
}

// =========================================================================
// Adversarial BDD tests — ID boundary and overflow edge cases
// =========================================================================

#[test]
fn step_idx_zero_is_valid() {
    let idx = StepIdx::new(0);
    assert_eq!(idx.get(), 0);
    assert_eq!(idx.as_usize(), 0);
}

#[test]
fn step_idx_max_u16_is_valid() {
    let idx = StepIdx::new(u16::MAX);
    assert_eq!(idx.get(), u16::MAX);
}

#[test]
fn step_idx_checked_add_overflow_returns_none() {
    let idx = StepIdx::new(u16::MAX);
    assert_eq!(idx.checked_add(1), None);
}

#[test]
fn step_idx_checked_add_zero_is_identity() {
    let idx = StepIdx::new(100);
    assert_eq!(idx.checked_add(0), Some(StepIdx::new(100)));
}

#[test]
fn step_idx_checked_add_exact_max_saturates() {
    let idx = StepIdx::new(0);
    assert_eq!(idx.checked_add(u16::MAX), Some(StepIdx::new(u16::MAX)));
}

#[test]
fn slot_idx_zero_is_valid() {
    let idx = SlotIdx::new(0);
    assert_eq!(idx.get(), 0);
    assert_eq!(idx.as_usize(), 0);
}

#[test]
fn slot_idx_max_u16_is_valid() {
    let idx = SlotIdx::new(u16::MAX);
    assert_eq!(idx.get(), u16::MAX);
}

#[test]
fn slot_idx_checked_add_overflow_returns_none() {
    let idx = SlotIdx::new(u16::MAX);
    assert_eq!(idx.checked_add(1), None);
}

#[test]
fn slot_idx_checked_add_exact_max() {
    let idx = SlotIdx::new(0);
    assert_eq!(idx.checked_add(u16::MAX), Some(SlotIdx::new(u16::MAX)));
}

#[test]
fn slot_idx_min_is_zero() {
    assert_eq!(SlotIdx::MIN.get(), 0);
}

#[test]
fn slot_idx_max_is_u16_max() {
    assert_eq!(SlotIdx::MAX.get(), u16::MAX);
}

#[test]
fn slot_idx_zero_constant_is_zero() {
    assert_eq!(SlotIdx::ZERO.get(), 0);
}

#[test]
fn const_idx_checked_add_overflow_returns_none() {
    use super::ConstIdx;
    let idx = ConstIdx::new(u16::MAX);
    assert_eq!(idx.checked_add(1), None);
}

#[test]
fn const_idx_checked_add_success() {
    use super::ConstIdx;
    let idx = ConstIdx::new(10);
    assert_eq!(idx.checked_add(5), Some(ConstIdx::new(15)));
}

#[test]
fn seq_no_zero_is_valid() {
    use super::SeqNo;
    assert_eq!(SeqNo::ZERO.get(), 0);
}

#[test]
fn seq_no_min_is_zero() {
    use super::SeqNo;
    assert_eq!(SeqNo::MIN.get(), 0);
}

#[test]
fn seq_no_max_is_u64_max() {
    use super::SeqNo;
    assert_eq!(SeqNo::MAX.get(), u64::MAX);
}

#[test]
fn seq_no_checked_add_overflow_returns_none() {
    use super::SeqNo;
    let seq = SeqNo::new(u64::MAX);
    assert_eq!(seq.checked_add(1), None);
}

#[test]
fn seq_no_checked_add_exact_max() {
    use super::SeqNo;
    let seq = SeqNo::new(0);
    assert_eq!(seq.checked_add(u64::MAX), Some(SeqNo::new(u64::MAX)));
}

#[test]
fn run_id_zero_constant() {
    assert_eq!(RunId::ZERO.get(), 0);
}

#[test]
fn run_id_max_u64() {
    let id = RunId::new(u64::MAX);
    assert_eq!(id.get(), u64::MAX);
}

#[test]
fn symbol_id_zero_is_valid() {
    use super::SymbolId;
    let id = SymbolId::new(0);
    assert_eq!(id.get(), 0);
}

#[test]
fn symbol_id_max_u32_is_valid() {
    use super::SymbolId;
    let id = SymbolId::new(u32::MAX);
    assert_eq!(id.get(), u32::MAX);
}

#[test]
fn list_id_max_u32_is_valid() {
    use super::ListId;
    let id = ListId::new(u32::MAX);
    assert_eq!(id.get(), u32::MAX);
}

#[test]
fn object_id_max_u32_is_valid() {
    use super::ObjectId;
    let id = ObjectId::new(u32::MAX);
    assert_eq!(id.get(), u32::MAX);
}

#[test]
fn blob_id_max_u64_is_valid() {
    use super::BlobId;
    let id = BlobId::new(u64::MAX);
    assert_eq!(id.get(), u64::MAX);
}

#[test]
fn workflow_id_zero_is_valid() {
    let id = WorkflowId::new(0);
    assert_eq!(id.get(), 0);
}

#[test]
fn workflow_id_max_u32() {
    let id = WorkflowId::new(u32::MAX);
    assert_eq!(id.get(), u32::MAX);
}

#[test]
fn action_id_zero_is_valid() {
    use super::ActionId;
    let id = ActionId::new(0);
    assert_eq!(id.get(), 0);
}

#[test]
fn accessor_idx_as_usize() {
    use super::AccessorIdx;
    let idx = AccessorIdx::new(42);
    assert_eq!(idx.as_usize(), 42);
}

#[test]
fn expr_idx_as_usize() {
    use super::ExprIdx;
    let idx = ExprIdx::new(13);
    assert_eq!(idx.as_usize(), 13);
}

#[test]
fn ids_from_str_valid() -> Result<(), String> {
    let step: StepIdx = "42".parse().map_err(|_| String::from("parse failed"))?;
    if step.get() != 42 {
        return Err(String::from("expected 42"));
    }
    Ok(())
}

#[test]
fn ids_from_str_invalid() {
    use super::SymbolId;
    let result: Result<SymbolId, _> = "not_a_number".parse();
    assert!(
        matches!(result, Err(_)),
        "non-numeric string must fail to parse"
    );
}

#[test]
fn workflow_digest_roundtrip() {
    use super::WorkflowDigest;
    let bytes = [0xAB_u8; 32];
    let digest = WorkflowDigest::from_bytes(bytes);
    assert_eq!(digest.as_bytes(), bytes);
}

#[test]
fn workflow_digest_zero_array() {
    use super::WorkflowDigest;
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    assert_eq!(digest.as_bytes(), [0u8; 32]);
}

// =========================================================================
// BLACKHAT security regression tests — IDs
// =========================================================================

// --- FanoutLimit::as_usize does not use unwrap_or ---

#[test]
fn fanout_limit_as_usize_zero() {
    use super::FanoutLimit;
    let limit = FanoutLimit::new(0);
    assert_eq!(limit.as_usize(), 0);
}

#[test]
fn fanout_limit_as_usize_max_u32() {
    use super::FanoutLimit;
    let limit = FanoutLimit::new(u32::MAX);
    // On all current platforms u32 fits in usize
    assert_eq!(limit.as_usize(), u32::MAX as usize);
}

#[test]
fn fanout_limit_as_usize_typical_value() {
    use super::FanoutLimit;
    let limit = FanoutLimit::new(1000);
    assert_eq!(limit.as_usize(), 1000);
}

// =========================================================================
// Edge-case tests — ID types: ordering, FromStr, BranchIdx, MaxAttempts,
// RetryCount, BranchCount, FanoutLimit, WorkflowDigest
// =========================================================================

// --- Ordering comparisons ---

#[test]
fn step_idx_ordering() {
    let a = StepIdx::new(0);
    let b = StepIdx::new(1);
    let c = StepIdx::new(u16::MAX);
    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
    assert!(a <= a);
    assert!(c >= c);
}

#[test]
fn slot_idx_ordering() {
    let a = SlotIdx::new(0);
    let b = SlotIdx::new(100);
    let c = SlotIdx::new(u16::MAX);
    assert!(a < b);
    assert!(b < c);
    assert!(a != c);
}

#[test]
fn seq_no_ordering() {
    let a = SeqNo::new(0);
    let b = SeqNo::new(u64::MAX);
    assert!(a < b);
}

#[test]
fn run_id_ordering() {
    let a = RunId::new(0);
    let b = RunId::new(u64::MAX);
    assert!(a < b);
}

#[test]
fn workflow_id_ordering() {
    let a = WorkflowId::new(0);
    let b = WorkflowId::new(u32::MAX);
    assert!(a < b);
}

// --- FromStr parsing edge cases ---

#[test]
fn from_str_parses_zero() -> Result<(), String> {
    let idx: StepIdx = "0".parse().map_err(|_| String::from("parse failed"))?;
    if idx.get() != 0 {
        return Err(String::from("expected 0"));
    }
    Ok(())
}

#[test]
fn from_str_parses_max_u16() -> Result<(), String> {
    let idx: SlotIdx = "65535".parse().map_err(|_| String::from("parse failed"))?;
    if idx.get() != u16::MAX {
        return Err(String::from("expected u16::MAX"));
    }
    Ok(())
}

#[test]
fn from_str_parses_max_u32() -> Result<(), String> {
    let id: WorkflowId = "4294967295"
        .parse()
        .map_err(|_| String::from("parse failed"))?;
    if id.get() != u32::MAX {
        return Err(String::from("expected u32::MAX"));
    }
    Ok(())
}

#[test]
fn from_str_parses_max_u64() -> Result<(), String> {
    let id: RunId = "18446744073709551615"
        .parse()
        .map_err(|_| String::from("parse failed"))?;
    if id.get() != u64::MAX {
        return Err(String::from("expected u64::MAX"));
    }
    Ok(())
}

#[test]
fn from_str_rejects_empty_string() {
    let result: Result<StepIdx, _> = "".parse();
    assert!(matches!(result, Err(_)), "empty string must fail to parse");
}

#[test]
fn from_str_rejects_negative() {
    let result: Result<StepIdx, _> = "-1".parse();
    assert!(
        matches!(result, Err(_)),
        "negative string must fail to parse"
    );
}

#[test]
fn from_str_rejects_overflow_for_u16() {
    let result: Result<StepIdx, _> = "65536".parse();
    assert!(matches!(result, Err(_)), "u16 overflow must fail to parse");
}

#[test]
fn from_str_rejects_overflow_for_u32() {
    let result: Result<WorkflowId, _> = "4294967296".parse();
    assert!(matches!(result, Err(_)), "u32 overflow must fail to parse");
}

#[test]
fn from_str_rejects_leading_whitespace() {
    let result: Result<StepIdx, _> = " 42".parse();
    assert!(matches!(result, Err(_)), "leading whitespace must fail");
}

// --- BranchIdx edge cases ---

#[test]
fn branch_idx_zero_is_first() {
    use super::BranchIdx;
    let idx = BranchIdx::new(0);
    assert!(idx.is_first());
    assert_eq!(idx.get(), 0);
}

#[test]
fn branch_idx_one_is_not_first() {
    use super::BranchIdx;
    let idx = BranchIdx::new(1);
    assert!(!idx.is_first());
}

#[test]
fn branch_idx_max_value() {
    use super::BranchIdx;
    let idx = BranchIdx::new(u16::MAX);
    assert!(!idx.is_first());
    assert_eq!(idx.get(), u16::MAX);
}

#[test]
fn branch_idx_from_u16() {
    use super::BranchIdx;
    let idx = BranchIdx::from(7u16);
    assert_eq!(idx.get(), 7);
}

// --- MaxAttempts edge cases ---

#[test]
fn max_attempts_one_is_valid() -> Result<(), String> {
    use super::MaxAttempts;
    let attempts = MaxAttempts::try_new(1).map_err(|e| e.to_string())?;
    assert_eq!(attempts.get(), 1);
    Ok(())
}

#[test]
fn max_attempts_max_u16_is_valid() -> Result<(), String> {
    use super::MaxAttempts;
    let attempts = MaxAttempts::try_new(u16::MAX).map_err(|e| e.to_string())?;
    assert_eq!(attempts.get(), u16::MAX);
    Ok(())
}

#[test]
fn max_attempts_zero_is_rejected() {
    use super::MaxAttempts;
    let result = MaxAttempts::try_new(0);
    assert!(
        matches!(
            result,
            Err(crate::EngineError::InvalidRepeatState)
        ),
        "max_attempts=0 must be rejected"
    );
}

// --- RetryCount edge cases ---

#[test]
fn retry_count_zero_is_valid() {
    use super::RetryCount;
    let count = RetryCount::new(0);
    assert_eq!(count.get(), 0);
}

#[test]
fn retry_count_next_increments() {
    use super::RetryCount;
    let count = RetryCount::new(0);
    let next = count.next();
    assert_eq!(next.get(), 1);
}

#[test]
fn retry_count_next_saturates_at_max() {
    use super::RetryCount;
    let count = RetryCount::new(u16::MAX);
    let next = count.next();
    assert_eq!(next.get(), u16::MAX);
}

#[test]
fn retry_count_max_u16() {
    use super::RetryCount;
    let count = RetryCount::new(u16::MAX);
    assert_eq!(count.get(), u16::MAX);
}

// --- BranchCount edge cases ---

#[test]
fn branch_count_zero_is_valid() {
    use super::BranchCount;
    let count = BranchCount::new(0);
    assert_eq!(count.get(), 0);
}

#[test]
fn branch_count_max_u16() {
    use super::BranchCount;
    let count = BranchCount::new(u16::MAX);
    assert_eq!(count.get(), u16::MAX);
}

#[test]
fn branch_count_from_u16() {
    use super::BranchCount;
    let count = BranchCount::from(5u16);
    assert_eq!(count.get(), 5);
}

// --- FanoutLimit edge cases ---

#[test]
fn fanout_limit_from_u32() {
    use super::FanoutLimit;
    let limit = FanoutLimit::from(100u32);
    assert_eq!(limit.get(), 100);
}

#[test]
fn fanout_limit_zero_get() {
    use super::FanoutLimit;
    let limit = FanoutLimit::new(0);
    assert_eq!(limit.get(), 0);
}

// --- WorkflowDigest edge cases ---

#[test]
fn workflow_digest_equality() {
    use super::WorkflowDigest;
    let a = WorkflowDigest::from_bytes([0xFF; 32]);
    let b = WorkflowDigest::from_bytes([0xFF; 32]);
    assert_eq!(a, b);
}

#[test]
fn workflow_digest_inequality() {
    use super::WorkflowDigest;
    let a = WorkflowDigest::from_bytes([0x00; 32]);
    let b = WorkflowDigest::from_bytes([0xFF; 32]);
    assert_ne!(a, b);
}

#[test]
fn workflow_digest_single_byte_difference() {
    use super::WorkflowDigest;
    let mut bytes_a = [0u8; 32];
    let bytes_b = [0u8; 32];
    bytes_a[31] = 1;
    let a = WorkflowDigest::from_bytes(bytes_a);
    let b = WorkflowDigest::from_bytes(bytes_b);
    assert_ne!(a, b);
}

// --- AccessorIdx checked arithmetic ---

#[test]
fn accessor_idx_as_usize_boundary() {
    use super::AccessorIdx;
    let idx = AccessorIdx::new(0);
    assert_eq!(idx.as_usize(), 0);
    let idx_max = AccessorIdx::new(u16::MAX);
    assert_eq!(idx_max.as_usize(), usize::from(u16::MAX));
}

// --- ExprIdx checked arithmetic ---

#[test]
fn expr_idx_as_usize_boundary() {
    use super::ExprIdx;
    let idx = ExprIdx::new(0);
    assert_eq!(idx.as_usize(), 0);
    let idx_max = ExprIdx::new(u16::MAX);
    assert_eq!(idx_max.as_usize(), usize::from(u16::MAX));
}

// --- ConstIdx as_usize boundary ---

#[test]
fn const_idx_as_usize_boundary() {
    use super::ConstIdx;
    let idx = ConstIdx::new(0);
    assert_eq!(idx.as_usize(), 0);
    let idx_max = ConstIdx::new(u16::MAX);
    assert_eq!(idx_max.as_usize(), usize::from(u16::MAX));
}

// --- Copy and Clone for all ID types ---

#[test]
fn id_types_copy_trait() {
    let step = StepIdx::new(42);
    let step_copy = step;
    assert_eq!(step, step_copy);

    let slot = SlotIdx::new(7);
    let slot_copy = slot;
    assert_eq!(slot, slot_copy);

    let run = RunId::new(99);
    let run_copy = run;
    assert_eq!(run, run_copy);
}

// --- Hash consistency for WorkflowDigest ---

#[test]
fn workflow_digest_hash_consistency() {
    use super::WorkflowDigest;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let a = WorkflowDigest::from_bytes([0xAB; 32]);
    let b = WorkflowDigest::from_bytes([0xAB; 32]);

    let mut hasher_a = DefaultHasher::new();
    let mut hasher_b = DefaultHasher::new();
    a.hash(&mut hasher_a);
    b.hash(&mut hasher_b);

    assert_eq!(hasher_a.finish(), hasher_b.finish());
}

// --- New tests for constants, Debug, Ord, FromStr, Hash ---

#[test]
fn step_idx_zero_constant_is_zero() {
    assert_eq!(StepIdx::ZERO.get(), 0);
    assert_eq!(StepIdx::ZERO.as_usize(), 0);
}

#[test]
fn step_idx_min_is_zero() {
    assert_eq!(StepIdx::MIN.get(), 0);
}

#[test]
fn step_idx_max_is_u16_max() {
    assert_eq!(StepIdx::MAX.get(), u16::MAX);
}

#[test]
fn action_id_max_u16_is_valid() {
    use super::ActionId;
    let id = ActionId::new(u16::MAX);
    assert_eq!(id.get(), u16::MAX);
}

#[test]
fn expr_idx_max_u16_is_valid() {
    use super::ExprIdx;
    let idx = ExprIdx::new(u16::MAX);
    assert_eq!(idx.get(), u16::MAX);
    assert_eq!(idx.as_usize(), usize::from(u16::MAX));
}

#[test]
fn const_idx_max_u16_is_valid() {
    use super::ConstIdx;
    let idx = ConstIdx::new(u16::MAX);
    assert_eq!(idx.get(), u16::MAX);
    assert_eq!(idx.as_usize(), usize::from(u16::MAX));
}

#[test]
fn debug_trait_contains_inner_value() {
    use super::ExprIdx;
    let idx = ExprIdx::new(42);
    let debug = format!("{idx:?}");
    assert!(
        debug.contains("42"),
        "Debug output must contain inner value 42, got: {debug}"
    );
}

#[test]
fn ord_comparison_expr_idx() {
    use super::ExprIdx;
    let a = ExprIdx::new(0);
    let b = ExprIdx::new(100);
    let c = ExprIdx::new(u16::MAX);
    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
    assert!(a <= a);
    assert!(c >= c);
}

#[test]
fn ord_comparison_action_id() {
    use super::ActionId;
    let a = ActionId::new(0);
    let b = ActionId::new(1);
    let c = ActionId::new(u16::MAX);
    assert!(a < b);
    assert!(b < c);
    assert!(a != c);
}

#[test]
fn from_str_parses_max_u16_for_action_id() -> Result<(), String> {
    use super::ActionId;
    let id: ActionId = "65535".parse().map_err(|_| String::from("parse failed"))?;
    if id.get() != u16::MAX {
        return Err(String::from("expected u16::MAX"));
    }
    Ok(())
}

#[test]
fn hash_consistency_for_equal_step_idx() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let a = StepIdx::new(42);
    let b = StepIdx::new(42);
    let mut hasher_a = DefaultHasher::new();
    let mut hasher_b = DefaultHasher::new();
    a.hash(&mut hasher_a);
    b.hash(&mut hasher_b);
    assert_eq!(hasher_a.finish(), hasher_b.finish());
}
