#![forbid(unsafe_code)]

//! Deterministic unit tests for the lexical N-step bounded collect
//! lowering. Invariants are stated in the parent module header; the
//! shared scope plumbing lives there.

use crate::{StepAst, StepPrimitive};
use vb_core::ids::StepIdx;
use vb_core::workflow::CompiledNodeKind;
use vb_core::workflow::NodeContinuation;

use super::super::SlotCompiler;
use super::super::part_02::bounded_collect::lower_bounded_collect as lower_canonical_collect;
use super::{ScopeMaps, scoped_collect};

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
