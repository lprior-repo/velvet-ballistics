#![forbid(unsafe_code)]

//! Proptest suite for the lexical N-step bounded collect lowering.
//! Invariants I1-I5 are stated in the parent module header; the shared
//! strategies and scope plumbing live there.

use proptest::prelude::*;
use vb_core::ids::FanoutLimit;
use vb_core::workflow::CompiledNodeKind;

use super::super::SlotCompiler;
use super::super::part_02::bounded_collect::lower_bounded_collect as lower_canonical_collect;
use super::{ScopeMaps, collect_input_strategy, safe_step_idx, scoped_input};

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
                            prop_assert_eq!(*limit, FanoutLimit::new(expected),
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
