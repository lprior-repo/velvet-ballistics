#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // =======================================================================
    // Helpers
    // =======================================================================

    fn make_nop_node(id: u16, next: Option<u16>) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: next.map(StepIdx::new),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    fn make_finish_node(id: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(result_slot),
            },
        }
    }

    fn make_do_node(id: u16, action: u16, input_slot: u16, next: Option<u16>) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: next.map(StepIdx::new),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(action),
                input: SlotIdx::new(input_slot),
            },
        }
    }

    fn make_choose_node(id: u16, targets: &[u16], otherwise: Option<u16>) -> CompiledNode {
        use vb_core::workflow::ExprBranch;
        let branches: Vec<ExprBranch> = targets
            .iter()
            .enumerate()
            .map(|(i, &t)| ExprBranch {
                condition: ExprIdx::new(u16::try_from(i.saturating_add(100)).unwrap_or(u16::MAX)),
                target: StepIdx::new(t),
            })
            .collect();
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: branches.into_boxed_slice(),
                otherwise: otherwise.map(StepIdx::new),
            },
        }
    }

    fn make_error_handler_node(id: u16, body: u16, handler: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(body),
                handler: StepIdx::new(handler),
                error_slot: None,
            },
        }
    }

    fn make_simple_parts(nodes: Vec<CompiledNode>, entry: u16) -> WorkflowParts {
        let node_count = nodes.len();
        let step_names: Vec<Box<str>> = (0..node_count)
            .map(|i| format!("step-{i}").into_boxed_str())
            .collect();

        // Scan nodes for max ConstIdx and ExprIdx referenced so we provide
        // enough dummy entries for validation.
        let mut max_const: usize = 0;
        let mut max_expr: usize = 0;
        for node in &nodes {
            match &node.kind {
                CompiledNodeKind::SetConst { value } => {
                    max_const = max_const.max(value.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ReduceStart { initial, .. } => {
                    max_const = max_const.max(initial.as_usize().saturating_add(1));
                }
                CompiledNodeKind::EvalExpr { expr } => {
                    max_expr = max_expr.max(expr.as_usize().saturating_add(1));
                }
                CompiledNodeKind::Choose { branches, .. } => {
                    for branch in branches.iter() {
                        max_expr = max_expr.max(branch.condition.as_usize().saturating_add(1));
                    }
                }
                _ => {}
            }
        }

        let constants: Vec<vb_core::value::ConstValue> = (0..max_const)
            .map(|_| vb_core::value::ConstValue::Null)
            .collect();
        let expressions: Vec<vb_core::workflow::ExprProgram> = (0..max_expr)
            .map(|_| {
                // Minimal valid expression: push one slot value, leaving depth=1.
                vb_core::workflow::ExprProgram::try_from_ops(Box::new([
                    vb_core::workflow::ExprOp::LoadSlot(SlotIdx::new(0)),
                ]))
                .expect("minimal expression should be valid")
            })
            .collect();

        // Determine slot count from nodes.
        let mut max_slot: usize = 4;
        for node in &nodes {
            if let Some(slot) = node.output {
                max_slot = max_slot.max(slot.as_usize().saturating_add(1));
            }
            if let Some(slot) = node.error_slot {
                max_slot = max_slot.max(slot.as_usize().saturating_add(1));
            }
            match &node.kind {
                CompiledNodeKind::Copy { source } => {
                    max_slot = max_slot.max(source.as_usize().saturating_add(1));
                }
                CompiledNodeKind::Do { input, .. } => {
                    max_slot = max_slot.max(input.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ForEachStart {
                    input, item_slot, ..
                } => {
                    max_slot = max_slot.max(input.as_usize().saturating_add(1));
                    max_slot = max_slot.max(item_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
                    max_slot = max_slot.max(iterator_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ForEachJoin { output } => {
                    max_slot = max_slot.max(output.as_usize().saturating_add(1));
                }
                CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => {
                    max_slot = max_slot.max(attempt_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
                    max_slot = max_slot.max(attempt_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::RepeatFinish { result } => {
                    max_slot = max_slot.max(result.as_usize().saturating_add(1));
                }
                CompiledNodeKind::RetryCheck { policy_slot, .. } => {
                    max_slot = max_slot.max(policy_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::CollectStart { source, .. } => {
                    max_slot = max_slot.max(source.as_usize().saturating_add(1));
                }
                CompiledNodeKind::CollectPage { collector_slot, .. } => {
                    max_slot = max_slot.max(collector_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::CollectNext { collector_slot, .. } => {
                    max_slot = max_slot.max(collector_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::CollectFinish { collector_slot } => {
                    max_slot = max_slot.max(collector_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ReduceStart {
                    input, accumulator, ..
                } => {
                    max_slot = max_slot.max(input.as_usize().saturating_add(1));
                    max_slot = max_slot.max(accumulator.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ReduceNext {
                    iterator_slot,
                    accumulator,
                    ..
                } => {
                    max_slot = max_slot.max(iterator_slot.as_usize().saturating_add(1));
                    max_slot = max_slot.max(accumulator.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ReduceFinish { accumulator } => {
                    max_slot = max_slot.max(accumulator.as_usize().saturating_add(1));
                }
                CompiledNodeKind::WaitUntil { deadline_slot } => {
                    max_slot = max_slot.max(deadline_slot.as_usize().saturating_add(1));
                }
                CompiledNodeKind::WaitEvent {
                    event,
                    timeout_slot,
                } => {
                    max_slot = max_slot.max(event.as_usize().saturating_add(1));
                    if let Some(ts) = timeout_slot {
                        max_slot = max_slot.max(ts.as_usize().saturating_add(1));
                    }
                }
                CompiledNodeKind::Ask {
                    prompt,
                    timeout_slot,
                } => {
                    max_slot = max_slot.max(prompt.as_usize().saturating_add(1));
                    if let Some(ts) = timeout_slot {
                        max_slot = max_slot.max(ts.as_usize().saturating_add(1));
                    }
                }
                CompiledNodeKind::AskResume { answer } => {
                    max_slot = max_slot.max(answer.as_usize().saturating_add(1));
                }
                CompiledNodeKind::Finish { result } => {
                    max_slot = max_slot.max(result.as_usize().saturating_add(1));
                }
                CompiledNodeKind::ChooseSlot { branches, .. } => {
                    for branch in branches.iter() {
                        max_slot = max_slot.max(branch.condition.as_usize().saturating_add(1));
                    }
                }
                CompiledNodeKind::TogetherBranch { accumulator, .. } => {
                    max_slot = max_slot.max(accumulator.as_usize().saturating_add(1));
                }
                CompiledNodeKind::TogetherJoin { accumulator, .. } => {
                    max_slot = max_slot.max(accumulator.as_usize().saturating_add(1));
                }
                _ => {}
            }
        }
        let slot_count = u16::try_from(max_slot).unwrap_or(u16::MAX);

        WorkflowParts {
            name: String::from("test-workflow").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: expressions.into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: constants.into_boxed_slice(),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(entry),
            resource_contract: ResourceContract::DEFAULT,
            step_names: step_names.into_boxed_slice(),
        }
    }

    fn build_graph_from_parts(parts: WorkflowParts) -> WorkflowGraph {
        let workflow = CompiledWorkflow::try_from_parts(parts).expect("test parts should be valid");
        build_graph(&workflow)
    }

    fn make_empty_document() -> FlowDocument {
        let node = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![node], 0);
        crate::graph_builder::build_document(&parts)
    }

    fn make_chain_document() -> FlowDocument {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_nop_node(1, Some(2));
        let n2 = make_finish_node(2, 0);
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        crate::graph_builder::build_document(&parts)
    }

    // =======================================================================
    // Viewport tests (preserved from original canvas.rs)
    // =======================================================================

    #[test]
    fn new_canvas_has_default_viewport_state() {
        let doc = make_empty_document();
        let canvas = WorkflowCanvas::new(doc);
        assert_eq!(canvas.pan(), (0.0, 0.0));
        assert!((canvas.zoom() - 1.0).abs() < f64::EPSILON);
        assert!(canvas.selected().is_none());
        assert_eq!(canvas.node_count(), 1);
    }

    #[test]
    fn set_pan_updates_pan() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_pan(10.0, 20.0);
        assert_eq!(canvas.pan(), (10.0, 20.0));
    }

    #[test]
    fn set_zoom_clamps_to_range() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_zoom(0.01);
        assert!((canvas.zoom() - MIN_ZOOM).abs() < f64::EPSILON);
        canvas.set_zoom(100.0);
        assert!((canvas.zoom() - MAX_ZOOM).abs() < f64::EPSILON);
        canvas.set_zoom(2.0);
        assert!((canvas.zoom() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_selected_updates_selection() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        assert!(canvas.selected().is_none());
        canvas.set_selected(Some(0));
        assert_eq!(canvas.selected(), Some(0));
        canvas.set_selected(None);
        assert!(canvas.selected().is_none());
    }

    #[test]
    fn viewport_rect_computes_world_bounds() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_pan(50.0, 100.0);
        canvas.set_zoom(2.0);
        let vr = canvas.viewport_rect(800.0, 600.0);
        assert!((vr.x - 50.0).abs() < f64::EPSILON);
        assert!((vr.y - 100.0).abs() < f64::EPSILON);
        assert!((vr.width - 400.0).abs() < f64::EPSILON);
        assert!((vr.height - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn viewport_rect_intersects_overlapping() {
        let vr = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(vr.intersects(50.0, 50.0, 100.0, 100.0));
        assert!(vr.intersects(10.0, 10.0, 20.0, 20.0));
        assert!(vr.intersects(0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn viewport_rect_no_intersection_when_disjoint() {
        let vr = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(!vr.intersects(200.0, 0.0, 100.0, 100.0));
        assert!(!vr.intersects(0.0, 200.0, 100.0, 100.0));
        assert!(!vr.intersects(0.0, -200.0, 100.0, 100.0));
        assert!(!vr.intersects(-200.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn visible_nodes_returns_intersecting_nodes() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        let viewport = ViewportRect {
            x: -1000.0,
            y: -1000.0,
            width: 5000.0,
            height: 5000.0,
        };
        let visible = canvas.visible_nodes(&viewport);
        assert_eq!(visible.len(), 3);
    }

    #[test]
    fn visible_nodes_excludes_offscreen_nodes() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        let viewport = ViewportRect {
            x: -10000.0,
            y: -10000.0,
            width: 1.0,
            height: 1.0,
        };
        let visible = canvas.visible_nodes(&viewport);
        assert!(visible.is_empty());
    }

    #[test]
    fn focus_jump_centers_on_node() {
        let doc = make_chain_document();
        let mut canvas = WorkflowCanvas::new(doc);
        let positions = canvas.test_positions();
        let target_pos = positions.get(&1).copied().unwrap_or([0.0; 2]);
        let ok = canvas.focus_jump(1, 800.0, 600.0);
        assert!(ok);
        let inv_zoom = 1.0 / canvas.zoom();
        let expected_x = target_pos[0] - 800.0 * inv_zoom / 2.0;
        let expected_y = target_pos[1] - 600.0 * inv_zoom / 2.0;
        assert!((canvas.pan().0 - expected_x).abs() < 0.01);
        assert!((canvas.pan().1 - expected_y).abs() < 0.01);
    }

    #[test]
    fn focus_jump_returns_false_for_invalid_step() {
        let doc = make_chain_document();
        let mut canvas = WorkflowCanvas::new(doc);
        let ok = canvas.focus_jump(999, 800.0, 600.0);
        assert!(!ok);
    }

    #[test]
    fn compute_edge_paths_produces_paths_for_chain() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        let paths = canvas.compute_edge_paths();
        assert_eq!(paths.len(), 2);
        let first = &paths[0];
        assert_eq!(first.source_step, 0);
        assert_eq!(first.target_step, 1);
        assert!(first.start[0] > 0.0);
        assert!(first.end[0] > first.start[0]);
    }

    #[test]
    fn edge_path_control_points_are_between_start_and_end() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        let paths = canvas.compute_edge_paths();
        for path in &paths {
            assert!(path.cp1[0] >= path.start[0]);
            assert!(path.cp2[0] <= path.end[0]);
            assert!(path.cp1[0] <= path.end[0]);
            assert!(path.cp2[0] >= path.start[0]);
        }
    }

    #[test]
    fn chain_layout_positions_increase_in_x() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        let positions = canvas.test_positions();
        let p0 = positions.get(&0).copied().unwrap_or([0.0; 2]);
        let p1 = positions.get(&1).copied().unwrap_or([0.0; 2]);
        let p2 = positions.get(&2).copied().unwrap_or([0.0; 2]);
        assert!(p0[0] < p1[0]);
        assert!(p1[0] < p2[0]);
    }

    #[test]
    fn edge_count_matches_document() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        assert_eq!(canvas.edge_count(), 2);
    }

    #[test]
    fn node_count_matches_document() {
        let doc = make_chain_document();
        let canvas = WorkflowCanvas::new(doc);
        assert_eq!(canvas.node_count(), 3);
    }

    #[test]
    fn viewport_at_min_zoom_covers_large_area() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_zoom(MIN_ZOOM);
        let vr = canvas.viewport_rect(800.0, 600.0);
        assert!((vr.width - 8000.0).abs() < f64::EPSILON);
        assert!((vr.height - 6000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn focus_jump_with_zoom_accounts_for_inv_zoom() {
        let doc = make_chain_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_zoom(0.5);
        let positions = canvas.test_positions();
        let Some(target_pos) = positions.get(&1).copied() else {
            return;
        };
        let ok = canvas.focus_jump(1, 800.0, 600.0);
        assert!(ok);
        let inv_zoom = 1.0 / canvas.zoom();
        let expected_x = target_pos[0] - 800.0 * inv_zoom / 2.0;
        let expected_y = target_pos[1] - 600.0 * inv_zoom / 2.0;
        assert!((canvas.pan().0 - expected_x).abs() < 0.01);
        assert!((canvas.pan().1 - expected_y).abs() < 0.01);
    }

    #[test]
    fn edge_count_for_single_node_document_is_zero() {
        let doc = make_empty_document();
        let canvas = WorkflowCanvas::new(doc);
        assert_eq!(canvas.edge_count(), 0);
    }

    #[test]
    fn negative_pan_values_are_valid() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        canvas.set_pan(-500.0, -300.0);
        let (px, py) = canvas.pan();
        assert!((px - (-500.0)).abs() < f64::EPSILON);
        assert!((py - (-300.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_document_has_zero_edge_paths() {
        let doc = make_empty_document();
        let canvas = WorkflowCanvas::new(doc);
        assert!(canvas.compute_edge_paths().is_empty());
    }

    #[test]
    fn viewport_intersects_edge_touching_rect() {
        let vr = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(!vr.intersects(100.0, 0.0, 50.0, 50.0));
    }

    #[test]
    fn focus_jump_on_single_node_succeeds() {
        let doc = make_empty_document();
        let mut canvas = WorkflowCanvas::new(doc);
        let ok = canvas.focus_jump(0, 800.0, 600.0);
        assert!(ok);
        let (px, py) = canvas.pan();
        let positions = canvas.test_positions();
        let Some(pos) = positions.get(&0).copied() else {
            return;
        };
        let inv_zoom = 1.0 / canvas.zoom();
        assert!((px - (pos[0] - 400.0 * inv_zoom)).abs() < 0.01);
        assert!((py - (pos[1] - 300.0 * inv_zoom)).abs() < 0.01);
    }

    // =======================================================================
    // Graph data model tests (Phase 4A)
    // =======================================================================

    #[test]
    fn graph_single_finish_node_produces_one_node() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn graph_single_finish_node_has_no_edges() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn graph_chain_of_three_produces_two_edges() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_nop_node(1, Some(2)),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn graph_chain_edges_are_sequential() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_nop_node(1, Some(2)),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        for edge in &graph.edges {
            assert_eq!(edge.edge_type, EdgeType::Sequential);
        }
    }

    #[test]
    fn graph_entry_step_matches_parts() {
        let parts = make_simple_parts(vec![make_nop_node(0, Some(1)), make_finish_node(1, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.entry_step, StepIdx::new(0));
    }

    #[test]
    fn graph_slot_count_matches_parts() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.slot_count, 4);
    }

    #[test]
    fn graph_workflow_name_matches_parts() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.workflow_name, "test-workflow");
    }

    #[test]
    fn graph_do_node_gets_action_id_badge() {
        let parts = make_simple_parts(
            vec![make_do_node(0, 17, 0, Some(1)), make_finish_node(1, 0)],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph.nodes[0]
                .badges
                .iter()
                .any(|b| *b == NodeBadge::ActionId(17))
        );
    }

    #[test]
    fn graph_do_node_kind_name_is_do() {
        let parts = make_simple_parts(
            vec![make_do_node(0, 1, 0, Some(1)), make_finish_node(1, 0)],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes[0].kind_name, "Do");
    }

    #[test]
    fn graph_finish_node_kind_name_is_finish() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes[0].kind_name, "Finish");
    }

    #[test]
    fn graph_node_position_is_none_before_layout() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert!(graph.nodes[0].position.is_none());
    }

    #[test]
    fn graph_node_step_idx_matches_compiled_id() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_nop_node(1, Some(2)),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes[0].step_idx, StepIdx::new(0));
        assert_eq!(graph.nodes[1].step_idx, StepIdx::new(1));
        assert_eq!(graph.nodes[2].step_idx, StepIdx::new(2));
    }

    #[test]
    fn graph_choose_node_produces_branch_edges() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1, 2], Some(3)),
                make_finish_node(1, 0),
                make_finish_node(2, 0),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let count = graph
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
            .count();
        assert_eq!(count, 3);
    }

    #[test]
    fn graph_branch_edge_condition_index_is_correct() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1, 2], Some(3)),
                make_finish_node(1, 0),
                make_finish_node(2, 0),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let branches: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
            .collect();
        assert_eq!(
            branches[0].edge_type,
            EdgeType::Branch { condition_index: 0 }
        );
        assert_eq!(branches[0].to_step, StepIdx::new(1));
        assert_eq!(
            branches[1].edge_type,
            EdgeType::Branch { condition_index: 1 }
        );
        assert_eq!(branches[1].to_step, StepIdx::new(2));
        assert_eq!(
            branches[2].edge_type,
            EdgeType::Branch { condition_index: 2 }
        );
        assert_eq!(branches[2].to_step, StepIdx::new(3));
    }

    #[test]
    fn graph_branch_edges_have_labels() {
        let parts = make_simple_parts(
            vec![make_choose_node(0, &[1], None), make_finish_node(1, 0)],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(graph.edges.first().is_some_and(|e| e.label.is_some()));
    }

    #[test]
    fn graph_error_handler_produces_error_route() {
        let parts = make_simple_parts(
            vec![
                make_error_handler_node(0, 1, 2),
                make_nop_node(1, None),
                make_nop_node(2, None),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let errs: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::ErrorRoute)
            .collect();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].label, Some(String::from("handler")));
    }

    #[test]
    fn graph_error_handler_produces_body_edge() {
        let parts = make_simple_parts(
            vec![
                make_error_handler_node(0, 1, 2),
                make_nop_node(1, None),
                make_nop_node(2, None),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let bodies: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| e.label == Some(String::from("body")))
            .collect();
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].to_step, StepIdx::new(1));
    }

    #[test]
    fn graph_node_badge_equality() {
        assert_eq!(NodeBadge::ActionId(5), NodeBadge::ActionId(5));
        assert_ne!(NodeBadge::ActionId(5), NodeBadge::ActionId(6));
        assert_eq!(NodeBadge::SecretSensitive, NodeBadge::SecretSensitive);
        assert_ne!(NodeBadge::SecretSensitive, NodeBadge::StrictDurable);
    }

    #[test]
    fn graph_edge_type_equality() {
        assert_eq!(EdgeType::Sequential, EdgeType::Sequential);
        assert_eq!(
            EdgeType::Branch { condition_index: 0 },
            EdgeType::Branch { condition_index: 0 }
        );
        assert_ne!(
            EdgeType::Branch { condition_index: 0 },
            EdgeType::Branch { condition_index: 1 }
        );
        assert_ne!(EdgeType::ErrorRoute, EdgeType::RetryRoute);
    }

    #[test]
    fn graph_workflow_graph_clone_roundtrip() {
        let parts = make_simple_parts(vec![make_nop_node(0, Some(1)), make_finish_node(1, 0)], 0);
        let graph = build_graph_from_parts(parts);
        let cloned = graph.clone();
        assert_eq!(cloned.nodes.len(), graph.nodes.len());
        assert_eq!(cloned.edges.len(), graph.edges.len());
        assert_eq!(cloned.workflow_name, graph.workflow_name);
        assert_eq!(cloned.entry_step, graph.entry_step);
        assert_eq!(cloned.slot_count, graph.slot_count);
    }

    #[test]
    fn graph_retry_check_produces_retry_and_exhausted() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        exhausted: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.edge_type == EdgeType::RetryRoute)
                .count(),
            1
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.edge_type == EdgeType::ErrorRoute)
                .count(),
            1
        );
    }

    #[test]
    fn graph_repeat_start_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatStart {
                        max_attempts: 3,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("body")))
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("done")))
        );
    }

    #[test]
    fn graph_repeat_start_gets_retry_max_badge() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatStart {
                        max_attempts: 5,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph.nodes[0]
                .badges
                .iter()
                .any(|b| *b == NodeBadge::RetryMax(5))
        );
    }

    #[test]
    fn graph_foreach_start_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.label == Some(String::from("body")))
                .count(),
            1
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.label == Some(String::from("done")))
                .count(),
            1
        );
    }

    #[test]
    fn graph_together_start_produces_branch_and_join() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                        join: StepIdx::new(3),
                    },
                },
                make_nop_node(1, None),
                make_nop_node(2, None),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.label == Some(String::from("branch-0"))
                    || e.label == Some(String::from("branch-1")))
                .count(),
            2
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.label == Some(String::from("join")))
                .count(),
            1
        );
    }

    #[test]
    fn graph_nop_node_has_no_badges() {
        let parts = make_simple_parts(vec![make_nop_node(0, None)], 0);
        let graph = build_graph_from_parts(parts);
        assert!(graph.nodes[0].badges.is_empty());
    }

    #[test]
    fn graph_workflow_node_clone_roundtrip() {
        let node = WorkflowNode {
            step_idx: StepIdx::new(42),
            kind_name: String::from("Do"),
            visual: node_kind_to_visual(&CompiledNodeKind::Nop),
            position: Some((100.0, 200.0)),
            badges: vec![NodeBadge::ActionId(7)],
        };
        let cloned = node.clone();
        assert_eq!(cloned.step_idx, node.step_idx);
        assert_eq!(cloned.kind_name, node.kind_name);
        assert_eq!(cloned.position, node.position);
        assert_eq!(cloned.badges, node.badges);
    }

    #[test]
    fn graph_workflow_edge_clone_roundtrip() {
        let edge = WorkflowEdge {
            from_step: StepIdx::new(0),
            to_step: StepIdx::new(1),
            edge_type: EdgeType::Branch { condition_index: 3 },
            label: Some(String::from("cond-3")),
        };
        let cloned = edge.clone();
        assert_eq!(cloned.from_step, edge.from_step);
        assert_eq!(cloned.to_step, edge.to_step);
        assert_eq!(cloned.edge_type, edge.edge_type);
        assert_eq!(cloned.label, edge.label);
    }

    #[test]
    fn graph_node_badge_debug_output() {
        let badge = NodeBadge::ActionId(17);
        let debug_str = format!("{badge:?}");
        assert!(debug_str.contains("ActionId"));
        assert!(debug_str.contains("17"));
    }

    #[test]
    fn graph_edge_type_debug_output() {
        let debug_str = format!("{:?}", EdgeType::RetryRoute);
        assert!(debug_str.contains("RetryRoute"));
    }

    #[test]
    fn graph_on_error_field_produces_no_separate_edge() {
        let mut node = make_nop_node(0, Some(1));
        node.on_error = Some(StepIdx::new(2));
        let parts = make_simple_parts(
            vec![node, make_finish_node(1, 0), make_nop_node(2, None)],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::Sequential);
    }

    #[test]
    fn graph_jump_node_produces_jump_edge() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(1),
                    },
                },
                make_finish_node(1, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].label, Some(String::from("jump")));
        assert_eq!(graph.edges[0].to_step, StepIdx::new(1));
    }

    #[test]
    fn graph_collect_start_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::CollectStart {
                        source: SlotIdx::new(0),
                        limit: 10,
                        page_size: 5,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("body")))
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("done")))
        );
    }

    #[test]
    fn graph_reduce_start_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ReduceStart {
                        input: SlotIdx::new(0),
                        accumulator: SlotIdx::new(1),
                        initial: ConstIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("body")))
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("done")))
        );
    }

    #[test]
    fn graph_together_branch_produces_join() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherBranch {
                        branch: 0,
                        entry: StepIdx::new(1),
                        join: StepIdx::new(2),
                        accumulator: SlotIdx::new(0),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let joins: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| e.label == Some(String::from("join")))
            .collect();
        assert_eq!(joins.len(), 1);
        assert_eq!(joins[0].to_step, StepIdx::new(2));
    }

    #[test]
    fn graph_choose_otherwise_label() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1], Some(2)),
                make_finish_node(1, 0),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let oth = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("otherwise")));
        assert!(oth.is_some());
        assert_eq!(oth.map(|e| e.to_step), Some(StepIdx::new(2)));
    }

    #[test]
    fn graph_node_visual_matches_kind() {
        let parts = make_simple_parts(
            vec![make_do_node(0, 1, 0, Some(1)), make_finish_node(1, 0)],
            0,
        );
        let graph = build_graph_from_parts(parts);
        use crate::workflow::node_mapping::{NEON_ORANGE, NEON_TEAL, NodeCategory, NodeShape};
        assert_eq!(graph.nodes[0].visual.category, NodeCategory::External);
        assert_eq!(graph.nodes[0].visual.shape, NodeShape::RoundedRect);
        assert_eq!(graph.nodes[0].visual.color, NEON_ORANGE);
        assert_eq!(graph.nodes[1].visual.category, NodeCategory::Terminal);
        assert_eq!(graph.nodes[1].visual.shape, NodeShape::Pill);
        assert_eq!(graph.nodes[1].visual.color, NEON_TEAL);
    }

    #[test]
    fn graph_all_node_badge_variants_constructible() {
        let _a = NodeBadge::ActionId(1);
        let _r = NodeBadge::RetryMax(3);
        let _t = NodeBadge::Timeout(30);
        let _s = NodeBadge::SecretSensitive;
        let _d = NodeBadge::StrictDurable;
        let _f = NodeBadge::RecentFailures(5);
    }

    #[test]
    fn graph_all_edge_type_variants_constructible() {
        let _s = EdgeType::Sequential;
        let _b = EdgeType::Branch { condition_index: 0 };
        let _e = EdgeType::ErrorRoute;
        let _r = EdgeType::RetryRoute;
        let _j = EdgeType::JoinRoute;
    }

    #[test]
    fn graph_workflow_node_debug() {
        let node = WorkflowNode {
            step_idx: StepIdx::new(0),
            kind_name: String::from("Nop"),
            visual: node_kind_to_visual(&CompiledNodeKind::Nop),
            position: None,
            badges: vec![],
        };
        assert!(format!("{node:?}").contains("Nop"));
    }

    #[test]
    fn graph_workflow_edge_debug() {
        let edge = WorkflowEdge {
            from_step: StepIdx::new(0),
            to_step: StepIdx::new(1),
            edge_type: EdgeType::JoinRoute,
            label: None,
        };
        assert!(format!("{edge:?}").contains("JoinRoute"));
    }

    #[test]
    fn graph_workflow_graph_debug() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert!(format!("{graph:?}").contains("test-workflow"));
    }

    #[test]
    fn graph_repeat_attempt_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatAttempt {
                        attempt_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("body")))
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("done")))
        );
    }

    #[test]
    fn graph_repeat_check_produces_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatCheck {
                        attempt_slot: SlotIdx::new(0),
                        done: StepIdx::new(1),
                    },
                },
                make_finish_node(1, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("done")))
        );
    }

    #[test]
    fn graph_collect_page_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::CollectPage {
                        collector_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("body")))
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("done")))
        );
    }

    #[test]
    fn graph_reduce_next_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ReduceNext {
                        iterator_slot: SlotIdx::new(0),
                        accumulator: SlotIdx::new(1),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("body")))
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.label == Some(String::from("done")))
        );
    }

    #[test]
    fn graph_choose_slot_produces_branch_edges() {
        use vb_core::workflow::SlotBranch;
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: Box::new([SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        }]),
                        otherwise: Some(StepIdx::new(2)),
                    },
                },
                make_finish_node(1, 0),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn graph_retry_check_gets_retry_max_badge() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        exhausted: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert!(
            graph.nodes[0]
                .badges
                .iter()
                .any(|b| *b == NodeBadge::RetryMax(3))
        );
    }

    #[test]
    fn graph_foreach_next_produces_body_and_done() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.label == Some(String::from("body")))
                .count(),
            1
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.label == Some(String::from("done")))
                .count(),
            1
        );
    }

    // =======================================================================
    // Comprehensive build_graph tests
    // =======================================================================

    // -- 1. Empty graph (single finish node = minimal valid graph) -----------

    #[test]
    fn build_graph_single_node_has_no_sequential_edges() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        let sequential: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::Sequential)
            .collect();
        assert!(sequential.is_empty());
    }

    #[test]
    fn build_graph_single_node_entry_step_is_zero() {
        let parts = make_simple_parts(vec![make_finish_node(0, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.entry_step, StepIdx::new(0));
    }

    // -- 2. Single node graph (various kinds) --------------------------------

    #[test]
    fn build_graph_nop_no_next_no_edges() {
        let parts = make_simple_parts(vec![make_nop_node(0, None)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn build_graph_do_node_no_next_no_sequential_edge() {
        let parts = make_simple_parts(vec![make_do_node(0, 5, 0, None)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty());
    }

    // -- 3. Linear chain (A -> B -> C) --------------------------------------

    #[test]
    fn build_graph_linear_chain_edge_sources_and_targets() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_nop_node(1, Some(2)),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].from_step, StepIdx::new(0));
        assert_eq!(graph.edges[0].to_step, StepIdx::new(1));
        assert_eq!(graph.edges[1].from_step, StepIdx::new(1));
        assert_eq!(graph.edges[1].to_step, StepIdx::new(2));
    }

    #[test]
    fn build_graph_linear_chain_no_labels_on_sequential() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_nop_node(1, Some(2)),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        for edge in &graph.edges {
            assert!(edge.label.is_none());
        }
    }

    #[test]
    fn build_graph_five_node_chain_has_four_edges() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_nop_node(1, Some(2)),
                make_nop_node(2, Some(3)),
                make_nop_node(3, Some(4)),
                make_finish_node(4, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.edges.len(), 4);
    }

    // -- 4. Branching graph (Choose with branches) ---------------------------

    #[test]
    fn build_graph_choose_two_branches_correct_targets() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1, 2], None),
                make_finish_node(1, 0),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let branches: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
            .collect();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].to_step, StepIdx::new(1));
        assert_eq!(branches[1].to_step, StepIdx::new(2));
    }

    #[test]
    fn build_graph_choose_branch_labels_are_cond_indexed() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1, 2, 3], None),
                make_finish_node(1, 0),
                make_finish_node(2, 0),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let labels: Vec<String> = graph
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
            .filter_map(|e| e.label.clone())
            .collect();
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], "cond-0");
        assert_eq!(labels[1], "cond-1");
        assert_eq!(labels[2], "cond-2");
    }

    #[test]
    fn build_graph_choose_with_otherwise_has_extra_branch() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1], Some(2)),
                make_finish_node(1, 0),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let branches: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
            .collect();
        assert_eq!(branches.len(), 2);
        let otherwise = branches
            .iter()
            .find(|e| e.label == Some(String::from("otherwise")));
        assert!(otherwise.is_some());
        assert_eq!(otherwise.map(|e| e.to_step), Some(StepIdx::new(2)));
    }

    // -- 5. Loop graph (ForEach with body) -----------------------------------

    #[test]
    fn build_graph_foreach_start_body_edge_is_sequential() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body_edge = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body_edge.is_some());
        assert_eq!(
            body_edge.map(|e| e.edge_type.clone()),
            Some(EdgeType::Sequential)
        );
    }

    #[test]
    fn build_graph_foreach_start_done_edge_is_join() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let done_edge = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("done")));
        assert!(done_edge.is_some());
        assert_eq!(
            done_edge.map(|e| e.edge_type.clone()),
            Some(EdgeType::JoinRoute)
        );
    }

    #[test]
    fn build_graph_foreach_next_body_edge_is_retry_route() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body_edge = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body_edge.is_some());
        assert_eq!(
            body_edge.map(|e| e.edge_type.clone()),
            Some(EdgeType::RetryRoute)
        );
    }

    // -- 6. Parallel graph (Together with branches) --------------------------

    #[test]
    fn build_graph_together_start_branch_edges_are_sequential() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                        join: StepIdx::new(3),
                    },
                },
                make_nop_node(1, None),
                make_nop_node(2, None),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let branch_edges: Vec<&WorkflowEdge> = graph
            .edges
            .iter()
            .filter(|e| e.label.as_ref().map_or(false, |l| l.starts_with("branch-")))
            .collect();
        assert_eq!(branch_edges.len(), 2);
        for edge in &branch_edges {
            assert_eq!(edge.edge_type, EdgeType::Sequential);
        }
    }

    // -- 7. Error/retry edges ------------------------------------------------

    #[test]
    fn build_graph_retry_check_body_is_retry_route() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        exhausted: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let retry = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("retry")));
        assert!(retry.is_some());
        assert_eq!(
            retry.map(|e| e.edge_type.clone()),
            Some(EdgeType::RetryRoute)
        );
        assert_eq!(retry.map(|e| e.to_step), Some(StepIdx::new(1)));
    }

    #[test]
    fn build_graph_retry_check_exhausted_is_error_route() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        exhausted: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let exhausted = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("exhausted")));
        assert!(exhausted.is_some());
        assert_eq!(
            exhausted.map(|e| e.edge_type.clone()),
            Some(EdgeType::ErrorRoute)
        );
        assert_eq!(exhausted.map(|e| e.to_step), Some(StepIdx::new(2)));
    }

    #[test]
    fn build_graph_error_handler_body_is_sequential() {
        let parts = make_simple_parts(
            vec![
                make_error_handler_node(0, 1, 2),
                make_nop_node(1, None),
                make_nop_node(2, None),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body.is_some());
        assert_eq!(
            body.map(|e| e.edge_type.clone()),
            Some(EdgeType::Sequential)
        );
    }

    #[test]
    fn build_graph_repeat_start_body_is_retry_route() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatStart {
                        max_attempts: 3,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body.is_some());
        assert_eq!(
            body.map(|e| e.edge_type.clone()),
            Some(EdgeType::RetryRoute)
        );
    }

    #[test]
    fn build_graph_repeat_attempt_body_is_retry_route() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatAttempt {
                        attempt_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body.is_some());
        assert_eq!(
            body.map(|e| e.edge_type.clone()),
            Some(EdgeType::RetryRoute)
        );
    }

    // -- 8. Next reference edges ---------------------------------------------

    #[test]
    fn build_graph_node_next_reference_creates_sequential_edge() {
        let parts = make_simple_parts(vec![make_nop_node(0, Some(1)), make_finish_node(1, 0)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::Sequential);
    }

    #[test]
    fn build_graph_forward_edge_is_not_labeled_loop() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_nop_node(1, Some(2)),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        for edge in &graph.edges {
            assert_ne!(edge.label, Some(String::from("loop")));
        }
    }

    // -- 9. Node badge computation -------------------------------------------

    #[test]
    fn build_graph_do_node_action_id_badge_value() {
        let parts = make_simple_parts(vec![make_do_node(0, 42, 0, None)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(
            graph.nodes[0].badges.first(),
            Some(&NodeBadge::ActionId(42))
        );
    }

    #[test]
    fn build_graph_repeat_start_max_attempts_badge_value() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatStart {
                        max_attempts: 7,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes[0].badges.first(), Some(&NodeBadge::RetryMax(7)));
    }

    // -- 10. Edge type classification ----------------------------------------

    #[test]
    fn build_graph_edge_to_join_target_classified_as_join() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachJoin {
                        output: SlotIdx::new(0),
                    },
                },
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::JoinRoute);
    }

    #[test]
    fn build_graph_edge_to_repeat_finish_classified_as_join() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatFinish {
                        result: SlotIdx::new(0),
                    },
                },
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::JoinRoute);
    }

    #[test]
    fn build_graph_edge_to_nop_is_sequential() {
        let parts = make_simple_parts(vec![make_nop_node(0, Some(1)), make_nop_node(1, None)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::Sequential);
    }

    // -- 11. Stress test (many nodes) ----------------------------------------

    fn make_long_chain_nodes(count: usize) -> Vec<CompiledNode> {
        let mut nodes = Vec::with_capacity(count);
        for i in 0..count.saturating_sub(1) {
            let next_id = u16::try_from(i.saturating_add(1)).unwrap_or(u16::MAX);
            nodes.push(make_nop_node(
                u16::try_from(i).unwrap_or(u16::MAX),
                Some(next_id),
            ));
        }
        let last = u16::try_from(count.saturating_sub(1)).unwrap_or(u16::MAX);
        nodes.push(make_finish_node(last, 0));
        nodes
    }

    #[test]
    fn build_graph_10_node_chain() {
        let nodes = make_long_chain_nodes(10);
        let parts = make_simple_parts(nodes, 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes.len(), 10);
        assert_eq!(graph.edges.len(), 9);
    }

    #[test]
    fn build_graph_100_node_chain() {
        let nodes = make_long_chain_nodes(100);
        let parts = make_simple_parts(nodes, 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes.len(), 100);
        assert_eq!(graph.edges.len(), 99);
    }

    #[test]
    fn build_graph_500_node_chain() {
        let nodes = make_long_chain_nodes(500);
        let parts = make_simple_parts(nodes, 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes.len(), 500);
        assert_eq!(graph.edges.len(), 499);
    }

    #[test]
    fn build_graph_stress_all_edges_are_sequential_forward() {
        let nodes = make_long_chain_nodes(100);
        let parts = make_simple_parts(nodes, 0);
        let graph = build_graph_from_parts(parts);
        for edge in &graph.edges {
            assert_eq!(edge.edge_type, EdgeType::Sequential);
            assert!(edge.label.is_none());
        }
    }

    // -- 12. Mixed graph (complex topology) ----------------------------------

    #[test]
    fn build_graph_choose_with_sequential_continuation() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1, 2], None),
                make_nop_node(1, Some(3)),
                make_nop_node(2, Some(3)),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 4);
        let branches = graph
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
            .count();
        assert_eq!(branches, 2);
        let sequential = graph
            .edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::Sequential)
            .count();
        assert_eq!(sequential, 2);
    }

    #[test]
    fn build_graph_jump_edge_is_sequential_type() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(1),
                    },
                },
                make_finish_node(1, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::Sequential);
        assert_eq!(graph.edges[0].to_step, StepIdx::new(1));
    }

    // -- ChooseSlot edge labels ----------------------------------------------

    #[test]
    fn build_graph_choose_slot_branch_labels_use_slot_prefix() {
        use vb_core::workflow::SlotBranch;
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: Box::new([
                            SlotBranch {
                                condition: SlotIdx::new(0),
                                target: StepIdx::new(1),
                            },
                            SlotBranch {
                                condition: SlotIdx::new(1),
                                target: StepIdx::new(2),
                            },
                        ]),
                        otherwise: None,
                    },
                },
                make_finish_node(1, 0),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let labels: Vec<String> = graph
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Branch { .. }))
            .filter_map(|e| e.label.clone())
            .collect();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0], "slot-cond-0");
        assert_eq!(labels[1], "slot-cond-1");
    }

    // -- Collect/Reduce edge types -------------------------------------------

    #[test]
    fn build_graph_collect_next_body_is_retry_route() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::CollectNext {
                        collector_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body.is_some());
        assert_eq!(
            body.map(|e| e.edge_type.clone()),
            Some(EdgeType::RetryRoute)
        );
    }

    #[test]
    fn build_graph_reduce_next_body_is_retry_route() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ReduceNext {
                        iterator_slot: SlotIdx::new(0),
                        accumulator: SlotIdx::new(1),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body.is_some());
        assert_eq!(
            body.map(|e| e.edge_type.clone()),
            Some(EdgeType::RetryRoute)
        );
    }

    #[test]
    fn build_graph_collect_start_body_is_sequential() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::CollectStart {
                        source: SlotIdx::new(0),
                        limit: 10,
                        page_size: 5,
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body.is_some());
        assert_eq!(
            body.map(|e| e.edge_type.clone()),
            Some(EdgeType::Sequential)
        );
    }

    #[test]
    fn build_graph_reduce_start_body_is_sequential() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ReduceStart {
                        input: SlotIdx::new(0),
                        accumulator: SlotIdx::new(1),
                        initial: ConstIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                make_nop_node(1, None),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let body = graph
            .edges
            .iter()
            .find(|e| e.label == Some(String::from("body")));
        assert!(body.is_some());
        assert_eq!(
            body.map(|e| e.edge_type.clone()),
            Some(EdgeType::Sequential)
        );
    }

    // -- TogetherStart with empty branches -----------------------------------

    #[test]
    fn build_graph_together_start_zero_branches_only_join_edge() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: Box::new([]),
                        join: StepIdx::new(1),
                    },
                },
                make_finish_node(1, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].label, Some(String::from("join")));
        assert_eq!(graph.edges[0].edge_type, EdgeType::JoinRoute);
    }

    // -- WorkflowGraph clone equality complex --------------------------------

    #[test]
    fn build_graph_clone_equality_complex() {
        let parts = make_simple_parts(
            vec![
                make_error_handler_node(0, 1, 2),
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Do {
                        action: ActionId::new(5),
                        input: SlotIdx::new(0),
                    },
                },
                make_nop_node(2, None),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let cloned = graph.clone();
        assert_eq!(graph, cloned);
    }

    // -- Node step_idx consistency -------------------------------------------

    #[test]
    fn build_graph_step_indices_match_node_order() {
        let parts = make_simple_parts(
            vec![
                make_nop_node(0, Some(1)),
                make_do_node(1, 10, 0, Some(2)),
                make_finish_node(2, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        for (i, node) in graph.nodes.iter().enumerate() {
            assert_eq!(
                node.step_idx,
                StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX))
            );
        }
    }

    // -- EdgeType/NodeBadge hash consistency ---------------------------------

    #[test]
    fn edge_type_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(EdgeType::Sequential);
        set.insert(EdgeType::Branch { condition_index: 0 });
        set.insert(EdgeType::ErrorRoute);
        set.insert(EdgeType::RetryRoute);
        set.insert(EdgeType::JoinRoute);
        assert_eq!(set.len(), 5);
        set.insert(EdgeType::Sequential);
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn node_badge_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NodeBadge::ActionId(1));
        set.insert(NodeBadge::ActionId(2));
        set.insert(NodeBadge::RetryMax(3));
        set.insert(NodeBadge::Timeout(10));
        set.insert(NodeBadge::SecretSensitive);
        set.insert(NodeBadge::StrictDurable);
        set.insert(NodeBadge::RecentFailures(5));
        assert_eq!(set.len(), 7);
        set.insert(NodeBadge::ActionId(1));
        assert_eq!(set.len(), 7);
    }

    // -- Complex: error handler wrapping retry check -------------------------

    #[test]
    fn build_graph_error_handler_wrapping_retry_check() {
        let parts = make_simple_parts(
            vec![
                make_error_handler_node(0, 1, 4),
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(2),
                        exhausted: StepIdx::new(3),
                    },
                },
                make_nop_node(2, None),
                make_finish_node(3, 0),
                make_finish_node(4, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 4);
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.edge_type == EdgeType::ErrorRoute)
                .count(),
            2
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|e| e.edge_type == EdgeType::RetryRoute)
                .count(),
            1
        );
    }

    // -- Do node badge count -------------------------------------------------

    #[test]
    fn build_graph_do_node_badge_list_length_is_one() {
        let parts = make_simple_parts(vec![make_do_node(0, 5, 0, None)], 0);
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.nodes[0].badges.len(), 1);
    }

    // -- TogetherStart with 3 branches --------------------------------------

    #[test]
    fn build_graph_together_start_three_branches() {
        let parts = make_simple_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: Box::new([StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]),
                        join: StepIdx::new(4),
                    },
                },
                make_nop_node(1, None),
                make_nop_node(2, None),
                make_nop_node(3, None),
                make_finish_node(4, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        let branch_labels: Vec<String> = graph
            .edges
            .iter()
            .filter_map(|e| {
                if e.label.as_ref().map_or(false, |l| l.starts_with("branch-")) {
                    e.label.clone()
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(branch_labels.len(), 3);
        assert!(branch_labels.contains(&String::from("branch-0")));
        assert!(branch_labels.contains(&String::from("branch-1")));
        assert!(branch_labels.contains(&String::from("branch-2")));
    }

    // -- Edge bounds checks --------------------------------------------------

    #[test]
    fn build_graph_all_edges_have_valid_steps() {
        let parts = make_simple_parts(
            vec![
                make_choose_node(0, &[1, 2], Some(3)),
                make_finish_node(1, 0),
                make_finish_node(2, 0),
                make_finish_node(3, 0),
            ],
            0,
        );
        let graph = build_graph_from_parts(parts);
        for edge in &graph.edges {
            assert!(edge.from_step.get() < 4, "from_step out of bounds");
            assert!(edge.to_step.get() < 4, "to_step out of bounds");
        }
    }

    // -- ViewportRect edge cases ---------------------------------------------

    #[test]
    fn viewport_rect_zero_size_no_intersection() {
        let vr = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        assert!(!vr.intersects(0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn viewport_rect_adjacent_top_no_overlap() {
        let vr = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(!vr.intersects(0.0, -50.0, 100.0, 50.0));
    }

    // -- Combined next + badge -----------------------------------------------

    #[test]
    fn build_graph_do_node_with_next_produces_sequential_and_badge() {
        let parts = make_simple_parts(
            vec![make_do_node(0, 99, 0, Some(1)), make_finish_node(1, 0)],
            0,
        );
        let graph = build_graph_from_parts(parts);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, EdgeType::Sequential);
        assert!(
            graph.nodes[0]
                .badges
                .iter()
                .any(|b| *b == NodeBadge::ActionId(99))
        );
    }

    // -- NodeBadge/EdgeType inequality ---------------------------------------

    #[test]
    fn node_badge_different_variants_not_equal() {
        assert_ne!(NodeBadge::ActionId(0), NodeBadge::RetryMax(0));
        assert_ne!(NodeBadge::Timeout(0), NodeBadge::RecentFailures(0));
        assert_ne!(NodeBadge::SecretSensitive, NodeBadge::StrictDurable);
    }

    #[test]
    fn edge_type_different_variants_not_equal() {
        assert_ne!(EdgeType::Sequential, EdgeType::JoinRoute);
        assert_ne!(EdgeType::ErrorRoute, EdgeType::RetryRoute);
        assert_ne!(
            EdgeType::Branch { condition_index: 0 },
            EdgeType::ErrorRoute
        );
    }
}
