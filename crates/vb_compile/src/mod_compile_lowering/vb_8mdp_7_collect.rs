#![cfg(test)]
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

//! Proptest: vb_8mdp_7_lower_canonical_collect_emission_properties
//!
//! Behaviors covered: B-013, B-014
//!
//! Invariants:
//!   I1: lower_canonical_collect always emits exactly 4 nodes
//!   I2: Node IDs are consecutive offsets from the input StepIdx (id, id+1, id+2, id+3)
//!   I3: All nodes reference the source slot
//!   I4: CollectStart.limit == pages.unwrap_or(1)
//!   I5: CollectStart.page_size == items.unwrap_or(1)
//!
//! Accessed via `super::lower_canonical_collect` from within the
//! mod_compile_lowering module tree.

use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::CompiledNodeKind;
use vb_yaml::ast::{StepAst, StepPrimitive};

use super::part_03::CollectLowering;
use super::part_03::lower_canonical_collect;
use super::part_07::SlotCompiler;

// ─────────────────────────────────────────────────────────────────
// Generation strategies
// ─────────────────────────────────────────────────────────────────

/// Strategy for a single Set body (exactly one Set step).
fn single_set_body() -> impl Strategy<Value = Vec<StepAst>> {
    any::<i64>().prop_map(|value| vec![StepAst {
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
    }])
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
// Proptest suites
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// I1+I2: Exactly 4 nodes with consecutive offsets.
    #[test]
    fn emits_four_nodes_with_consecutive_offsets(
        id in safe_step_idx(),
        input in collect_input_strategy(),
    ) {
        let mut builder = SlotCompiler::new();
        let current_id = id;

        let result = lower_canonical_collect(
            0,
            current_id,
            CollectLowering {
                source: &input.source,
                pages: input.pages,
                items: input.items,
                body: &input.body,
                next: None,
            },
            &mut builder,
        );

        // Some source strings may fail parsing; only check valid ones
        if result.is_ok() {
            let nodes = builder.nodes;

            prop_assert_eq!(nodes.len(), 4,
                "lower_canonical_collect must emit exactly 4 nodes");

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

            // Node 2: CollectPage at id+2
            prop_assert_eq!(nodes[2].id.get(), current_id.get().saturating_add(2),
                "node[2] must be at id+2");
            prop_assert!(
                matches!(&nodes[2].kind, CompiledNodeKind::CollectPage { .. }),
                "node[2] must be CollectPage"
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

    /// I3: All nodes reference the source slot.
    #[test]
    fn all_nodes_reference_source_slot(
        id in safe_step_idx(),
        input in collect_input_strategy(),
    ) {
        let mut builder = SlotCompiler::new();
        let result = lower_canonical_collect(
            0,
            id,
            CollectLowering {
                source: &input.source,
                pages: input.pages,
                items: input.items,
                body: &input.body,
                next: None,
            },
            &mut builder,
        );

        if result.is_ok() {
            let nodes = &builder.nodes;

            // CollectStart must reference the source
            match &nodes[0].kind {
                CompiledNodeKind::CollectStart { source, .. } => {
                    // source is a SlotIdx, it should be recorded
                    assert!(source.as_usize() <= u16::MAX as usize,
                        "source slot within valid range");
                }
                _ => prop_assert!(false, "node[0] must be CollectStart"),
            }

            // CollectPage must reference the collector_slot (same as source)
            match &nodes[2].kind {
                CompiledNodeKind::CollectPage { collector_slot, .. } => {
                    // collector_slot should match the source slot
                    if let CompiledNodeKind::CollectStart { source, .. } = &nodes[0].kind {
                        prop_assert_eq!(*collector_slot, *source,
                            "CollectPage.collector_slot must match CollectStart.source");
                    }
                }
                _ => prop_assert!(false, "node[2] must be CollectPage"),
            }

            // CollectFinish must reference the collector_slot
            match &nodes[3].kind {
                CompiledNodeKind::CollectFinish { collector_slot } => {
                    if let CompiledNodeKind::CollectStart { source, .. } = &nodes[0].kind {
                        prop_assert_eq!(*collector_slot, *source,
                            "CollectFinish.collector_slot must match CollectStart.source");
                    }
                }
                _ => prop_assert!(false, "node[3] must be CollectFinish"),
            }
        }
    }

    /// I4+I5: CollectStart.limit and page_size match defaults or explicit values.
    #[test]
    fn collect_start_limit_and_page_size_correct(
        id in safe_step_idx(),
        input in collect_input_strategy(),
    ) {
        let mut builder = SlotCompiler::new();
        let result = lower_canonical_collect(
            0,
            id,
            CollectLowering {
                source: &input.source,
                pages: input.pages,
                items: input.items,
                body: &input.body,
                next: None,
            },
            &mut builder,
        );

        if result.is_ok() {
            match &builder.nodes[0].kind {
                CompiledNodeKind::CollectStart { limit, page_size, .. } => {
                    prop_assert_eq!(*limit, input.pages.unwrap_or(1),
                        "CollectStart.limit must equal pages.unwrap_or(1)");
                    prop_assert_eq!(*page_size, input.items.unwrap_or(1),
                        "CollectStart.page_size must equal items.unwrap_or(1)");
                }
                _ => prop_assert!(false, "node[0] must be CollectStart"),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Deterministic unit tests
// ─────────────────────────────────────────────────────────────────

#[test]
fn default_pages_and_items_are_1() {
    let mut builder = SlotCompiler::new();
    let body = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "42".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let result = lower_canonical_collect(
        0,
        StepIdx::new(10),
        CollectLowering {
            source: "0",
            pages: None,
            items: None,
            body: &body,
            next: None,
        },
        &mut builder,
    );
    assert!(result.is_ok(), "default collect should compile: {result:?}");

    let nodes = &builder.nodes;
    assert_eq!(nodes.len(), 4);

    match &nodes[0].kind {
        CompiledNodeKind::CollectStart { limit, page_size, .. } => {
            assert_eq!(*limit, 1, "default pages (limit) should be 1");
            assert_eq!(*page_size, 1, "default items (page_size) should be 1");
        }
        other => assert!(false, "expected CollectStart, got {other:?}"),
    }
}

#[test]
fn explicit_pages_and_items_are_preserved() {
    let mut builder = SlotCompiler::new();
    let body = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let result = lower_canonical_collect(
        0,
        StepIdx::new(10),
        CollectLowering {
            source: "0",
            pages: Some(5),
            items: Some(10),
            body: &body,
            next: None,
        },
        &mut builder,
    );
    assert!(result.is_ok(), "explicit collect should compile: {result:?}");

    let nodes = &builder.nodes;
    assert_eq!(nodes.len(), 4);

    match &nodes[0].kind {
        CompiledNodeKind::CollectStart { limit, page_size, .. } => {
            assert_eq!(*limit, 5, "explicit pages should be 5");
            assert_eq!(*page_size, 10, "explicit items should be 10");
        }
        other => assert!(false, "expected CollectStart, got {other:?}"),
    }
}

#[test]
fn collect_finish_preserves_next() {
    let mut builder = SlotCompiler::new();
    let body = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let next_step = StepIdx::new(99);
    let result = lower_canonical_collect(
        0,
        StepIdx::new(10),
        CollectLowering {
            source: "0",
            pages: None,
            items: None,
            body: &body,
            next: Some(next_step),
        },
        &mut builder,
    );
    assert!(result.is_ok(), "collect with next should compile: {result:?}");

    let nodes = &builder.nodes;
    assert_eq!(nodes.len(), 4);

    match &nodes[3].kind {
        CompiledNodeKind::CollectFinish { .. } => {
            assert_eq!(nodes[3].next, Some(next_step),
                "CollectFinish.next must preserve the given next step");
        }
        other => assert!(false, "expected CollectFinish, got {other:?}"),
    }
}

#[test]
fn node_ids_are_consecutive_from_step_idx() {
    let mut builder = SlotCompiler::new();
    let body = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "5".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let start = StepIdx::new(42);
    let result = lower_canonical_collect(
        0,
        start,
        CollectLowering {
            source: "0",
            pages: None,
            items: None,
            body: &body,
            next: None,
        },
        &mut builder,
    );
    assert!(result.is_ok(), "collect compile: {result:?}");

    let nodes = &builder.nodes;
    assert_eq!(nodes[0].id.get(), 42, "node 0 at start");
    assert_eq!(nodes[1].id.get(), 43, "node 1 at start+1");
    assert_eq!(nodes[2].id.get(), 44, "node 2 at start+2");
    assert_eq!(nodes[3].id.get(), 45, "node 3 at start+3");
}
