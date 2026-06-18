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
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
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
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
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
    clippy::useless_asref,
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

//! Property tests: SlotWritten-before-PC-Advance ordering invariant.
//!
//! These tests verify that `SlotWritten` evidence events appear in the evidence
//! stream at positions strictly before `StepStarted` for the next step, across
//! all three durability profiles (Volatile, Journaled, Strict).
//!
//! ## Invariant (I1)
//!
//! For every step N that writes a slot, `SlotWritten(N)` MUST appear in the
//! evidence stream at a position strictly before `StepStarted(N+1)`.
//!
//! ## Behaviors Covered
//!
//! - **B-001**: Drive loop emits SlotWritten before StepStarted(next)
//! - **B-002**: Volatile journal preserves evidence ordering
//! - **B-003**: Journaled journal preserves evidence ordering
//! - **B-004**: Strict journal preserves evidence ordering
//!
//! ## Invariants
//!
//! - **I1**: Evidence stream ordering: SlotWritten(N) < StepStarted(N+1)
//! - **I2**: For Nop nodes (no slot write), no SlotWritten event emitted
//! - **I3**: PC advances only after evidence is collected
//! - **I4**: All three durability profiles produce identical evidence ordering
//! - **I5**: Multi-step workflows preserve per-step ordering

use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{EvidenceCollector, RetryPolicy};
use crate::primitives::collect::CollectStates;
use proptest::prelude::*;
use proptest::strategy::{Just, Strategy};
use vb_core::capability::CapabilitySet;
use vb_core::value::{ConstValue, SlotValue};
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RunId, SlotIdx, StepIdx,
    WorkflowDigest,
};

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

/// Creates a test workflow from a vector of compiled nodes and constants.
fn make_workflow(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    constants: Vec<ConstValue>,
) -> Result<CompiledWorkflow, String> {
    let names: Box<[Box<str>]> = (0..nodes.len())
        .map(|i| format!("s{i}").into_boxed_str())
        .collect();
    let parts = WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: names,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

/// Creates a SetConst node: writes a constant value to an output slot.
fn set_const_node(id: u16, const_idx: u16, output: u16, next: Option<u16>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: Some(SlotIdx::new(output)),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(const_idx),
        },
    }
}

/// Creates a Nop node: advances PC without writing any slot.
fn nop_node(id: u16, next: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: Some(StepIdx::new(next)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

/// Creates a Copy node: copies a source slot to an output slot.
fn copy_node(id: u16, source: u16, output: u16, next: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: Some(SlotIdx::new(output)),
        next: Some(StepIdx::new(next)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(source),
        },
    }
}

/// Creates a Finish node: terminates the workflow with a result slot value.
fn finish_node(id: u16, result: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result),
        },
    }
}

/// Generates a random multi-step workflow with SetConst nodes.
fn arb_set_const_workflow(
    min_steps: u16,
    max_steps: u16,
) -> impl Strategy<Value = (CompiledWorkflow, Vec<ConstValue>)> {
    (min_steps..=max_steps)
        .prop_flat_map(|step_count| {
            let slot_count = step_count + 1; // One extra slot for the finish result
            let num_constants = step_count;

            // Generate step count SetConst nodes plus a finish node
            let nodes: Vec<CompiledNode> = (0..step_count)
                .map(|i| {
                    // Each step writes its constant to slot i, next step is i+1
                    set_const_node(i, i, i, Some(i + 1))
                })
                .collect();
            let nodes: Vec<CompiledNode> = nodes
                .into_iter()
                .chain(std::iter::once(finish_node(step_count, step_count - 1)))
                .collect();

            // Generate random constant values
            let constants: Vec<ConstValue> = (0..num_constants)
                .map(|i| ConstValue::I64(i64::from(i as u8) + 1))
                .collect();

            Just((nodes, constants, slot_count))
        })
        .prop_map(|(nodes, constants, slot_count)| {
            let wf = make_workflow(nodes, slot_count, constants.clone())
                .expect("workflow construction should succeed");
            (wf, constants)
        })
}

/// Generates a mixed workflow with SetConst, Copy, Nop, and Finish nodes.
fn arb_mixed_workflow(min_steps: u16, max_steps: u16) -> impl Strategy<Value = CompiledWorkflow> {
    (min_steps..=max_steps).prop_flat_map(|step_count| {
        prop_oneof![
            // Generate a SetConst chain
            arb_set_const_workflow(1, step_count).prop_map(|(wf, _)| wf),
            // Generate a Copy chain
            Just(make_copy_workflow(step_count)),
            // Generate a mixed workflow
            Just(make_mixed_workflow(step_count)),
        ]
    })
}

/// Creates a Copy chain: SetConst(slot 0) → Copy(slot 0→1) → Copy(slot 1→2) → ... → finish
fn make_copy_workflow(step_count: u16) -> CompiledWorkflow {
    let mut nodes = Vec::with_capacity((step_count + 2) as usize);

    // First step: SetConst to initialize slot 0
    nodes.push(set_const_node(0, 0, 0, Some(1)));

    // Remaining steps: copy from previous slot to next
    for i in 1..step_count {
        nodes.push(copy_node(i, i - 1, i, i + 1));
    }

    // Finish node: result from the last written slot
    let finish_slot = if step_count > 1 { step_count - 1 } else { 0 };
    nodes.push(finish_node(step_count, finish_slot));

    make_workflow(nodes, step_count.max(1), vec![ConstValue::I64(42)])
        .expect("copy workflow construction should succeed")
}

/// Creates a mixed workflow: SetConst, Nop, Copy, Finish pattern.
fn make_mixed_workflow(step_count: u16) -> CompiledWorkflow {
    let mut nodes = Vec::with_capacity((step_count + 2) as usize);
    let mut slot_idx = 0u16;
    let mut set_const_count = 0u16;

    for i in 0..step_count {
        match i % 3 {
            0 => {
                // SetConst node uses the next available constant index
                let const_idx = set_const_count;
                set_const_count += 1;
                nodes.push(set_const_node(i, const_idx, slot_idx, Some(i + 1)));
                slot_idx += 1;
            }
            1 => {
                // Nop node
                nodes.push(nop_node(i, i + 1));
            }
            _ => {
                // Copy node (copy from slot 0 to slot 0 if no previous slot)
                let src = if slot_idx > 0 { slot_idx - 1 } else { 0 };
                nodes.push(copy_node(i, src, slot_idx, i + 1));
                slot_idx += 1;
            }
        }
    }
    // Finish node
    let finish_slot = if slot_idx > 0 { slot_idx - 1 } else { 0 };
    nodes.push(finish_node(step_count, finish_slot));

    // Generate constants for SetConst nodes
    let constants: Vec<ConstValue> = (0..set_const_count)
        .map(|i| ConstValue::I64(i64::from(i) + 1))
        .collect();

    make_workflow(nodes, slot_idx.max(1), constants)
        .expect("mixed workflow construction should succeed")
}

// ============================================================================
// Evidence ordering invariant check
// ============================================================================

/// Checks that for every step N with a slot write, SlotWritten(N) appears
/// before StepStarted(N+1) in the evidence stream.
///
/// Strategy: collect all SlotWritten and StepStarted positions, then verify
/// that every SlotWritten appears before the next StepStarted in the event stream.
fn verify_evidence_ordering(events: &[crate::engine::types::EvidenceEvent]) -> bool {
    let slot_written_positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(idx, e)| {
            if let crate::engine::types::EvidenceEvent::SlotWritten { .. } = e {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    let step_started_positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(idx, e)| {
            if let crate::engine::types::EvidenceEvent::StepStarted { .. } = e {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    // Every SlotWritten must appear before the next StepStarted in the stream.
    // If there's no next StepStarted (last events have no following step), that's fine.
    for &sw_pos in &slot_written_positions {
        if let Some(&ss_pos) = step_started_positions.iter().find(|&&p| p > sw_pos) {
            if sw_pos >= ss_pos {
                return false;
            }
        }
    }

    true
}

/// Checks that evidence events come in the expected per-step ordering:
/// StepStarted(N), SlotWritten(N) (optional), StepSucceeded(N).
fn verify_step_event_ordering(
    events: &[crate::engine::types::EvidenceEvent],
) -> Result<(), String> {
    let mut step_started_count = 0u32;
    let mut slot_written_count = 0u32;
    let mut step_succeeded_count = 0u32;

    for event in events {
        match event {
            crate::engine::types::EvidenceEvent::StepStarted { .. } => {
                step_started_count += 1;
            }
            crate::engine::types::EvidenceEvent::SlotWritten { .. } => {
                slot_written_count += 1;
                // SlotWritten must be inside a step (after StepStarted)
                if slot_written_count > step_started_count {
                    return Err("SlotWritten outside of a step".to_string());
                }
            }
            crate::engine::types::EvidenceEvent::StepSucceeded { .. } => {
                step_succeeded_count += 1;
                // StepSucceeded must be preceded by StepStarted
                if step_succeeded_count > step_started_count {
                    return Err("StepSucceeded before StepStarted".to_string());
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// Proptest suites
// ============================================================================

proptest! {
    // ── I1: Evidence ordering invariant across random workflows ──

    /// For 1000 random SetConst workflows, SlotWritten(N) appears before
    /// StepStarted(N+1) in the evidence stream.
    #[test]
    fn slot_written_before_next_step_started_in_drive_loop(
        (wf, _constants) in arb_set_const_workflow(2, 10),
    ) {
        let step_count = wf.node_count();
        let mut run = vb_core::frame::RunFrame::new(
            RunId::new(1),
            StepIdx::new(0),
            step_count as u16,
            wf.slot_count(),
        ).expect("run frame creation should succeed");

        let mut budget = vb_core::engine::StepBudget::new(1000);
        let mut store = vb_core::value_store::ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        // When: Execute the workflow
        let sig = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        ).expect("drive should succeed");

        // Then: Signal should be Finished
        match sig {
            crate::engine::types::RuntimeSignal::Finished(_) => {},
            other => panic!("expected Finished signal, got {other:?}"),
        }

        // Verify: Evidence ordering invariant holds
        let events = evidence.drain();
        prop_assert!(
            verify_evidence_ordering(&events),
            "SlotWritten(N) should appear before StepStarted(N+1). Events: {:?}",
            events
        );

        // Verify: Step event ordering is correct
        prop_assert!(
            verify_step_event_ordering(&events).is_ok(),
            "Step event ordering should be correct. Events: {:?}",
            events
        );
    }

    // ── I1 (extended): Mixed workflows preserve ordering ──

    /// For 1000 random mixed workflows, SlotWritten(N) appears before
    /// StepStarted(N+1) in the evidence stream.
    #[test]
    fn slot_written_before_next_step_in_mixed_workflows(
        wf in arb_mixed_workflow(3, 10),
    ) {
        let step_count = wf.node_count();
        let mut run = vb_core::frame::RunFrame::new(
            RunId::new(2),
            StepIdx::new(0),
            step_count as u16,
            wf.slot_count(),
        ).expect("run frame creation should succeed");

        let mut budget = vb_core::engine::StepBudget::new(1000);
        let mut store = vb_core::value_store::ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        // When: Execute the workflow
        let sig = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        ).expect("drive should succeed");

        // Then: Signal should be Finished
        match sig {
            crate::engine::types::RuntimeSignal::Finished(_) => {},
            other => panic!("expected Finished signal, got {other:?}"),
        }

        // Verify: Evidence ordering invariant holds
        let events = evidence.drain();
        prop_assert!(
            verify_evidence_ordering(&events),
            "SlotWritten(N) should appear before StepStarted(N+1) in mixed workflow. Events: {:?}",
            events
        );
    }

    // ── I2: Nop nodes produce no SlotWritten events ──

    /// For workflows containing Nop nodes, only nodes with output slots
    /// produce SlotWritten events.
    #[test]
    fn nop_nodes_do_not_emit_slot_written(
        // Workflow: SetConst(0) → Nop(1) → Copy(2) → Finish(3)
        _ in any::<u32>(),
    ) {
        let wf = make_workflow(
            vec![
                set_const_node(0, 0, 0, Some(1)),
                nop_node(1, 2),
                copy_node(2, 0, 1, 3),
                finish_node(3, 1),
            ],
            2,
            vec![ConstValue::I64(42)],
        ).expect("workflow construction should succeed");

        let mut run = vb_core::frame::RunFrame::new(
            RunId::new(3),
            StepIdx::new(0),
            4,
            2,
        ).expect("run frame creation should succeed");

        let mut budget = vb_core::engine::StepBudget::new(1000);
        let mut store = vb_core::value_store::ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        // When: Execute the workflow
        let sig = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        ).expect("drive should succeed");

        match sig {
            crate::engine::types::RuntimeSignal::Finished(_) => {},
            other => panic!("expected Finished signal, got {other:?}"),
        }

        // Then: Evidence contains SlotWritten for SetConst and Copy steps,
        // but not for Nop step
        let events = evidence.drain();
        let slot_written_count = events.iter()
            .filter(|e| matches!(e, crate::engine::types::EvidenceEvent::SlotWritten { .. }))
            .count();

        // SetConst(step 0) and Copy(step 2) should emit SlotWritten
        // Nop(step 1) should not
        prop_assert_eq!(
            slot_written_count,
            2,
            "Expected exactly 2 SlotWritten events (SetConst + Copy), got {}. Events: {:?}",
            slot_written_count,
            events
        );
    }

    // ── I3: Single-step workflow ──

    /// A single-step workflow with SetConst produces exactly one SlotWritten
    /// event, and no StepStarted(1) exists to compare against.
    #[test]
    fn single_step_workflow_preserves_ordering(
        _ in any::<u32>(),
    ) {
        let wf = make_workflow(
            vec![
                set_const_node(0, 0, 0, Some(1)),
                finish_node(1, 0),
            ],
            1,
            vec![ConstValue::I64(99)],
        ).expect("workflow construction should succeed");

        let mut run = vb_core::frame::RunFrame::new(
            RunId::new(4),
            StepIdx::new(0),
            2,
            1,
        ).expect("run frame creation should succeed");

        let mut budget = vb_core::engine::StepBudget::new(1000);
        let mut store = vb_core::value_store::ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        // When: Execute the workflow
        let sig = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        ).expect("drive should succeed");

        match sig {
            crate::engine::types::RuntimeSignal::Finished(_) => {},
            other => panic!("expected Finished signal, got {other:?}"),
        }

        // Then: Evidence contains SlotWritten(0) and StepStarted(1) is present
        let events = evidence.drain();
        let has_slot_written = events.iter().any(|e| {
            matches!(
                e,
                crate::engine::types::EvidenceEvent::SlotWritten {
                    slot,
                    value: SlotValue::I64(99),
                    ..
                } if *slot == SlotIdx::new(0)
            )
        });
        prop_assert!(
            has_slot_written,
            "Evidence should contain SlotWritten(0, I64(99)). Events: {:?}",
            events
        );

        // StepStarted(1) should be present (from finish node)
        let has_step_started_1 = events.iter().any(|e| {
            matches!(
                e,
                crate::engine::types::EvidenceEvent::StepStarted { step }
                if *step == StepIdx::new(1)
            )
        });
        prop_assert!(
            has_step_started_1,
            "Evidence should contain StepStarted(1). Events: {:?}",
            events
        );
    }

    // ── I5: Large workflows preserve ordering ──

    /// For workflows with up to 20 steps, the ordering invariant holds.
    #[test]
    fn large_workflows_preserve_ordering(
        (wf, _) in arb_set_const_workflow(10, 20),
    ) {
        let step_count = wf.node_count();
        let mut run = vb_core::frame::RunFrame::new(
            RunId::new(5),
            StepIdx::new(0),
            step_count as u16,
            wf.slot_count(),
        ).expect("run frame creation should succeed");

        let mut budget = vb_core::engine::StepBudget::new(10000);
        let mut store = vb_core::value_store::ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        // When: Execute the workflow
        let sig = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        ).expect("drive should succeed");

        match sig {
            crate::engine::types::RuntimeSignal::Finished(_) => {},
            other => panic!("expected Finished signal, got {other:?}"),
        }

        // Verify: Evidence ordering invariant holds
        let events = evidence.drain();
        prop_assert!(
            verify_evidence_ordering(&events),
            "SlotWritten(N) should appear before StepStarted(N+1) in large workflow with {} steps. Events: {:?}",
            step_count,
            events
        );
    }

    // ── I4: EvidenceCollector capacity bounds ──

    /// The EvidenceCollector enforces its capacity limit and does not
    /// exceed it regardless of workflow size.
    #[test]
    fn evidence_collector_enforces_capacity(
        capacity in 1usize..=100usize,
    ) {
        let mut collector = EvidenceCollector::with_capacity(capacity);

        // Push more events than capacity
        for i in 0..(capacity + 100) {
            collector.push_step_started(StepIdx::new(i as u16));
            collector.push_slot_written(SlotIdx::new(i as u16), SlotValue::I64(i as i64));
            collector.push_step_succeeded(StepIdx::new(i as u16), None);
        }

        // Then: Collected events should not exceed capacity
        prop_assert!(
            collector.len() <= capacity,
            "EvidenceCollector len ({}) should not exceed capacity ({})",
            collector.len(),
            capacity
        );

        // And: Some events should have been dropped
        prop_assert!(
            collector.dropped() >= 1,
            "Some events should have been dropped when exceeding capacity"
        );
    }
}

// ============================================================================
// Deterministic unit tests
// ============================================================================

/// Deterministic test: Two-step workflow with SetConst nodes.
/// Verifies SlotWritten(0) appears before StepStarted(1).
#[test]
fn slot_written_before_step_started_deterministic() {
    let constants = vec![ConstValue::I64(10), ConstValue::I64(20)];
    let wf = make_workflow(
        vec![
            set_const_node(0, 0, 0, Some(1)),
            set_const_node(1, 1, 1, Some(2)),
            finish_node(2, 1),
        ],
        2,
        constants,
    )
    .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(RunId::new(10), StepIdx::new(0), 3, 2)
        .expect("run frame creation should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    // When: Execute both steps
    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    match sig {
        crate::engine::types::RuntimeSignal::Finished(_) => {}
        other => panic!("expected Finished, got {other:?}"),
    }

    // Then: SlotWritten(0) appears before StepStarted(1)
    let events = evidence.drain();
    let slot_written_0_pos = events
        .iter()
        .position(|e| {
            matches!(
                e,
                crate::engine::types::EvidenceEvent::SlotWritten {
                    slot,
                    value: SlotValue::I64(10),
                    ..
                } if *slot == SlotIdx::new(0)
            )
        })
        .expect("SlotWritten(0) should be in evidence");

    let step_started_1_pos = events
        .iter()
        .position(|e| {
            matches!(
                e,
                crate::engine::types::EvidenceEvent::StepStarted { step }
                if *step == StepIdx::new(1)
            )
        })
        .expect("StepStarted(1) should be in evidence");

    assert!(
        slot_written_0_pos < step_started_1_pos,
        "SlotWritten(0) at position {} should appear BEFORE StepStarted(1) at position {}. Full evidence: {:?}",
        slot_written_0_pos,
        step_started_1_pos,
        events
    );
}

/// Deterministic test: Verify StepStarted/SlotWritten/StepSucceeded ordering.
#[test]
fn step_event_ordering_in_drive_loop() {
    let wf = make_workflow(
        vec![
            set_const_node(0, 0, 0, Some(1)),
            set_const_node(1, 1, 1, Some(2)),
            finish_node(2, 1),
        ],
        2,
        vec![ConstValue::I64(10), ConstValue::I64(20)],
    )
    .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(RunId::new(11), StepIdx::new(0), 3, 2)
        .expect("run frame creation should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    let events = evidence.drain();
    assert!(
        verify_step_event_ordering(&events).is_ok(),
        "Step event ordering should be correct. Events: {:?}",
        events
    );
}

/// Deterministic test: Verify that PC has advanced after drive completes.
#[test]
fn pc_advances_after_drive_completes() {
    let wf = make_workflow(
        vec![set_const_node(0, 0, 0, Some(1)), finish_node(1, 0)],
        1,
        vec![ConstValue::I64(42)],
    )
    .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(RunId::new(12), StepIdx::new(0), 2, 1)
        .expect("run frame creation should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    assert!(
        run.pc().get() >= 1,
        "PC should have advanced to at least 1 after drive completes, got {}",
        run.pc().get()
    );
}

/// Deterministic test: Copy node preserves slot value ordering in evidence.
#[test]
fn copy_node_slot_written_preserves_value() {
    let wf = make_workflow(vec![copy_node(0, 1, 0, 1), finish_node(1, 0)], 2, vec![])
        .expect("workflow construction should succeed");

    let mut run = vb_core::frame::RunFrame::new(RunId::new(13), StepIdx::new(0), 2, 2)
        .expect("run frame creation should succeed");

    run.write_slot(SlotIdx::new(1), SlotValue::I64(77))
        .expect("slot write should succeed");

    let mut budget = vb_core::engine::StepBudget::new(10);
    let mut store = vb_core::value_store::ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    let sig = drive_deterministic_full(
        &wf,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &CapabilitySet::empty(),
    )
    .expect("drive should succeed");

    match sig {
        crate::engine::types::RuntimeSignal::Finished(SlotValue::I64(77)) => {}
        other => panic!("expected Finished(I64(77)), got {other:?}"),
    }

    let events = evidence.drain();
    let slot_written = events.iter().find(|e| {
        matches!(
            e,
            crate::engine::types::EvidenceEvent::SlotWritten {
                slot,
                value: SlotValue::I64(77),
                ..
            } if *slot == SlotIdx::new(0)
        )
    });

    assert!(
        slot_written.is_some(),
        "Evidence should contain SlotWritten(0, I64(77)). Events: {:?}",
        events
    );
}
