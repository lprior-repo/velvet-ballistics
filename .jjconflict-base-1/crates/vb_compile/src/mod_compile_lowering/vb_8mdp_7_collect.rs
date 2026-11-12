#![cfg(test)]
#![forbid(unsafe_code)]

//! Proptest: vb_8mdp_7_lower_canonical_collect_emission_properties
//!
//! Behaviors covered: B-013, B-014
//!
//! Invariants (lexical N-step lowering; production is
//! `mod_compile_lowering::bounded_collect::lower_bounded_collect`,
//! aliased below as `lower_canonical_collect`):
//!   I1: single-step bodies emit exactly 4 nodes (N + 3 with N = 1)
//!   I2: Node IDs are consecutive offsets from the input StepIdx
//!       (id, id+1, ..., id+N+2)
//!   I3: CollectNext and CollectFinish share the fresh collector slot;
//!       CollectStart carries the recorded source slot
//!   I4: CollectStart.limit == pages.unwrap_or(1) * items.unwrap_or(1)
//!       (total item bound; both-absent and zero bounds are rejected)
//!   I5: CollectStart.page_size == items.unwrap_or(1)
//!
//! Accessed via `super::part_02::bounded_collect` from within the
//! mod_compile_lowering module tree.

use crate::{StepAst, StepPrimitive};
use proptest::prelude::*;
use std::collections::HashMap;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

use super::part_02::{CanonicalScope, bounded_collect::CollectLowering};

// ─────────────────────────────────────────────────────────────────
// Generation strategies
// ─────────────────────────────────────────────────────────────────

/// Strategy for a single Set body (exactly one Set step).
fn single_set_body() -> impl Strategy<Value = Vec<StepAst>> {
    any::<i64>().prop_map(|value| {
        vec![StepAst {
            id: "set_step".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Set {
                output: "x".to_string(),
                value: value.to_string(),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]
    })
}

/// Collect input: source slot text, pages, items, body.
#[derive(Debug, Clone)]
struct CollectInput {
    source: String,
    pages: Option<u32>,
    items: Option<u32>,
    body: Vec<StepAst>,
}

fn collect_input_strategy() -> impl Strategy<Value = CollectInput> {
    (
        "[0-9]".prop_map(|s: String| s),
        any::<Option<u32>>(),
        any::<Option<u32>>(),
        single_set_body(),
    )
        .prop_map(|(source, pages, items, body)| CollectInput {
            source,
            pages,
            items,
            body,
        })
}

/// Valid StepIdx values that won't overflow when adding 3.
fn safe_step_idx() -> impl Strategy<Value = StepIdx> {
    (0u16..=65530u16).prop_map(StepIdx::new)
}

// ─────────────────────────────────────────────────────────────────
// Lexical scope plumbing
// ─────────────────────────────────────────────────────────────────

/// Owned lexical-scope maps backing a borrowed `CanonicalScope`.
/// Fixture bodies use step ids "s"/"set_step" and outputs "x"/"y",
/// so the fixed variable "batch" never collides (see
/// `validate_variable_name`, `validate_variable_capture`, `validate_body`).
#[derive(Default)]
struct ScopeMaps {
    outputs: HashMap<String, SlotIdx>,
    step_outputs: HashMap<String, SlotIdx>,
    slot_types: HashMap<SlotIdx, ExprType>,
}

/// Build a `CollectLowering` over borrowed scope maps.
fn scoped_collect<'a>(
    source: &'a str,
    pages: Option<u32>,
    items: Option<u32>,
    body: &'a [StepAst],
    next: Option<StepIdx>,
    maps: &'a ScopeMaps,
) -> CollectLowering<'a> {
    CollectLowering {
        source,
        variable: "batch",
        pages,
        items,
        body,
        next,
        scope: CanonicalScope {
            outputs: &maps.outputs,
            step_outputs: &maps.step_outputs,
            slot_types: &maps.slot_types,
            parent: None,
        },
    }
}

fn scoped_input<'a>(
    input: &'a CollectInput,
    next: Option<StepIdx>,
    maps: &'a ScopeMaps,
) -> CollectLowering<'a> {
    scoped_collect(
        &input.source,
        input.pages,
        input.items,
        &input.body,
        next,
        maps,
    )
}

#[path = "vb_8mdp_7_collect_proptests.rs"]
mod proptests;

#[path = "vb_8mdp_7_collect_tests.rs"]
mod unit_tests;
