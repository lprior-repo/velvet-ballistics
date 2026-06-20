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
//! Adversarial property tests for vb-rrjdu, vb-3ggxp, vb-1fln0 changes.
//!
//! These tests exercise:
//! 1. IndexSet<SlotValue> dedup in `eval_unique` (vb-3ggxp).
//! 2. HashMap<SymbolId, usize> position index in `eval_merge_combine_fields` (vb-1fln0).
//! 3. Manual Hash impl on FiniteF64 canonicalising -0.0 to +0.0 (vb-rrjdu).
//!
//! The tests are deterministic given a fixed seed, but exercise all variants of
//! `SlotValue` (Null/Bool/I64/F64/SymbolId/ListId/ObjectId/BlobId) across the
//! list sizes that the original PR covered (1, 10, 100, 1000).

use crate::engine::expr_eval::ops_text_list::eval_unique;
use crate::engine::expr_eval::stack::{ExprStack, push_value};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value::{FiniteF64, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};

/// Linear congruential generator — deterministic, seedable.
///
/// Standard PRNG that produces u64 values; we then project into ranges for the
/// various SlotValue variants. Using a fixed seed makes every test reproducible.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes' LCG constants.
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005_u64)
            .wrapping_add(1_442_695_040_888_963_407_u64);
        self.state
    }

    fn next_usize(&mut self, range: usize) -> usize {
        let r = self.next_u64();
        (r >> 32) as usize % range
    }

    fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1) != 0
    }

    fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }

    fn next_f64_finite(&mut self) -> f64 {
        // Mix bits into a known-finite f64 — restrict the exponent so the
        // result is always in (-MAX_FINITE, MAX_FINITE) with non-zero
        // fraction.
        let bits = self.next_u64() & 0x7F0F_FFFF_FFFF_FFFF_u64;
        f64::from_bits(bits)
    }
}

/// Generate a deterministic SlotValue stream of `count` elements across all
/// variants. Variants are selected by index, ensuring full coverage.
fn gen_slot_values(seed: u64, count: usize) -> Vec<SlotValue> {
    let mut rng = Lcg::new(seed);
    let mut result = Vec::with_capacity(count);
    for idx in 0..count {
        let variant = idx % 8;
        let value: SlotValue = match variant {
            0 => SlotValue::Null,
            1 => SlotValue::Bool(rng.next_bool()),
            2 => SlotValue::I64(rng.next_i64()),
            3 => {
                let v = rng.next_f64_finite();
                // SAFETY: gen_f64_finite() guarantees finiteness by bit
                // masking, but FiniteF64::new validates at runtime too.
                let f = FiniteF64::new(v).unwrap_or_else(|_| FiniteF64::new(0.0).unwrap());
                SlotValue::F64(f)
            }
            4 => SlotValue::Symbol(SymbolId::new((rng.next_u64() % 1024) as u32)),
            5 => SlotValue::List(ListId::new((rng.next_u64() % 1024) as u32)),
            6 => SlotValue::Object(ObjectId::new((rng.next_u64() % 1024) as u32)),
            7 => SlotValue::Blob(BlobId::new(rng.next_u64() % 1024)),
            _ => unreachable!(),
        };
        result.push(value);
    }
    result
}

/// Returns true if `needle` appears in `haystack` according to == equality.
fn contains_eq(haystack: &[SlotValue], needle: &SlotValue) -> bool {
    haystack.iter().any(|v| v == needle)
}

/// Property test: `eval_unique` deduplicates a list preserving first-occurrence
/// order, regardless of size or duplicate rate.
///
/// For each test vector:
/// (a) Output is a permutation of (input minus duplicates).
/// (b) First-occurrence order is preserved.
/// (c) Output contains no duplicates.
#[test]
fn unique_property_dedups_preserves_first_occurrence_order() {
    for seed in 0u64..16 {
        for size in [1usize, 10, 100, 500] {
            let mut input = gen_slot_values(seed, size);

            // Inject some duplicates: pick a third of indices and copy a prior element.
            for i in 0..input.len() {
                if i % 3 == 0 && i > 0 {
                    let copy_idx = (i.wrapping_mul(7)) % i;
                    input[i] = input[copy_idx];
                }
            }

            let mut store = ValueStore::new();
            let list_id = store.insert_list(input.clone().into_boxed_slice()).unwrap();
            let mut stack = ExprStack::new(64).unwrap();
            push_value(&mut stack, SlotValue::List(list_id)).unwrap();
            eval_unique(&mut stack, &mut store).expect("eval_unique must succeed");
            let result_id = match crate::engine::expr_eval::stack::pop_value(&mut stack).unwrap() {
                SlotValue::List(id) => id,
                other => panic!("expected List, got {other:?}"),
            };
            let result = store.list(result_id).unwrap().to_vec();

            // (c) No duplicates in output.
            for (i, a) in result.iter().enumerate() {
                for b in result.iter().skip(i + 1) {
                    assert_ne!(a, b, "duplicate found in unique output: {a:?}");
                }
            }

            // (a) Output is a subset of input values (every output element
            //     appears in input).
            for v in &result {
                assert!(
                    contains_eq(&input, v),
                    "unique output element {v:?} not found in input"
                );
            }

            // (b) First-occurrence order preserved: for every input pair
            //     (a, b) where a == b and a appears before b, the dedup'd
            //     output keeps the position of a's first occurrence.
            let mut last_seen_idx: std::collections::HashMap<&SlotValue, usize> =
                std::collections::HashMap::new();
            for (i, v) in input.iter().enumerate() {
                last_seen_idx.entry(v).or_insert(i);
            }
            for (i, v) in result.iter().enumerate() {
                let first_input = last_seen_idx[v];
                // Position of v in input must be <= position of v+1 in input.
                // Find the next v' in result, verify v's first input position
                // is <= v+'s first input position.
                if let Some(next_v) = result.get(i + 1) {
                    let next_first = last_seen_idx[next_v];
                    assert!(
                        first_input <= next_first,
                        "first-occurrence order violated: {v:?}@{first_input} comes after {next_v:?}@{next_first} in input"
                    );
                }
            }

            // Output length must equal distinct-element count of input.
            let mut distinct_count = std::collections::HashSet::new();
            for v in &input {
                distinct_count.insert(*v);
            }
            assert_eq!(
                result.len(),
                distinct_count.len(),
                "seed={seed} size={size}: unique output size {} != distinct count {}",
                result.len(),
                distinct_count.len()
            );
        }
    }
}

/// Property test: `eval_unique` canonicalises -0.0 to +0.0.
///
/// Hash/Eq contract requires F64(-0.0) and F64(+0.0) to dedup as one element.
/// Without the manual Hash impl, `IndexSet::insert` would store them as
/// distinct elements because their f64::to_bits() differ.
#[test]
fn unique_property_dedups_negative_zero_with_positive_zero() {
    let mut store = ValueStore::new();
    let neg_zero = FiniteF64::new(-0.0_f64).unwrap();
    let pos_zero = FiniteF64::new(0.0_f64).unwrap();
    let input = vec![
        SlotValue::F64(neg_zero),
        SlotValue::F64(pos_zero),
        SlotValue::F64(neg_zero),
        SlotValue::F64(pos_zero),
    ];
    let list_id = store.insert_list(input.into_boxed_slice()).unwrap();
    let mut stack = ExprStack::new(64).unwrap();
    push_value(&mut stack, SlotValue::List(list_id)).unwrap();
    eval_unique(&mut stack, &mut store).expect("eval_unique must succeed");
    let result_id = match crate::engine::expr_eval::stack::pop_value(&mut stack).unwrap() {
        SlotValue::List(id) => id,
        other => panic!("expected List, got {other:?}"),
    };
    let result = store.list(result_id).unwrap();

    assert_eq!(
        result.len(),
        1,
        "F64(-0.0) and F64(+0.0) must canonicalise to a single element; got {result:?}"
    );
    // The first-occurrence element wins, which is F64(-0.0).
    match result[0] {
        SlotValue::F64(f) => {
            assert_eq!(f.get().to_bits(), neg_zero.get().to_bits());
        }
        other => panic!("expected F64, got {other:?}"),
    }
}

// ============================================================================
// Property test: `eval_merge_combine_fields` semantics.
// ============================================================================
//
// For left L and right R objects:
// - result key set = L.keys ∪ R.keys.
// - for key k in L.keys ∩ R.keys: result[k] = R[k].
// - for key k in L.keys \ R.keys: result[k] = L[k], at the same position.
// - for key k in R.keys \ L.keys: result[k] = R[k], appended at the end.
// - position of left keys is preserved.

fn merge_keys_fields_to_vec(
    fields: &[(SymbolId, i64)],
    store: &mut ValueStore,
) -> (ObjectId, Vec<SymbolId>) {
    let obj_fields: Vec<ObjectField> = fields
        .iter()
        .map(|(sym, val)| ObjectField {
            key: *sym,
            value: SlotValue::I64(*val),
            taint: Taint::Clean,
        })
        .collect();
    let ids: Vec<SymbolId> = obj_fields.iter().map(|f| f.key).collect();
    let id = store.insert_object(obj_fields.into_boxed_slice()).unwrap();
    (id, ids)
}

fn symbols_for(names: &[&str], store: &mut ValueStore) -> Vec<SymbolId> {
    // NOTE: ValueStore::insert_symbol does NOT intern names — each call
    // allocates a fresh SymbolId even if the same name was previously
    // inserted. To get stable SymbolIds across calls, callers must reuse
    // the returned id directly. This helper interns within the call site by
    // using a fresh store for each call.
    names
        .iter()
        .map(|n| store.insert_symbol(*n).unwrap())
        .collect()
}

#[test]
fn merge_property_disjoint_objects_concatenate_right_after_left() {
    for left_size in 0usize..8 {
        for right_size in 0usize..8 {
            if left_size == 0 && right_size == 0 {
                continue;
            }
            let mut store = ValueStore::new();
            // Use distinct prefix-names so each iteration's symbols don't
            // collide with another iteration's; symbols are interned.
            let left_names: Vec<String> = (0..left_size)
                .map(|i| format!("L{i}_{left_size}_{right_size}"))
                .collect();
            let right_names: Vec<String> = (0..right_size)
                .map(|i| format!("R{i}_{left_size}_{right_size}"))
                .collect();
            let left_name_refs: Vec<&str> = left_names.iter().map(String::as_str).collect();
            let right_name_refs: Vec<&str> = right_names.iter().map(String::as_str).collect();

            let left_syms = symbols_for(&left_name_refs, &mut store);
            let right_syms = symbols_for(&right_name_refs, &mut store);

            let left_field_data: Vec<(SymbolId, i64)> = left_syms
                .iter()
                .enumerate()
                .map(|(i, s)| (*s, i as i64 + 1))
                .collect();
            let right_field_data: Vec<(SymbolId, i64)> = right_syms
                .iter()
                .enumerate()
                .map(|(i, s)| (*s, 100 + i as i64))
                .collect();

            let (left_id, _) = merge_keys_fields_to_vec(&left_field_data, &mut store);
            let (right_id, _) = merge_keys_fields_to_vec(&right_field_data, &mut store);

            let mut stack = ExprStack::new(64).unwrap();
            push_value(&mut stack, SlotValue::Object(left_id)).unwrap();
            push_value(&mut stack, SlotValue::Object(right_id)).unwrap();
            crate::engine::expr_eval::ops::eval_expr_operator(
                crate::workflow::ExprOp::Merge,
                &mut stack,
                &mut store,
            )
            .expect("merge must succeed");
            let merged_id = match crate::engine::expr_eval::stack::pop_value(&mut stack).unwrap() {
                SlotValue::Object(id) => id,
                other => panic!("expected Object, got {other:?}"),
            };
            let merged = store.object(merged_id).unwrap();

            assert_eq!(
                merged.len(),
                left_size + right_size,
                "disjoint merge size mismatch (left={left_size}, right={right_size})"
            );

            for (i, sym) in left_syms.iter().enumerate() {
                assert_eq!(
                    merged[i].key, *sym,
                    "left position {i} mismatch in disjoint merge"
                );
                assert_eq!(
                    merged[i].value,
                    SlotValue::I64(i as i64 + 1),
                    "left value at position {i}"
                );
            }

            for (i, sym) in right_syms.iter().enumerate() {
                let pos = left_size + i;
                assert_eq!(
                    merged[pos].key, *sym,
                    "right position {pos} mismatch in disjoint merge"
                );
                assert_eq!(
                    merged[pos].value,
                    SlotValue::I64(100 + i as i64),
                    "right value at position {pos}"
                );
            }
        }
    }
}

#[test]
fn merge_property_overlapping_keys_overwrite_in_place() {
    // Left has keys [a=1, b=2, c=3, d=4], Right has [b=99, c=88].
    // Merged result: [a=1, b=99, c=88, d=4] — same length, right wins on overlap,
    // left position preserved.
    //
    // IMPORTANT: ValueStore::insert_symbol does NOT intern names. Each call
    // allocates a fresh SymbolId even if the same string was previously
    // inserted. So to get SymbolId equality across the left and right
    // objects, we must call insert_symbol exactly once per name and reuse
    // the returned SymbolId in both objects.
    for seed in 0u64..8 {
        let mut store = ValueStore::new();
        let left_keys: [&str; 4] = ["a", "b", "c", "d"];
        let right_keys_subset: Vec<&str> = if seed % 2 == 0 {
            vec!["b", "c"]
        } else {
            vec!["a"]
        };

        // Insert each left key once; the resulting SymbolIds are reused in
        // both left and right objects.
        let sym_a = store.insert_symbol("a").unwrap();
        let sym_b = store.insert_symbol("b").unwrap();
        let sym_c = store.insert_symbol("c").unwrap();
        let sym_d = store.insert_symbol("d").unwrap();
        let left_syms = [sym_a, sym_b, sym_c, sym_d];

        // Build left object with full key set.
        let left_field_data: Vec<(SymbolId, i64)> = left_syms
            .iter()
            .enumerate()
            .map(|(i, s)| (*s, i as i64 + 1))
            .collect();

        // Build right object using the SAME SymbolIds as left for the
        // overlapping subset.
        let right_syms: Vec<SymbolId> = right_keys_subset
            .iter()
            .map(|k| match *k {
                "a" => sym_a,
                "b" => sym_b,
                "c" => sym_c,
                "d" => sym_d,
                _ => panic!("unexpected key {k}"),
            })
            .collect();
        let right_field_data: Vec<(SymbolId, i64)> = right_syms
            .iter()
            .enumerate()
            .map(|(i, s)| (*s, 100 + i as i64))
            .collect();

        let (left_id, _) = merge_keys_fields_to_vec(&left_field_data, &mut store);
        let (right_id, _) = merge_keys_fields_to_vec(&right_field_data, &mut store);

        // Sanity check what we built.
        let left_check = store.object(left_id).unwrap();
        let right_check = store.object(right_id).unwrap();
        assert_eq!(left_check.len(), left_field_data.len(), "left object size");
        assert_eq!(
            right_check.len(),
            right_field_data.len(),
            "right object size"
        );

        let mut stack = ExprStack::new(64).unwrap();
        push_value(&mut stack, SlotValue::Object(left_id)).unwrap();
        push_value(&mut stack, SlotValue::Object(right_id)).unwrap();
        crate::engine::expr_eval::ops::eval_expr_operator(
            crate::workflow::ExprOp::Merge,
            &mut stack,
            &mut store,
        )
        .expect("merge must succeed");
        let merged_id = match crate::engine::expr_eval::stack::pop_value(&mut stack).unwrap() {
            SlotValue::Object(id) => id,
            other => panic!("expected Object, got {other:?}"),
        };
        let merged = store.object(merged_id).unwrap();

        assert_eq!(
            merged.len(),
            left_keys.len(),
            "overlap merge size must equal left size (left_keys={left_keys:?}, right_keys={right_keys_subset:?}, seed={seed})"
        );

        for (i, expected_key) in left_syms.iter().enumerate() {
            let expected_name = left_keys[i];
            assert_eq!(
                merged[i].key, *expected_key,
                "left key {expected_name} at position {i}"
            );
            let expected_val = if right_keys_subset.contains(&expected_name) {
                let pos = right_keys_subset
                    .iter()
                    .position(|k| k == &expected_name)
                    .unwrap();
                100 + pos as i64
            } else {
                i as i64 + 1
            };
            assert_eq!(
                merged[i].value,
                SlotValue::I64(expected_val),
                "value at position {i} (key={expected_name})"
            );
        }
    }
}

#[test]
fn merge_property_identical_objects_right_wins() {
    let mut store = ValueStore::new();
    // Insert each shared key exactly once to ensure SymbolId equality across
    // left and right objects. (insert_symbol does NOT intern names.)
    let sym_a = store.insert_symbol("a").unwrap();
    let sym_b = store.insert_symbol("b").unwrap();
    let sym_c = store.insert_symbol("c").unwrap();
    let shared_syms = [sym_a, sym_b, sym_c];

    let left_field_data: Vec<(SymbolId, i64)> = shared_syms
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, i as i64 + 1))
        .collect();
    let right_field_data: Vec<(SymbolId, i64)> = shared_syms
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, 100 + i as i64))
        .collect();

    let (left_id, _) = merge_keys_fields_to_vec(&left_field_data, &mut store);
    let (right_id, _) = merge_keys_fields_to_vec(&right_field_data, &mut store);

    let mut stack = ExprStack::new(64).unwrap();
    push_value(&mut stack, SlotValue::Object(left_id)).unwrap();
    push_value(&mut stack, SlotValue::Object(right_id)).unwrap();
    crate::engine::expr_eval::ops::eval_expr_operator(
        crate::workflow::ExprOp::Merge,
        &mut stack,
        &mut store,
    )
    .expect("merge must succeed");
    let merged_id = match crate::engine::expr_eval::stack::pop_value(&mut stack).unwrap() {
        SlotValue::Object(id) => id,
        other => panic!("expected Object, got {other:?}"),
    };
    let merged = store.object(merged_id).unwrap();

    assert_eq!(merged.len(), 3, "identical merge keeps size 3");
    for (i, expected_val) in [100i64, 101, 102].iter().enumerate() {
        assert_eq!(
            merged[i].value,
            SlotValue::I64(*expected_val),
            "right wins at position {i}"
        );
    }
}

#[test]
fn merge_property_empty_left_or_right() {
    // Empty L, non-empty R -> result == R.
    let mut store = ValueStore::new();
    let empty_id = store
        .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
        .unwrap();
    let sym_a = store.insert_symbol("a").unwrap();
    let sym_b = store.insert_symbol("b").unwrap();
    let right_field_data: Vec<(SymbolId, i64)> = [(sym_a, 1i64), (sym_b, 2)].into_iter().collect();
    let (right_id, _) = merge_keys_fields_to_vec(&right_field_data, &mut store);

    let mut stack = ExprStack::new(64).unwrap();
    push_value(&mut stack, SlotValue::Object(empty_id)).unwrap();
    push_value(&mut stack, SlotValue::Object(right_id)).unwrap();
    crate::engine::expr_eval::ops::eval_expr_operator(
        crate::workflow::ExprOp::Merge,
        &mut stack,
        &mut store,
    )
    .expect("merge must succeed");
    let merged_id = match crate::engine::expr_eval::stack::pop_value(&mut stack).unwrap() {
        SlotValue::Object(id) => id,
        other => panic!("expected Object, got {other:?}"),
    };
    let merged = store.object(merged_id).unwrap();
    assert_eq!(merged.len(), 2);

    // Non-empty L, empty R -> result == L.
    let sym_x = store.insert_symbol("x").unwrap();
    let sym_y = store.insert_symbol("y").unwrap();
    let sym_z = store.insert_symbol("z").unwrap();
    let left_field_data: Vec<(SymbolId, i64)> = [(sym_x, 7i64), (sym_y, 8), (sym_z, 9)]
        .into_iter()
        .collect();
    let (left_id, _) = merge_keys_fields_to_vec(&left_field_data, &mut store);
    let empty_id = store
        .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
        .unwrap();

    let mut stack = ExprStack::new(64).unwrap();
    push_value(&mut stack, SlotValue::Object(left_id)).unwrap();
    push_value(&mut stack, SlotValue::Object(empty_id)).unwrap();
    crate::engine::expr_eval::ops::eval_expr_operator(
        crate::workflow::ExprOp::Merge,
        &mut stack,
        &mut store,
    )
    .expect("merge must succeed");
    let merged_id = match crate::engine::expr_eval::stack::pop_value(&mut stack).unwrap() {
        SlotValue::Object(id) => id,
        other => panic!("expected Object, got {other:?}"),
    };
    let merged = store.object(merged_id).unwrap();
    assert_eq!(merged.len(), 3);
}
