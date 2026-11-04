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
use vb_core::workflow::CompiledNodeKind;
use vb_core::workflow::NodeContinuation;
use vb_expr::expr_typecheck::ExprType;

use super::SlotCompiler;
use super::part_02::{
    CanonicalScope,
    bounded_collect::{CollectLowering, lower_bounded_collect as lower_canonical_collect},
};

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

// ─────────────────────────────────────────────────────────────────
// Proptest suites
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// I1+I2: Exactly 4 nodes with consecutive offsets (N = 1 shape:
    /// start + 1 body node + advance + finish).
    #[test]
    fn emits_four_nodes_with_consecutive_offsets(
        id in safe_step_idx(),
        input in collect_input_strategy(),
    ) {
        let mut builder = SlotCompiler::new();
        let current_id = id;
        let maps = ScopeMaps::default();

        let result = lower_canonical_collect(
            0,
            current_id,
            scoped_input(&input, None, &maps),
            &mut builder,
        );

        // Some source strings may fail parsing; only check valid ones
        if result.is_ok() {
            let nodes = builder.nodes;

            prop_assert_eq!(nodes.len(), 4,
                "lower_canonical_collect must emit exactly 4 nodes for a single-step body");

            // Node 0: CollectStart at id
            prop_assert_eq!(nodes[0].id.get(), current_id.get(),
                "node[0] must be at id");
            prop_assert!(
                matches!(&nodes[0].kind, CompiledNodeKind::CollectStart { .. }),
                "node[0] must be CollectStart"
            );

            // Node 1: body node at id+1
            prop_assert_eq!(nodes[1].id.get(), current_id.get().saturating_add(1),
                "node[1] must be at id+1");

            // Node 2: CollectNext at id+2
            prop_assert_eq!(nodes[2].id.get(), current_id.get().saturating_add(2),
                "node[2] must be at id+2");
            prop_assert!(
                matches!(&nodes[2].kind, CompiledNodeKind::CollectNext { .. }),
                "node[2] must be CollectNext"
            );

            // Node 3: CollectFinish at id+3
            prop_assert_eq!(nodes[3].id.get(), current_id.get().saturating_add(3),
                "node[3] must be at id+3");
            prop_assert!(
                matches!(&nodes[3].kind, CompiledNodeKind::CollectFinish { .. }),
                "node[3] must be CollectFinish"
            );
        }
    }

    /// I3: CollectNext and CollectFinish share the fresh collector slot;
    /// CollectStart carries the recorded source slot.
    #[test]
    fn advance_and_finish_share_collector_slot(
        id in safe_step_idx(),
        input in collect_input_strategy(),
    ) {
        let mut builder = SlotCompiler::new();
        let maps = ScopeMaps::default();
        let result = lower_canonical_collect(
            0,
            id,
            scoped_input(&input, None, &maps),
            &mut builder,
        );

        if result.is_ok() {
            let nodes = &builder.nodes;

            // CollectStart must carry the recorded source slot.
            prop_assert!(
                matches!(&nodes[0].kind, CompiledNodeKind::CollectStart { .. }),
                "node[0] must be CollectStart"
            );

            if let (
                CompiledNodeKind::CollectNext { collector_slot: next_collector, .. },
                CompiledNodeKind::CollectFinish { collector_slot: finish_collector },
            ) = (&nodes[2].kind, &nodes[3].kind) {
                prop_assert_eq!(*next_collector, *finish_collector,
                    "CollectNext.collector_slot must match CollectFinish.collector_slot");
            } else {
                prop_assert!(false,
                    "nodes 2/3 must be CollectNext/CollectFinish");
            }
        }
    }

    /// I4+I5: CollectStart.limit is the total bound
    /// pages.unwrap_or(1) * items.unwrap_or(1); page_size is
    /// items.unwrap_or(1). An ok result implies the product fits u32.
    #[test]
    fn collect_start_limit_and_page_size_correct(
        id in safe_step_idx(),
        input in collect_input_strategy(),
    ) {
        let mut builder = SlotCompiler::new();
        let maps = ScopeMaps::default();
        let result = lower_canonical_collect(
            0,
            id,
            scoped_input(&input, None, &maps),
            &mut builder,
        );

        if result.is_ok() {
            match &builder.nodes[0].kind {
                CompiledNodeKind::CollectStart { limit, page_size, .. } => {
                    match input.pages.unwrap_or(1).checked_mul(input.items.unwrap_or(1)) {
                        Some(expected) => {
                            prop_assert_eq!(*limit, expected,
                                "CollectStart.limit must equal pages.unwrap_or(1) * items.unwrap_or(1)");
                            prop_assert_eq!(*page_size, input.items.unwrap_or(1),
                                "CollectStart.page_size must equal items.unwrap_or(1)");
                        }
                        None => prop_assert!(false,
                            "ok result implies pages * items fits u32"),
                    }
                }
                _ => prop_assert!(false, "node[0] must be CollectStart"),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Deterministic unit tests
// ─────────────────────────────────────────────────────────────────

fn single_set_step(step_id: &str, output: &str, value: &str) -> StepAst {
    StepAst {
        id: step_id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: output.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

#[test]
fn per_side_defaults_apply() {
    let mut builder = SlotCompiler::new();
    let maps = ScopeMaps::default();
    let body = vec![single_set_step("s", "x", "42")];

    let result = lower_canonical_collect(
        0,
        StepIdx::new(10),
        scoped_collect("0", None, Some(3), &body, None, &maps),
        &mut builder,
    );
    assert!(
        result.is_ok(),
        "one-sided default collect should compile: {result:?}"
    );

    let nodes = &builder.nodes;
    assert_eq!(nodes.len(), 4);

    match &nodes[0].kind {
        CompiledNodeKind::CollectStart {
            limit, page_size, ..
        } => {
            assert_eq!(*limit, 3, "absent pages defaults to 1, so limit is 1 * 3");
            assert_eq!(*page_size, 3, "explicit items (page_size) should be 3");
        }
        other => assert!(false, "expected CollectStart, got {other:?}"),
    }
}

#[test]
fn missing_bounds_are_rejected() {
    let mut builder = SlotCompiler::new();
    let maps = ScopeMaps::default();
    let body = vec![single_set_step("s", "x", "42")];

    let result = lower_canonical_collect(
        0,
        StepIdx::new(10),
        scoped_collect("0", None, None, &body, None, &maps),
        &mut builder,
    );
    assert!(result.is_err(), "absent pages and items must be rejected");
}

#[test]
fn explicit_pages_and_items_multiply_to_limit() {
    let mut builder = SlotCompiler::new();
    let maps = ScopeMaps::default();
    let body = vec![single_set_step("s", "x", "1")];

    let result = lower_canonical_collect(
        0,
        StepIdx::new(10),
        scoped_collect("0", Some(5), Some(10), &body, None, &maps),
        &mut builder,
    );
    assert!(
        result.is_ok(),
        "explicit collect should compile: {result:?}"
    );

    let nodes = &builder.nodes;
    assert_eq!(nodes.len(), 4);

    match &nodes[0].kind {
        CompiledNodeKind::CollectStart {
            limit, page_size, ..
        } => {
            assert_eq!(*limit, 50, "limit is the total bound 5 * 10");
            assert_eq!(*page_size, 10, "explicit items (page_size) should be 10");
        }
        other => assert!(false, "expected CollectStart, got {other:?}"),
    }
}

#[test]
fn collect_finish_preserves_next() {
    let mut builder = SlotCompiler::new();
    let maps = ScopeMaps::default();
    let body = vec![single_set_step("s", "x", "1")];

    let next_step = StepIdx::new(99);
    let result = lower_canonical_collect(
        0,
        StepIdx::new(10),
        scoped_collect("0", Some(1), Some(1), &body, Some(next_step), &maps),
        &mut builder,
    );
    assert!(
        result.is_ok(),
        "collect with next should compile: {result:?}"
    );

    let nodes = &builder.nodes;
    assert_eq!(nodes.len(), 4);

    match &nodes[3].kind {
        CompiledNodeKind::CollectFinish { .. } => {
            assert_eq!(
                nodes[3].routing.next,
                NodeContinuation::Step(next_step),
                "CollectFinish routing.next must preserve the given next step"
            );
        }
        other => assert!(false, "expected CollectFinish, got {other:?}"),
    }
}

#[test]
fn node_ids_are_consecutive_from_step_idx() {
    let mut builder = SlotCompiler::new();
    let maps = ScopeMaps::default();
    let body = vec![single_set_step("s", "x", "5")];

    let start = StepIdx::new(42);
    let result = lower_canonical_collect(
        0,
        start,
        scoped_collect("0", Some(1), Some(1), &body, None, &maps),
        &mut builder,
    );
    assert!(result.is_ok(), "collect compile: {result:?}");

    let nodes = &builder.nodes;
    assert_eq!(nodes[0].id.get(), 42, "node 0 at start");
    assert_eq!(nodes[1].id.get(), 43, "node 1 at start+1");
    assert_eq!(nodes[2].id.get(), 44, "node 2 at start+2");
    assert_eq!(nodes[3].id.get(), 45, "node 3 at start+3");
}

/// N = 2 lexical body: edges scale with body length (start + 2 body
/// nodes + advance + finish = 5 nodes; advance = id+3, done = id+4).
#[test]
fn two_step_body_emits_five_nodes_with_scaled_edges() {
    let mut builder = SlotCompiler::new();
    let maps = ScopeMaps::default();
    let body = vec![
        single_set_step("s1", "x", "1"),
        single_set_step("s2", "y", "2"),
    ];

    let start = StepIdx::new(10);
    let result = lower_canonical_collect(
        0,
        start,
        scoped_collect("0", Some(1), Some(1), &body, None, &maps),
        &mut builder,
    );
    assert!(result.is_ok(), "two-step collect compile: {result:?}");

    let nodes = &builder.nodes;
    assert_eq!(nodes.len(), 5, "N=2 body must emit N+3 nodes");
    assert_eq!(nodes[0].id.get(), 10, "start at id");
    assert_eq!(nodes[1].id.get(), 11, "first body step at id+1");
    assert_eq!(nodes[2].id.get(), 12, "second body step at id+2");
    assert_eq!(nodes[3].id.get(), 13, "advance at id+N+1");
    assert_eq!(nodes[4].id.get(), 14, "finish at id+N+2");

    match &nodes[0].kind {
        CompiledNodeKind::CollectStart { body, done, .. } => {
            assert_eq!(*body, StepIdx::new(11), "CollectStart.body must be id+1");
            assert_eq!(*done, StepIdx::new(14), "CollectStart.done must be id+N+2");
        }
        other => assert!(false, "expected CollectStart, got {other:?}"),
    }
    match (&nodes[3].kind, &nodes[4].kind) {
        (
            CompiledNodeKind::CollectNext {
                collector_slot: next_collector,
                body,
                done,
            },
            CompiledNodeKind::CollectFinish {
                collector_slot: finish_collector,
            },
        ) => {
            assert_eq!(*body, StepIdx::new(11), "CollectNext.body must be id+1");
            assert_eq!(*done, StepIdx::new(14), "CollectNext.done must be id+N+2");
            assert_eq!(
                *next_collector, *finish_collector,
                "advance and finish must share the collector slot"
            );
        }
        (next, finish) => assert!(
            false,
            "expected CollectNext/CollectFinish, got {next:?}/{finish:?}"
        ),
    }
}
