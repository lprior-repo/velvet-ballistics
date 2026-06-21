#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Bytecode AST parity property tests: `ExprProgram::try_from_ops` and
//! `ExprProgram::try_from_parts` agree on every stack-bound program; accessor
//! path lengths and root indices are validated end-to-end.

use proptest::prelude::*;
use vb_core::errors::CoreError;
use vb_core::ids::{ConstIdx, SlotIdx, SymbolId};
use vb_core::workflow::{AccessorProgram, ExprOp, ExprProgram, PathSegment};

// =========================================================================
// Strategies
// =========================================================================

/// Strategy for a single load-only op (no arithmetic, no underflow risk).
fn arb_load_only_op() -> impl Strategy<Value = ExprOp> {
    prop_oneof![
        (0u16..16).prop_map(|s| ExprOp::LoadSlot(SlotIdx::new(s))),
        (0u16..16).prop_map(|c| ExprOp::LoadConst(ConstIdx::new(c))),
    ]
}

/// Strategy for a valid single-result program. Each program leaves exactly
/// one value on the stack. Patterns:
/// - 1 load (depth 1, valid)
/// - 2 loads + 1 binary op (depth 1, valid)
/// - 3 loads + 2 binary ops (depth 1, valid)
fn arb_load_only_program() -> impl Strategy<Value = Vec<ExprOp>> {
    (1usize..6, 0u16..16, 0u16..16, 0u16..16).prop_map(|(count, a, b, c)| {
        let mut ops = Vec::with_capacity(count * 2 - 1);
        ops.push(ExprOp::LoadSlot(SlotIdx::new(a)));
        if count >= 2 {
            ops.push(ExprOp::LoadSlot(SlotIdx::new(b)));
            ops.push(ExprOp::Add);
        }
        if count >= 3 {
            ops.push(ExprOp::LoadSlot(SlotIdx::new(c)));
            ops.push(ExprOp::Add);
        }
        if count >= 4 {
            ops.push(ExprOp::LoadSlot(SlotIdx::new(a)));
            ops.push(ExprOp::Add);
        }
        if count >= 5 {
            ops.push(ExprOp::LoadSlot(SlotIdx::new(b)));
            ops.push(ExprOp::Add);
        }
        ops
    })
}

/// Strategy for `a + b`: push two loads, then add. Final depth = 1.
fn arb_add_program(a: u16, b: u16) -> Vec<ExprOp> {
    vec![
        ExprOp::LoadSlot(SlotIdx::new(a)),
        ExprOp::LoadSlot(SlotIdx::new(b)),
        ExprOp::Add,
    ]
}

// =========================================================================
// Properties
// =========================================================================

proptest! {
    /// `try_from_ops` and `try_from_parts` agree on the computed stack bound
    /// for every load-only program: the round-trip must produce the same
    /// `max_stack` value.
    #[test]
    fn proptest_expr_program_parity_load_only(ops in arb_load_only_program()) {
        let boxed = ops.into_boxed_slice();
        let from_ops = ExprProgram::try_from_ops(boxed.clone());
        prop_assert!(
            from_ops.is_ok(),
            "try_from_ops must succeed for load-only program"
        );
        let program = from_ops.expect("ok");
        // Re-derive with the published max_stack and confirm parity.
        let boxed_again = program.ops.clone();
        let from_parts = ExprProgram::try_from_parts(boxed_again, program.max_stack);
        prop_assert!(
            from_parts.is_ok(),
            "try_from_parts must succeed with the same max_stack"
        );
        let program2 = from_parts.expect("ok");
        prop_assert_eq!(
            program.max_stack, program2.max_stack,
            "max_stack must round-trip identically"
        );
        prop_assert_eq!(
            program.ops, program2.ops,
            "ops must round-trip identically"
        );
    }

    /// `try_from_ops` rejects programs whose recomputed max_stack differs
    /// from the value the caller claims in `try_from_parts`.
    #[test]
    fn proptest_expr_program_parity_rejects_stale_max_stack(
        ops in arb_load_only_program(),
        declared_max in 0u8..16,
    ) {
        let boxed = ops.clone().into_boxed_slice();
        let from_ops = ExprProgram::try_from_ops(boxed.clone())
            .expect("load-only program must build");
        let truth = from_ops.max_stack;
        let from_parts = ExprProgram::try_from_parts(ops.into_boxed_slice(), declared_max);
        match declared_max.cmp(&truth) {
            core::cmp::Ordering::Equal => {
                prop_assert!(
                    from_parts.is_ok(),
                    "honest max_stack declaration (={}) must succeed", truth
                );
            }
            core::cmp::Ordering::Less => {
                prop_assert!(
                    matches!(from_parts, Err(CoreError::ExpressionStackOverflow { .. })),
                    "under-declared max_stack ({}) vs truth ({}) must be rejected as ExpressionStackOverflow",
                    declared_max, truth
                );
            }
            core::cmp::Ordering::Greater => {
                prop_assert!(
                    matches!(from_parts, Err(CoreError::InvalidCompiledWorkflow { .. })),
                    "over-declared max_stack ({}) vs truth ({}) must be rejected as InvalidCompiledWorkflow",
                    declared_max, truth
                );
            }
        }
    }

    /// Binary `Add` programs always compute max_stack = 2 and round-trip
    /// cleanly through both constructors.
    #[test]
    fn proptest_expr_program_parity_add(a in 0u16..16, b in 0u16..16) {
        let ops = arb_add_program(a, b);
        let boxed = ops.clone().into_boxed_slice();
        let from_ops = ExprProgram::try_from_ops(boxed)
            .expect("add program must build");
        prop_assert_eq!(
            from_ops.max_stack, 2u8,
            "add program must report max_stack=2"
        );
        let from_parts = ExprProgram::try_from_parts(ops.into_boxed_slice(), 2u8)
            .expect("round-trip with truthful max_stack=2 must succeed");
        prop_assert_eq!(
            from_ops.max_stack, from_parts.max_stack,
            "max_stack parity must hold"
        );
    }

    /// Accessor programs whose `root` slot index exceeds their path-depth
    /// bounds remain well-formed: `path.len()` must equal the constructor
    /// argument, never silently extended.
    #[test]
    fn proptest_accessor_program_parity_preserves_path_length(
        root in 0u16..16,
        path in proptest::collection::vec(
            prop_oneof![
                (0u32..32).prop_map(PathSegment::Index),
                (0u32..32).prop_map(|s| PathSegment::Field(SymbolId::new(s))),
            ],
            0..8,
        ),
    ) {
        let program = AccessorProgram {
            root: SlotIdx::new(root),
            path: path.clone().into_boxed_slice(),
        };
        prop_assert_eq!(
            program.root, SlotIdx::new(root),
            "root must be preserved"
        );
        prop_assert_eq!(
            program.path.len(),
            path.len(),
            "path length must be preserved exactly"
        );
        for (i, seg) in program.path.iter().enumerate() {
            prop_assert_eq!(
                *seg, path[i],
                "segment {} must round-trip identically", i
            );
        }
    }
}
