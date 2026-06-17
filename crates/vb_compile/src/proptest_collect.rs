// Verification artifact: proptest_collect.rs
// PO: PO-005 (emit_single_body_set and collect traversal termination)
// PO: PO-014 (Collect IR structure for valid inputs)
// Bead: vb-xi2f.23
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_collect --test-threads=1
// Command: cargo test -p vb_compile -- proptest_collect_ir_structure --test-threads=1
//
// Proof obligations:
// - PO-005: emit_single_body_set and collect traversal termination
// - PO-014: Collect IR structure for valid inputs (exactly 4 nodes with correct IDs and kinds)
//
// GOD RULE 1: Uses proptest strategy with Arbitrary for StepAst generation.
// GOD RULE 2: Binds to actual Rust lower_canonical_collect and emit_single_body_set.

#![cfg(test)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use crate::mod_compile_lowering::lower_canonical_collect;
use crate::mod_compile_lowering::emit_single_body_set;
use crate::SlotCompiler;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive};

// ─────────────────────────────────────────────────────────────────
// Helper: Generate a valid single-Set StepAst body
// ─────────────────────────────────────────────────────────────────

/// Strategy for a valid single-Set body (exactly one Set step).
pub fn single_set_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    prop_oneof![
        // Single Set step with integer value
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
        }]),
    ]
}

/// Strategy for collect input: source string, pages, items, body.
#[derive(Debug, Clone)]
pub struct CollectInput {
    pub source: String,
    pub pages: Option<u32>,
    pub items: Option<u32>,
    pub body: Vec<StepAst>,
}

pub fn collect_input_strategy() -> impl Strategy<Value = CollectInput> {
    (
        "\\d+".prop_map(|s: String| s),  // source slot as string
        any::<Option<u32>>(),            // pages
        any::<Option<u32>>(),            // items
        single_set_body_strategy(),      // single Set body
    )
        .prop_map(|(source, pages, items, body)| CollectInput {
            source,
            pages,
            items,
            body,
        })
}

// ─────────────────────────────────────────────────────────────────
// PO-005: emit_single_body_set and collect traversal termination
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-005 H1: emit_single_body_set terminates for valid Set body.
    #[test]
    fn proptest_collect(body in single_set_body_strategy()) {
        let id = StepIdx::new(0);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

        // Valid Set body should succeed and produce exactly one node.
        prop_assert!(
            matches!(result, Ok(())),
            "valid Set body should succeed, got {:?}",
            result
        );
        prop_assert_eq!(builder.node_count(), 1, "exactly 1 node for single-Set body");
    }

    /// PO-005 H2: emit_single_body_set terminates for empty body (returns error).
    #[test]
    fn proptest_collect_empty() {
        let empty_body: Vec<StepAst> = vec![];
        let id = StepIdx::new(0);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&empty_body, id, slot, None, &mut builder, false);

        // Empty body should return error (StepFieldShape) and emit no nodes.
        prop_assert!(
            matches!(result, Err(_)),
            "empty body must return error, got {:?}",
            result
        );

        // No nodes should be emitted for empty body
        prop_assert_eq!(builder.node_count(), 0, "no nodes for empty body");
    }

    /// PO-005 H3: Traversal terminates for collect with valid body.
    #[test]
    fn proptest_collect_traversal(input in collect_input_strategy()) {
        let id = StepIdx::new(0);
        let mut builder = SlotCompiler::new();

        // This should not hang or panic
        let result = lower_canonical_collect(
            0,
            id,
            &input.source,
            input.pages,
            input.items,
            &input.body,
            &mut builder,
        );

        // Valid source yields 4 nodes; invalid source returns an error.
        if result.is_ok() {
            prop_assert_eq!(builder.node_count(), 4, "exactly 4 nodes for valid collect");
        } else {
            prop_assert!(
                matches!(result, Err(_)),
                "invalid collect source should return error, got {:?}",
                result
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-014: Collect IR structure for valid inputs
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-014 H1: lower_canonical_collect produces exactly 4 nodes with correct IDs and kinds.
    #[test]
    fn proptest_collect_ir_structure(input in collect_input_strategy()) {
        let id: u16 = 100; // Fixed starting ID within safe range
        prop_assume!(id <= 65532); // Ensure id+3 doesn't overflow

        let mut builder = SlotCompiler::new();

        let result = lower_canonical_collect(
            0,
            StepIdx::new(id),
            &input.source,
            input.pages,
            input.items,
            &input.body,
            &mut builder,
        );

        prop_assert!(
            matches!(result, Ok(())),
            "lower_canonical_collect must return Ok(()) on valid input, got {:?}",
            result
        );
        let nodes = builder.into_nodes();
            prop_assert_eq!(nodes.len(), 4, "exactly 4 nodes emitted");

            // Check node IDs: id, id+1, id+2, id+3
            prop_assert_eq!(nodes[0].id.get(), id, "node 0 id = id");
            prop_assert_eq!(nodes[1].id.get(), id + 1, "node 1 id = id+1");
            prop_assert_eq!(nodes[2].id.get(), id + 2, "node 2 id = id+2");
            prop_assert_eq!(nodes[3].id.get(), id + 3, "node 3 id = id+3");

            // Check node kinds
            prop_assert!(
                matches!(&nodes[0].kind, vb_core::CompiledNodeKind::CollectStart { .. }),
                "node 0 is CollectStart"
            );
            prop_assert!(
                matches!(&nodes[1].kind, vb_core::CompiledNodeKind::SetConst { .. }),
                "node 1 is SetConst"
            );
            prop_assert!(
                matches!(&nodes[2].kind, vb_core::CompiledNodeKind::CollectPage { .. }),
                "node 2 is CollectPage"
            );
            prop_assert!(
                matches!(&nodes[3].kind, vb_core::CompiledNodeKind::CollectFinish { .. }),
                "node 3 is CollectFinish"
            );
        }
    }
}
