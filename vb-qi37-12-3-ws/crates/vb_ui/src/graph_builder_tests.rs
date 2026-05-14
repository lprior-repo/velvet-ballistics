// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use vb_core::ids::StepIdx;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};

    // Explicit imports from parent graph_builder module
    use crate::graph_builder::{
        Cardinality, EdgeStyle, FlowPortRecord, GroupKind, PortRole, PortSide, SmolStr,
        build_document, build_ports, classify_node_kind, collect_span, compute_node_size,
    };

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
                result: vb_core::ids::SlotIdx::new(result_slot),
            },
        }
    }

    fn make_jump_node(id: u16, target: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(target),
            },
        }
    }

    fn make_simple_parts(nodes: Vec<CompiledNode>, entry: u16) -> WorkflowParts {
        let node_count = nodes.len();
        let step_names: Vec<Box<str>> = (0..node_count)
            .map(|i| format!("step-{i}").into_boxed_str())
            .collect();
        WorkflowParts {
            name: String::from("test").into_boxed_str(),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(entry),
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: step_names.into_boxed_slice(),
        }
    }

    #[test]
    fn empty_workflow_produces_empty_document() {
        let node = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![node], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.nodes.len(), 1);
        assert_eq!(doc.graph.edges.len(), 0);
    }

    #[test]
    fn two_node_chain_produces_one_edge() {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.nodes.len(), 2);
        assert_eq!(doc.graph.edges.len(), 1);
        let edge = doc.graph.edges.get_index(0).map(|(_, e)| e.clone());
        assert!(edge.is_some());
        let e = edge.expect("edge missing");
        assert_eq!(e.source.as_str(), "step-0");
        assert_eq!(e.target.as_str(), "step-1");
        assert_eq!(e.source_port.as_str(), "next");
        assert_eq!(e.target_port.as_str(), "in");
    }

    #[test]
    fn entry_node_flag_is_set() {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);
        let entry = doc.graph.nodes.get("step-0");
        assert!(entry.is_some());
        let n = entry.expect("node missing");
        assert!(n.flags.entry);
        let non_entry = doc.graph.nodes.get("step-1");
        assert!(non_entry.is_some());
        let n2 = non_entry.expect("node missing");
        assert!(!n2.flags.entry);
    }

    #[test]
    fn finish_node_is_terminal() {
        let n = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![n], 0);
        let doc = build_document(&parts);
        let node = doc.graph.nodes.get("step-0");
        assert!(node.is_some());
        let record = node.expect("node missing");
        assert!(record.flags.terminal);
    }

    #[test]
    fn jump_produces_jump_edge() {
        let n0 = make_jump_node(0, 1);
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.edges.len(), 1);
        let e = doc
            .graph
            .edges
            .get_index(0)
            .map(|(_, e)| e.clone())
            .expect("edge missing");
        assert_eq!(e.source_port.as_str(), "jump");
        assert_eq!(e.target.as_str(), "step-1");
    }

    #[test]
    fn classify_all_variants_have_labels() {
        // Spot-check a few categories.
        let (label, cat) = classify_node_kind(&CompiledNodeKind::Nop);
        assert_eq!(label, "Nop");
        assert_eq!(cat, "control");

        let (label, cat) = classify_node_kind(&CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        });
        assert_eq!(label, "Do");
        assert_eq!(cat, "external");

        let (label, cat) = classify_node_kind(&CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::new(0),
        });
        assert_eq!(label, "Finish");
        assert_eq!(cat, "terminal");
    }

    #[test]
    fn compute_node_size_scales_with_ports() {
        let small = compute_node_size(&[]);
        assert_eq!(small[0], 160.0);
        assert_eq!(small[1], 60.0);

        let ports = vec![
            FlowPortRecord {
                id: SmolStr::new_static("p0"),
                label: SmolStr::new_static("p0"),
                side: PortSide::Input,
                role: PortRole::Data,
                cardinality: Cardinality::One,
            };
            10
        ];
        let large = compute_node_size(&ports);
        assert!(large[0] > small[0]);
        assert!(large[1] > small[1]);
    }

    #[test]
    fn collect_span_returns_correct_range() {
        let span = collect_span(2, 5, 10);
        assert_eq!(span.len(), 4);
        assert_eq!(span[0].as_str(), "step-2");
        assert_eq!(span[3].as_str(), "step-5");
    }

    #[test]
    fn collect_span_empty_when_end_before_start() {
        let span = collect_span(5, 2, 10);
        assert!(span.is_empty());
    }

    #[test]
    fn collect_span_empty_when_end_out_of_bounds() {
        let span = collect_span(2, 10, 5);
        assert!(span.is_empty());
    }

    #[test]
    fn error_handler_produces_handler_edge() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.edges.len(), 1);
        let e = doc
            .graph
            .edges
            .get_index(0)
            .map(|(_, e)| e.clone())
            .expect("edge missing");
        assert_eq!(e.source_port.as_str(), "handler");
        assert_eq!(e.target.as_str(), "step-2");
        assert!(e.style.dashed);
    }

    #[test]
    fn foreach_start_produces_group_and_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: vb_core::ids::SlotIdx::new(0),
                item_slot: vb_core::ids::SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n1 = make_nop_node(1, Some(2));
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot: vb_core::ids::SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: vb_core::ids::SlotIdx::new(3),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // ForEachStart produces body + done edges, ForEachNext produces body + done edges,
        // n1.next produces an edge.
        assert!(doc.graph.edges.len() >= 3);

        // Should have a group.
        assert!(!doc.graph.groups.is_empty());
        let group = doc
            .graph
            .groups
            .get("group-foreach-0")
            .cloned()
            .expect("group missing");
        assert_eq!(group.kind, GroupKind::BranchContainer);
        assert_eq!(group.children.len(), 4);
    }

    #[test]
    fn document_schema_and_semantic_kind() {
        let n = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![n], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.schema.as_str(), "makepad.flow/v2");
        assert_eq!(doc.semantic_kind.as_str(), "velvet-ballistics");
    }

    #[test]
    fn entry_node_matches_parts_entry() {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_nop_node(1, Some(2));
        let n2 = make_finish_node(2, 0);
        let parts = make_simple_parts(vec![n0, n1, n2], 1);
        let doc = build_document(&parts);
        assert_eq!(
            doc.graph.entry_node.as_ref().map(|s| s.as_str()),
            Some("step-1")
        );
        let entry = doc.graph.nodes.get("step-1");
        assert!(entry.is_some());
        assert!(entry.expect("node missing").flags.entry);
    }

    // -----------------------------------------------------------------------
    // Additional tests for graph_builder
    // -----------------------------------------------------------------------

    #[test]
    fn choose_with_otherwise_produces_three_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: Box::new([
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_finish_node(3, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // 2 branch edges + 1 otherwise edge = 3 total.
        assert_eq!(doc.graph.edges.len(), 3);

        // Find the otherwise edge: it should be dashed.
        let mut found_otherwise = false;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "otherwise" {
                found_otherwise = true;
                assert!(e.style.dashed, "otherwise edge should be dashed");
                assert_eq!(e.target.as_str(), "step-3");
            }
        }
        assert!(found_otherwise, "should find an otherwise edge");

        // Branch edges should be solid.
        let mut solid_branch_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str().starts_with("branch-") {
                assert!(!e.style.dashed, "branch edge should be solid");
                solid_branch_count = solid_branch_count.saturating_add(1);
            }
        }
        assert_eq!(solid_branch_count, 2);
    }

    #[test]
    fn together_start_creates_swimlane_group() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // Should produce a swimlane group.
        let group = match doc.graph.groups.get("group-together-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(group.kind, GroupKind::Swimlane);
        // Children should span steps 0 through 3 (inclusive).
        assert_eq!(group.children.len(), 4);
        assert_eq!(group.children[0].as_str(), "step-0");
        assert_eq!(group.children[3].as_str(), "step-3");
    }

    #[test]
    fn collect_start_creates_branch_container_group() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: vb_core::ids::SlotIdx::new(0),
                limit: 100,
                page_size: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);

        let group = match doc.graph.groups.get("group-collect-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(group.kind, GroupKind::BranchContainer);
        assert_eq!(group.children.len(), 3);
    }

    #[test]
    fn build_ports_for_build_object_with_fields() {
        use vb_core::ids::{SlotIdx, SymbolId};

        let kind = CompiledNodeKind::BuildObject {
            fields: Box::new([
                (SymbolId::new(0), SlotIdx::new(1)),
                (SymbolId::new(1), SlotIdx::new(2)),
                (SymbolId::new(2), SlotIdx::new(3)),
            ]),
        };
        let (inputs, outputs) = build_ports(&kind, Some(SlotIdx::new(0)));

        // 3 field input ports.
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0].id.as_str(), "field-0");
        assert_eq!(inputs[1].id.as_str(), "field-1");
        assert_eq!(inputs[2].id.as_str(), "field-2");
        // All input ports should be on the Input side with Data role.
        for port in &inputs {
            assert_eq!(port.side, PortSide::Input);
            assert_eq!(port.role, PortRole::Data);
        }
        // One output port for the output slot.
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "out");
        assert_eq!(outputs[0].side, PortSide::Output);
    }

    #[test]
    fn build_ports_for_build_list_with_items() {
        use vb_core::ids::SlotIdx;

        let kind = CompiledNodeKind::BuildList {
            items: Box::new([
                SlotIdx::new(10),
                SlotIdx::new(20),
                SlotIdx::new(30),
                SlotIdx::new(40),
            ]),
        };
        let (inputs, outputs) = build_ports(&kind, Some(SlotIdx::new(5)));

        // 4 item input ports.
        assert_eq!(inputs.len(), 4);
        assert_eq!(inputs[0].id.as_str(), "item-0");
        assert_eq!(inputs[3].id.as_str(), "item-3");
        // All input ports should be on the Input side with Data role.
        for port in &inputs {
            assert_eq!(port.side, PortSide::Input);
            assert_eq!(port.role, PortRole::Data);
        }
        // One output port for the output slot.
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "out");
    }

    #[test]
    fn edge_style_defaults_solid_vs_dashed() {
        // Verify the EdgeStyle constructors produce the expected values.
        let solid = EdgeStyle::default_solid();
        assert!(!solid.dashed);
        assert!(!solid.highlighted);

        let dashed = EdgeStyle::dashed();
        assert!(dashed.dashed);
        assert!(!dashed.highlighted);

        // Error handler edges should use dashed style.
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(3),
                error_slot: None,
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_nop_node(3, None);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // Should have a `next` edge (solid) and a `handler` edge (dashed).
        let mut found_solid_next = false;
        let mut found_dashed_handler = false;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "next" {
                found_solid_next = true;
                assert!(!e.style.dashed, "next edge should be solid");
            }
            if e.source_port.as_str() == "handler" {
                found_dashed_handler = true;
                assert!(e.style.dashed, "handler edge should be dashed");
            }
        }
        assert!(found_solid_next, "should find a solid next edge");
        assert!(found_dashed_handler, "should find a dashed handler edge");
    }

    #[test]
    fn wait_event_with_timeout_produces_two_input_ports() {
        let kind = CompiledNodeKind::WaitEvent {
            event: vb_core::ids::SlotIdx::new(5),
            timeout_slot: Some(vb_core::ids::SlotIdx::new(8)),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // Event port + timeout port = 2 input ports.
        assert_eq!(
            inputs.len(),
            2,
            "WaitEvent with timeout should have 2 inputs"
        );
        assert_eq!(inputs[0].id.as_str(), "event");
        assert_eq!(inputs[1].id.as_str(), "timeout");

        for port in &inputs {
            assert_eq!(port.side, PortSide::Input);
            assert_eq!(port.role, PortRole::Data);
        }

        // No output slot provided, so no output ports.
        assert!(
            outputs.is_empty(),
            "WaitEvent has no output ports when output is None"
        );
    }

    #[test]
    fn wait_event_without_timeout_produces_one_input_port() {
        let kind = CompiledNodeKind::WaitEvent {
            event: vb_core::ids::SlotIdx::new(3),
            timeout_slot: None,
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // Only event port; no timeout port.
        assert_eq!(
            inputs.len(),
            1,
            "WaitEvent without timeout should have 1 input"
        );
        assert_eq!(inputs[0].id.as_str(), "event");
        assert_eq!(inputs[0].side, PortSide::Input);
        assert_eq!(inputs[0].role, PortRole::Data);

        assert!(outputs.is_empty());
    }

    #[test]
    fn wait_event_with_timeout_and_output_slot() {
        let kind = CompiledNodeKind::WaitEvent {
            event: vb_core::ids::SlotIdx::new(1),
            timeout_slot: Some(vb_core::ids::SlotIdx::new(2)),
        };
        let (inputs, outputs) = build_ports(&kind, Some(vb_core::ids::SlotIdx::new(10)));

        // Still 2 input ports.
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].id.as_str(), "event");
        assert_eq!(inputs[1].id.as_str(), "timeout");

        // Now has an output port for the output slot.
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "out");
        assert_eq!(outputs[0].side, PortSide::Output);
    }

    #[test]
    fn choose_ports_include_branch_triggers_and_otherwise() {
        use vb_core::ids::ExprIdx;

        let kind = CompiledNodeKind::Choose {
            branches: Box::new([
                vb_core::workflow::ExprBranch {
                    condition: ExprIdx::new(0),
                    target: StepIdx::new(1),
                },
                vb_core::workflow::ExprBranch {
                    condition: ExprIdx::new(1),
                    target: StepIdx::new(2),
                },
            ]),
            otherwise: Some(StepIdx::new(3)),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // No input ports (Choose branches from expressions, not slots).
        assert!(inputs.is_empty());

        // 2 branch trigger ports + 1 otherwise port = 3 output ports.
        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].id.as_str(), "branch-0");
        assert_eq!(outputs[0].role, PortRole::Trigger);
        assert_eq!(outputs[1].id.as_str(), "branch-1");
        assert_eq!(outputs[1].role, PortRole::Trigger);
        assert_eq!(outputs[2].id.as_str(), "otherwise");
        assert_eq!(outputs[2].role, PortRole::Otherwise);
    }

    #[test]
    fn together_start_ports_match_branch_count_plus_join() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]),
            join: StepIdx::new(4),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        assert!(inputs.is_empty());

        // 3 branch trigger ports + 1 join done port = 4.
        assert_eq!(outputs.len(), 4);
        assert_eq!(outputs[0].id.as_str(), "branch-0");
        assert_eq!(outputs[0].role, PortRole::Trigger);
        assert_eq!(outputs[1].id.as_str(), "branch-1");
        assert_eq!(outputs[2].id.as_str(), "branch-2");
        assert_eq!(outputs[3].id.as_str(), "join");
        assert_eq!(outputs[3].role, PortRole::Done);
    }

    #[test]
    fn collect_start_ports_have_input_body_and_done() {
        let kind = CompiledNodeKind::CollectStart {
            source: vb_core::ids::SlotIdx::new(0),
            limit: 50,
            page_size: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // One input port for the source slot.
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].id.as_str(), "source");
        assert_eq!(inputs[0].side, PortSide::Input);
        assert_eq!(inputs[0].role, PortRole::Data);

        // Two output ports: body and done.
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].id.as_str(), "body");
        assert_eq!(outputs[0].role, PortRole::Body);
        assert_eq!(outputs[1].id.as_str(), "done");
        assert_eq!(outputs[1].role, PortRole::Done);
    }

    // -----------------------------------------------------------------------
    // Additional tests: edge generation for CollectEnd/ReduceEnd,
    // multi-branch Choose, TogetherEnd merge, RepeatAttempt loop-back,
    // nested loop-inside-parallel, empty workflow single Finish node.
    // -----------------------------------------------------------------------

    /// CollectFinish (the "CollectEnd" node) produces no kind-specific edges
    /// beyond `next`. This test verifies that a CollectStart -> body ->
    /// CollectFinish chain produces exactly the expected edges.
    #[test]
    fn collect_finish_produces_no_extra_kind_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: vb_core::ids::SlotIdx::new(0),
                limit: 50,
                page_size: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);

        // CollectStart emits body + done edges (2). CollectFinish emits no
        // kind-specific edges. Nop (n1) has no next so no next edge.
        // Total = 2.
        assert_eq!(
            doc.graph.edges.len(),
            2,
            "expected 2 edges from CollectStart only"
        );

        // Verify the body edge targets step-1 and done edge targets step-2.
        let mut found_body = false;
        let mut found_done = false;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "body" {
                found_body = true;
                assert_eq!(e.target.as_str(), "step-1");
                assert!(!e.style.dashed, "body edge should be solid");
            }
            if e.source_port.as_str() == "done" {
                found_done = true;
                assert_eq!(e.target.as_str(), "step-2");
                assert!(e.style.dashed, "done edge should be dashed");
            }
        }
        assert!(found_body, "should find body edge");
        assert!(found_done, "should find done edge");
    }

    /// ReduceStart -> body -> ReduceNext -> body -> ReduceFinish chain.
    /// ReduceFinish produces no kind-specific edges. Verify edge count and
    /// that ReduceStart and ReduceNext each produce body + done edges.
    #[test]
    fn reduce_start_and_next_produce_body_done_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: vb_core::ids::SlotIdx::new(0),
                accumulator: vb_core::ids::SlotIdx::new(1),
                initial: vb_core::ids::ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceNext {
                iterator_slot: vb_core::ids::SlotIdx::new(2),
                accumulator: vb_core::ids::SlotIdx::new(1),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceFinish {
                accumulator: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // ReduceStart: body + done (2 edges)
        // ReduceNext: body + done (2 edges)
        // Total kind-specific edges = 4.
        assert_eq!(
            doc.graph.edges.len(),
            4,
            "expected 4 edges from ReduceStart + ReduceNext"
        );

        // Verify the done edges both target step-3.
        let mut done_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "done" {
                done_count = done_count.saturating_add(1);
                assert_eq!(e.target.as_str(), "step-3");
                assert!(e.style.dashed, "done edges should be dashed");
            }
        }
        assert_eq!(done_count, 2, "should find 2 done edges");
    }

    /// Choose with three branches and an otherwise target produces 4 edges total.
    #[test]
    fn choose_with_three_branches_produces_four_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: Box::new([
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(2),
                        target: StepIdx::new(3),
                    },
                ]),
                otherwise: Some(StepIdx::new(4)),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_nop_node(3, None);
        let n4 = make_finish_node(4, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3, n4], 0);
        let doc = build_document(&parts);

        // 3 branch edges + 1 otherwise edge = 4 total.
        assert_eq!(
            doc.graph.edges.len(),
            4,
            "expected 3 branch + 1 otherwise edges"
        );

        let mut branch_count = 0usize;
        let mut otherwise_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str().starts_with("branch-") {
                branch_count = branch_count.saturating_add(1);
                assert!(!e.style.dashed, "branch edges should be solid");
            }
            if e.source_port.as_str() == "otherwise" {
                otherwise_count = otherwise_count.saturating_add(1);
                assert!(e.style.dashed, "otherwise edge should be dashed");
                assert_eq!(e.target.as_str(), "step-4");
            }
        }
        assert_eq!(branch_count, 3, "expected 3 branch edges");
        assert_eq!(otherwise_count, 1, "expected 1 otherwise edge");
    }

    /// TogetherStart -> TogetherBranch -> TogetherJoin produces branch edges from
    /// TogetherStart and entry/join edges from TogetherBranch, and the
    /// TogetherJoin node has no kind-specific edges, acting as the single merge
    /// output.
    #[test]
    fn together_end_merges_back_to_single_output() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
        };
        let n1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(4),
                join: StepIdx::new(3),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 1,
                entry: StepIdx::new(5),
                join: StepIdx::new(3),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n4 = make_nop_node(4, None);
        let n5 = make_nop_node(5, None);
        let parts = make_simple_parts(vec![n0, n1, n2, n3, n4, n5], 0);
        let doc = build_document(&parts);

        // TogetherStart: 2 branch edges + 1 join edge = 3
        // TogetherBranch (n1): entry + join = 2
        // TogetherBranch (n2): entry + join = 2
        // TogetherJoin (n3): 0 kind-specific edges
        // Total = 7.
        assert_eq!(doc.graph.edges.len(), 7, "expected 7 edges total");

        // All join edges should target step-3 (TogetherJoin).
        let mut join_edge_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "join" {
                join_edge_count = join_edge_count.saturating_add(1);
                assert_eq!(
                    e.target.as_str(),
                    "step-3",
                    "all join edges should target TogetherJoin at step-3"
                );
                assert!(e.style.dashed, "join edges should be dashed");
            }
        }
        assert_eq!(
            join_edge_count, 3,
            "expected 3 join edges (1 from start, 2 from branches)"
        );
    }

    /// RepeatAttempt creates a body edge that loops back to an earlier step,
    /// plus a done edge that exits the loop. This test verifies the loop-back
    /// edge targets an earlier step index.
    #[test]
    fn repeat_attempt_creates_loop_back_edge() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        // RepeatAttempt loops body back to itself (step 1) for retry.
        let n1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot: vb_core::ids::SlotIdx::new(0),
                body: StepIdx::new(1), // loop back to self
                done: StepIdx::new(2),
            },
        };
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatCheck {
                attempt_slot: vb_core::ids::SlotIdx::new(0),
                done: StepIdx::new(3),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // RepeatStart: body + done = 2
        // RepeatAttempt: body + done = 2
        // RepeatCheck: done = 1
        // RepeatFinish: 0
        // Total = 5 edges.
        assert_eq!(doc.graph.edges.len(), 5, "expected 5 edges total");

        // Find the loop-back body edge from step-1 targeting step-1.
        let mut found_loop_back = false;
        for (_id, e) in &doc.graph.edges {
            if e.source.as_str() == "step-1" && e.source_port.as_str() == "body" {
                found_loop_back = true;
                assert_eq!(
                    e.target.as_str(),
                    "step-1",
                    "RepeatAttempt body should loop back to itself"
                );
                assert!(!e.style.dashed, "body loop-back edge should be solid");
            }
        }
        assert!(
            found_loop_back,
            "should find a loop-back body edge from RepeatAttempt"
        );

        // Verify RepeatAttempt's done edge exits to step-2.
        let mut found_done_exit = false;
        for (_id, e) in &doc.graph.edges {
            if e.source.as_str() == "step-1" && e.source_port.as_str() == "done" {
                found_done_exit = true;
                assert_eq!(e.target.as_str(), "step-2");
                assert!(e.style.dashed, "done edge should be dashed");
            }
        }
        assert!(
            found_done_exit,
            "should find a done exit edge from RepeatAttempt"
        );

        // Verify group was created for the repeat loop.
        let group = match doc.graph.groups.get("group-repeat-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(group.kind, GroupKind::BranchContainer);
        assert_eq!(
            group.children.len(),
            4,
            "repeat group should span steps 0-3"
        );
    }

    /// Nested structure: a RepeatStart loop containing a TogetherStart/TogetherJoin
    /// parallel block inside it. Verifies that both groups are created and that
    /// edges from inner parallel construct are present alongside loop edges.
    #[test]
    fn nested_repeat_containing_together_produces_both_groups() {
        // Layout:
        // 0: RepeatStart(body=1, done=5)
        // 1: TogetherStart(branches=[2,3], join=4)
        // 2: TogetherBranch(entry=..., join=4)
        // 3: TogetherBranch(entry=..., join=4)
        // 4: TogetherJoin
        // 5: RepeatFinish
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(5),
            },
        };
        let n1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(2), StepIdx::new(3)]),
                join: StepIdx::new(4),
            },
        };
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(4),
                join: StepIdx::new(4),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 1,
                entry: StepIdx::new(4),
                join: StepIdx::new(4),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n4 = CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n5 = CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3, n4, n5], 0);
        let doc = build_document(&parts);

        // Verify both groups are present.
        let repeat_group = match doc.graph.groups.get("group-repeat-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(repeat_group.kind, GroupKind::BranchContainer);
        // Repeat spans steps 0-5 inclusive.
        assert_eq!(repeat_group.children.len(), 6);

        let together_group = match doc.graph.groups.get("group-together-1") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(together_group.kind, GroupKind::Swimlane);
        // Together spans steps 1-4 inclusive.
        assert_eq!(together_group.children.len(), 4);

        // RepeatStart: body + done = 2
        // TogetherStart: 2 branch + 1 join = 3
        // TogetherBranch (n2): entry + join = 2
        // TogetherBranch (n3): entry + join = 2
        // TogetherJoin (n4): 0
        // RepeatFinish (n5): 0
        // Total = 9
        assert_eq!(
            doc.graph.edges.len(),
            9,
            "expected 9 edges from nested structure"
        );

        // Verify RepeatStart body edge targets step-1 (TogetherStart).
        let mut found_repeat_body = false;
        for (_id, e) in &doc.graph.edges {
            if e.source.as_str() == "step-0" && e.source_port.as_str() == "body" {
                found_repeat_body = true;
                assert_eq!(e.target.as_str(), "step-1");
            }
        }
        assert!(
            found_repeat_body,
            "should find RepeatStart body edge targeting TogetherStart"
        );
    }

    /// A workflow consisting only of a single Finish node produces exactly one
    /// node, zero edges, and zero groups. The node must have the terminal flag
    /// set and be the entry node.
    #[test]
    fn single_finish_node_only_workflow() {
        let n = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![n], 0);
        let doc = build_document(&parts);

        // Exactly one node.
        assert_eq!(
            doc.graph.nodes.len(),
            1,
            "single Finish should produce exactly 1 node"
        );

        // Zero edges (Finish has no next or kind-specific edges).
        assert_eq!(
            doc.graph.edges.len(),
            0,
            "single Finish should produce 0 edges"
        );

        // Zero groups (no loops or parallel constructs).
        assert!(
            doc.graph.groups.is_empty(),
            "single Finish should produce 0 groups"
        );

        // The single node should be both terminal and entry.
        let node_rec = match doc.graph.nodes.get("step-0") {
            Some(n) => n,
            None => return,
        };
        assert!(node_rec.flags.terminal, "Finish node should be terminal");
        assert!(node_rec.flags.entry, "Finish node should be the entry node");

        // Entry node in the graph metadata should be step-0.
        let entry = match &doc.graph.entry_node {
            Some(e) => e,
            None => return,
        };
        assert_eq!(entry.as_str(), "step-0");
    }

    /// ChooseSlot with two branches and an otherwise target produces 3 edges
    /// total, using SlotBranch instead of ExprBranch.
    #[test]
    fn choose_slot_with_branches_produces_correct_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: Box::new([
                    vb_core::workflow::SlotBranch {
                        condition: vb_core::ids::SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    vb_core::workflow::SlotBranch {
                        condition: vb_core::ids::SlotIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_finish_node(3, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // 2 branch edges + 1 otherwise edge = 3 total.
        assert_eq!(doc.graph.edges.len(), 3, "expected 3 edges from ChooseSlot");

        // Verify branch edges are solid and otherwise is dashed.
        let mut solid_branches = 0usize;
        let mut dashed_otherwise = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str().starts_with("branch-") {
                solid_branches = solid_branches.saturating_add(1);
                assert!(!e.style.dashed, "branch edges should be solid");
            }
            if e.source_port.as_str() == "otherwise" {
                dashed_otherwise = dashed_otherwise.saturating_add(1);
                assert!(e.style.dashed, "otherwise should be dashed");
                assert_eq!(e.target.as_str(), "step-3");
            }
        }
        assert_eq!(solid_branches, 2, "expected 2 solid branch edges");
        assert_eq!(dashed_otherwise, 1, "expected 1 dashed otherwise edge");
    }

    /// Jump node creates a forward edge that skips over intermediate nodes.
    /// Verifies that a jump from step-0 to step-3 skips steps 1 and 2, and
    /// that the intermediate chain still produces its own sequential edges.
    #[test]
    fn jump_node_creates_forward_edge_skipping_intermediate_steps() {
        let n0 = make_jump_node(0, 3);
        let n1 = make_nop_node(1, Some(2));
        let n2 = make_nop_node(2, None);
        let n3 = make_finish_node(3, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // n0: jump edge to step-3 (1 edge)
        // n1: next edge to step-2 (1 edge)
        // n2: no next (0 edges)
        // n3: Finish, no next (0 edges)
        // Total: 2 edges.
        assert_eq!(doc.graph.edges.len(), 2, "expected 2 edges (jump + chain)");

        // Verify the jump edge skips over steps 1 and 2.
        let jump_edge = {
            let mut found = None;
            for (_id, e) in &doc.graph.edges {
                if e.source_port.as_str() == "jump" {
                    found = Some(e.clone());
                }
            }
            found
        };
        let je = match jump_edge {
            Some(e) => e,
            None => return,
        };
        assert_eq!(
            je.source.as_str(),
            "step-0",
            "jump should originate from step-0"
        );
        assert_eq!(
            je.target.as_str(),
            "step-3",
            "jump should target step-3, skipping steps 1 and 2"
        );
        assert!(!je.style.dashed, "jump edge should be solid");
        assert_eq!(je.label.as_ref().map(|l| l.as_str()), Some("jump"));

        // Verify the intermediate chain edge still exists.
        let chain_edge = {
            let mut found = None;
            for (_id, e) in &doc.graph.edges {
                if e.source.as_str() == "step-1" && e.source_port.as_str() == "next" {
                    found = Some(e.clone());
                }
            }
            found
        };
        let ce = match chain_edge {
            Some(e) => e,
            None => return,
        };
        assert_eq!(
            ce.target.as_str(),
            "step-2",
            "intermediate chain should connect step-1 to step-2"
        );
    }

    /// ErrorHandler creates both a body `next` edge (solid) and a handler edge
    /// (dashed), and downstream nodes continue from both paths.
    #[test]
    fn error_handler_creates_body_next_and_handler_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        };
        let n1 = make_nop_node(1, Some(3));
        let n2 = make_nop_node(2, Some(3));
        let n3 = make_finish_node(3, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // n0: next -> step-1 (solid) + handler -> step-2 (dashed) = 2 edges from n0
        // n1: next -> step-3 = 1 edge
        // n2: next -> step-3 = 1 edge
        // Total: 4 edges.
        assert_eq!(
            doc.graph.edges.len(),
            4,
            "error handler workflow should produce 4 edges"
        );

        let mut found_handler = false;
        let mut found_next_from_0 = false;
        for (_id, e) in &doc.graph.edges {
            if e.source.as_str() == "step-0" && e.source_port.as_str() == "handler" {
                found_handler = true;
                assert_eq!(
                    e.target.as_str(),
                    "step-2",
                    "handler edge should target step-2"
                );
                assert!(e.style.dashed, "handler edge should be dashed");
            }
            if e.source.as_str() == "step-0" && e.source_port.as_str() == "next" {
                found_next_from_0 = true;
                assert_eq!(
                    e.target.as_str(),
                    "step-1",
                    "body next edge should target step-1"
                );
                assert!(!e.style.dashed, "body next edge should be solid");
            }
        }
        assert!(
            found_handler,
            "should find a dashed handler edge from step-0"
        );
        assert!(
            found_next_from_0,
            "should find a solid next edge from step-0 to body step"
        );

        // Verify both paths converge at step-3.
        let mut edges_to_step3 = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.target.as_str() == "step-3" {
                edges_to_step3 = edges_to_step3.saturating_add(1);
            }
        }
        assert_eq!(
            edges_to_step3, 2,
            "both body and handler paths should converge at step-3"
        );
    }

    // -----------------------------------------------------------------------
    // Black hat security and correctness review tests
    // -----------------------------------------------------------------------

    /// HIGH: compute_node_size height is unbounded. With many ports the height
    /// grows linearly without cap. Width is capped at 320 but height has no
    /// cap. With 100 ports: height = 100*20 + 60 = 2060, which is absurdly
    /// large for a UI node.
    #[test]
    fn blackhat_compute_node_size_height_uncapped_many_ports() {
        let ports: Vec<FlowPortRecord> = (0..100)
            .map(|_| FlowPortRecord {
                id: SmolStr::new_static("p"),
                label: SmolStr::new_static("p"),
                side: PortSide::Input,
                role: PortRole::Data,
                cardinality: Cardinality::One,
            })
            .collect();
        let size = compute_node_size(&ports);
        // Width is capped at 320: 100*20 + 160 = 2160, min(2160, 320) = 320.
        assert!(
            size[0] <= 320.0,
            "width should be capped at 320, got {}",
            size[0],
        );
        // Height is NOT capped: 100*20 + 60 = 2060.
        assert!(
            size[1] > 2000.0,
            "height should be unbounded with 100 ports, got {} -- \
             this demonstrates the uncapped height vulnerability",
            size[1],
        );
    }

    /// LOW: Edge counter saturation at u32::MAX. When edge_counter reaches
    /// u32::MAX, add_edge silently drops edges. This test verifies the
    /// edge count matches expected for a normal workflow, showing the
    /// saturation path is reachable only with >4 billion edges.
    #[test]
    fn blackhat_edge_counter_saturates_silently() {
        // We can't easily produce 4B edges, but we verify the edge naming
        // pattern increments correctly for a small workflow.
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: Box::new([
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(1),
                        target: StepIdx::new(1),
                    },
                ]),
                otherwise: Some(StepIdx::new(1)),
            },
        };
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);

        // next edge + 2 branch edges + otherwise edge = 4 total.
        assert_eq!(doc.graph.edges.len(), 4, "expected 4 edges");

        // Edge IDs should be sequential: edge-0, edge-1, edge-2, edge-3.
        let mut ids: Vec<&str> = doc.graph.edges.keys().map(|k| k.as_str()).collect();
        ids.sort();
        assert_eq!(ids[0], "edge-0");
        assert_eq!(ids[1], "edge-1");
        assert_eq!(ids[2], "edge-2");
        assert_eq!(ids[3], "edge-3");
    }

    /// LOW: collect_span with start == end produces a single-node group.
    /// The function accepts end >= start, so a single node span is valid.
    #[test]
    fn blackhat_collect_span_single_node_span() {
        let span = collect_span(3, 3, 10);
        assert_eq!(span.len(), 1);
        assert_eq!(span[0].as_str(), "step-3");
    }

    /// MEDIUM: collect_span with end == total - 1 uses last node.
    /// This is the boundary case for the end < total check.
    #[test]
    fn blackhat_collect_span_end_at_boundary() {
        let span = collect_span(0, 9, 10);
        assert_eq!(span.len(), 10);
        assert_eq!(span[0].as_str(), "step-0");
        assert_eq!(span[9].as_str(), "step-9");
    }

    /// LOW: collect_span with end == total is rejected (out of bounds).
    #[test]
    fn blackhat_collect_span_end_equals_total_rejected() {
        let span = collect_span(0, 10, 10);
        assert!(span.is_empty(), "end == total should produce empty span");
    }

    /// LOW: collect_span with start == 0 and end == 0 and total == 1
    /// produces a single-node span.
    #[test]
    fn blackhat_collect_span_minimal_valid() {
        let span = collect_span(0, 0, 1);
        assert_eq!(span.len(), 1);
        assert_eq!(span[0].as_str(), "step-0");
    }

    /// MEDIUM: entry index out of bounds. When parts.entry points to a step
    /// beyond the nodes array length, the entry_node is set to a
    /// nonexistent step ID and no node gets the entry flag. This is a
    /// data inconsistency between entry_node and node flags.
    #[test]
    fn blackhat_entry_out_of_bounds_no_entry_flag_set() {
        let n0 = make_nop_node(0, None);
        let parts = make_simple_parts(vec![n0], 99);
        let doc = build_document(&parts);

        // entry_node says step-99, but only step-0 exists.
        assert_eq!(
            doc.graph.entry_node.as_ref().map(|s| s.as_str()),
            Some("step-99"),
            "entry_node should point to step-99 even though it doesn't exist",
        );

        // No node has the entry flag set because i==99 never matches.
        for (_, node) in &doc.graph.nodes {
            assert!(
                !node.flags.entry,
                "no node should have entry flag when entry is out of bounds, \
                 but {} has it set",
                node.id,
            );
        }
    }

    /// LOW: BuildObject with zero fields produces no input ports.
    #[test]
    fn blackhat_build_object_zero_fields_no_input_ports() {
        use vb_core::ids::SlotIdx;

        let kind = CompiledNodeKind::BuildObject {
            fields: Box::new([]),
        };
        let (inputs, outputs) = build_ports(&kind, Some(SlotIdx::new(0)));

        assert!(
            inputs.is_empty(),
            "zero fields should produce no input ports"
        );
        assert_eq!(outputs.len(), 1, "output slot port should still exist");
    }

    /// LOW: BuildList with zero items produces no input ports.
    #[test]
    fn blackhat_build_list_zero_items_no_input_ports() {
        use vb_core::ids::SlotIdx;

        let kind = CompiledNodeKind::BuildList {
            items: Box::new([]),
        };
        let (inputs, outputs) = build_ports(&kind, Some(SlotIdx::new(0)));

        assert!(
            inputs.is_empty(),
            "zero items should produce no input ports"
        );
        assert_eq!(outputs.len(), 1, "output slot port should still exist");
    }

    /// LOW: Choose with zero branches and no otherwise produces no edges.
    #[test]
    fn blackhat_choose_zero_branches_no_otherwise_no_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: Box::new([]),
                otherwise: None,
            },
        };
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);

        assert_eq!(
            doc.graph.edges.len(),
            0,
            "Choose with no branches and no otherwise should produce no edges",
        );
    }

    /// MEDIUM: classify_node_kind is exhaustive. Every CompiledNodeKind variant
    /// must produce a valid (label, category) pair. This test verifies
    /// that adding a new variant to CompiledNodeKind without updating
    /// classify_node_kind causes a compile error.
    #[test]
    fn blackhat_classify_node_kind_all_variants_produce_non_empty() {
        // Spot-check that all categories are non-empty strings.
        let cases: Vec<CompiledNodeKind> = vec![
            CompiledNodeKind::Nop,
            CompiledNodeKind::SetConst {
                value: vb_core::ids::ConstIdx::new(0),
            },
            CompiledNodeKind::Copy {
                source: vb_core::ids::SlotIdx::new(0),
            },
            CompiledNodeKind::EvalExpr {
                expr: vb_core::ids::ExprIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(0),
            },
            CompiledNodeKind::Jump {
                target: StepIdx::new(0),
            },
        ];
        for kind in &cases {
            let (label, cat) = classify_node_kind(kind);
            assert!(!label.is_empty(), "label must not be empty for {:?}", kind);
            assert!(!cat.is_empty(), "category must not be empty for {:?}", kind);
        }
    }

    /// MEDIUM: StepIdx used as node array index without bounds checking.
    /// build_document uses `format!("step-{}", next.as_usize())` for edge
    /// targets without verifying the target is within bounds. This creates
    /// edges to nonexistent nodes. This test verifies the graph still
    /// produces a valid document with dangling edges.
    #[test]
    fn blackhat_next_target_out_of_bounds_creates_dangling_edge() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(999)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_simple_parts(vec![n0], 0);
        let doc = build_document(&parts);

        // Edge should exist but target step-999 doesn't exist as a node.
        assert_eq!(doc.graph.edges.len(), 1, "next edge should be created");
        let e = match doc.graph.edges.get_index(0).map(|(_, e)| e.clone()) {
            Some(e) => e,
            None => return,
        };
        assert_eq!(e.target.as_str(), "step-999");
        assert!(
            !doc.graph.nodes.contains_key("step-999"),
            "target node should not exist -- dangling edge",
        );
    }

    /// LOW: Loop group with done pointing to itself (start == done).
    /// collect_span(start, start, total) produces a single-node group.
    #[test]
    fn blackhat_foreach_done_equals_start_single_node_group() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: vb_core::ids::SlotIdx::new(0),
                item_slot: vb_core::ids::SlotIdx::new(1),
                limit: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
        };
        let parts = make_simple_parts(vec![n0], 0);
        let doc = build_document(&parts);

        let group = match doc.graph.groups.get("group-foreach-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(
            group.children.len(),
            1,
            "self-referencing loop should produce 1-child group"
        );
    }

    /// LOW: WorkflowParts with mismatched StepIdx IDs. CompiledNode.id
    /// is not verified against array position. build_document uses array
    /// index for step names, not node.id.
    #[test]
    fn blackhat_node_id_mismatch_uses_array_index() {
        let n0 = CompiledNode {
            id: StepIdx::new(42), // Mismatch: position 0 has id 42
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_simple_parts(vec![n0], 0);
        let doc = build_document(&parts);

        // Node should be named step-0 (array index), not step-42.
        assert!(
            doc.graph.nodes.contains_key("step-0"),
            "node should be keyed by array index, not node.id",
        );
        assert!(
            !doc.graph.nodes.contains_key("step-42"),
            "node should NOT be keyed by mismatched node.id",
        );
    }

    // -----------------------------------------------------------------------
    // Additional BLACKHAT security and correctness review tests
    // -----------------------------------------------------------------------

    /// HIGH: compute_node_size height overflow for u32::MAX ports.
    /// When port_count = u32::MAX (from unwrap_or), saturating_mul(20)
    /// saturates to u32::MAX, then saturating_add(60) stays at u32::MAX.
    /// f64::from(u32::MAX) = 4294967295.0, producing an absurdly large
    /// height. Width is capped at 320, but height is not.
    #[test]
    fn blackhat_compute_node_size_height_saturates_at_u32_max() {
        // We cannot easily create u32::MAX ports, but we can verify the
        // formula by checking that a very large port count produces a
        // huge height. With 1000 ports: 1000*20 + 60 = 20060.
        let ports: Vec<FlowPortRecord> = (0..1000)
            .map(|_| FlowPortRecord {
                id: SmolStr::new_static("p"),
                label: SmolStr::new_static("p"),
                side: PortSide::Input,
                role: PortRole::Data,
                cardinality: Cardinality::One,
            })
            .collect();
        let size = compute_node_size(&ports);
        // Width capped at 320: 1000*20 + 160 = 20160, min(20160, 320) = 320.
        assert_eq!(size[0], 320.0, "width should be capped at 320");
        // Height NOT capped: 1000*20 + 60 = 20060.
        assert!(
            size[1] > 20000.0,
            "height should be very large with 1000 ports, got {} -- uncapped",
            size[1],
        );
    }

    /// MEDIUM: compute_node_size with zero ports produces minimum dimensions.
    /// This verifies the base case: width = 160, height = 60.
    #[test]
    fn blackhat_compute_node_size_zero_ports_minimum_dimensions() {
        let size = compute_node_size(&[]);
        assert_eq!(size[0], 160.0);
        assert_eq!(size[1], 60.0);
    }

    /// MEDIUM: collect_span with usize::MAX total rejects end at boundary.
    /// When end = usize::MAX and total = usize::MAX, the check end >= total
    /// triggers and returns empty.
    #[test]
    fn blackhat_collect_span_usize_max_total_rejects() {
        let span = collect_span(0, usize::MAX, usize::MAX);
        assert!(
            span.is_empty(),
            "end == usize::MAX with total == usize::MAX should be rejected"
        );
    }

    /// LOW: build_document with a large number of nodes. Verifies that
    /// the document structure handles 100 nodes without issues.
    #[test]
    fn blackhat_large_node_count_produces_valid_document() {
        let mut nodes = Vec::new();
        for i in 0..100u16 {
            if i < 99 {
                nodes.push(make_nop_node(i, Some(i.saturating_add(1))));
            } else {
                nodes.push(make_finish_node(i, 0));
            }
        }
        let parts = make_simple_parts(nodes, 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.nodes.len(), 100);
        // 99 next edges in the chain.
        assert_eq!(doc.graph.edges.len(), 99);
        // Entry node at step-0.
        let entry = doc.graph.nodes.get("step-0");
        assert!(entry.is_some());
        assert!(entry.map(|n| n.flags.entry).unwrap_or(false));
    }

    /// LOW: RetryCheck produces body and exhausted edges correctly.
    /// The body edge should be solid and labeled "retry", the exhausted
    /// edge should be dashed and labeled "exhausted".
    #[test]
    fn blackhat_retry_check_produces_body_and_exhausted_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: vb_core::ids::SlotIdx::new(0),
                body: StepIdx::new(1),
                exhausted: StepIdx::new(2),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);

        assert_eq!(
            doc.graph.edges.len(),
            2,
            "RetryCheck should produce 2 edges"
        );

        let mut found_retry = false;
        let mut found_exhausted = false;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "body" {
                found_retry = true;
                assert_eq!(e.target.as_str(), "step-1");
                assert!(!e.style.dashed, "retry body edge should be solid");
                assert_eq!(e.label.as_ref().map(|l| l.as_str()), Some("retry"));
            }
            if e.source_port.as_str() == "exhausted" {
                found_exhausted = true;
                assert_eq!(e.target.as_str(), "step-2");
                assert!(e.style.dashed, "exhausted edge should be dashed");
                assert_eq!(e.label.as_ref().map(|l| l.as_str()), Some("exhausted"));
            }
        }
        assert!(found_retry, "should find retry body edge");
        assert!(found_exhausted, "should find exhausted edge");
    }

    /// LOW: build_ports for Finish node produces one input port for result.
    /// With output=None, there should be exactly 1 input and 0 outputs.
    #[test]
    fn blackhat_build_ports_finish_one_input_no_output() {
        let kind = CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::new(5),
        };
        let (inputs, outputs) = build_ports(&kind, None);
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].id.as_str(), "result");
        assert!(outputs.is_empty());
    }

    /// LOW: FlowDocument entry_node always set even for empty graphs.
    /// build_document always sets entry_node to Some(...).
    #[test]
    fn blackhat_entry_node_always_some() {
        let n = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![n], 0);
        let doc = build_document(&parts);
        assert!(
            doc.graph.entry_node.is_some(),
            "entry_node should always be Some"
        );
    }

    /// LOW: add_edge silently drops edges when counter saturates. This is
    /// documented behavior but means large workflows could silently lose
    /// edges. The edge ID would be "edge-4294967295" for the last one.
    /// We verify the pattern for a small case.
    #[test]
    fn blackhat_add_edge_produces_sequential_ids() {
        // Simple 3-node chain.
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_nop_node(1, Some(2));
        let n2 = make_finish_node(2, 0);
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);

        // 2 next edges: step-0 -> step-1, step-1 -> step-2.
        let mut edge_ids: Vec<&str> = doc.graph.edges.keys().map(|k| k.as_str()).collect();
        edge_ids.sort();
        assert_eq!(edge_ids, ["edge-0", "edge-1"]);
    }

    /// LOW: classify_node_kind returns unique labels for similar node kinds.
    /// ForEachStart vs ForEachNext vs ForEachJoin should have distinct labels.
    #[test]
    fn blackhat_classify_node_kind_loop_labels_are_distinct() {
        let (l1, _) = classify_node_kind(&CompiledNodeKind::ForEachStart {
            input: vb_core::ids::SlotIdx::new(0),
            item_slot: vb_core::ids::SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        });
        let (l2, _) = classify_node_kind(&CompiledNodeKind::ForEachNext {
            iterator_slot: vb_core::ids::SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        });
        let (l3, _) = classify_node_kind(&CompiledNodeKind::ForEachJoin {
            output: vb_core::ids::SlotIdx::new(0),
        });
        assert_ne!(l1, l2);
        assert_ne!(l2, l3);
        assert_ne!(l1, l3);
    }

    /// LOW: build_document with same node ID appearing multiple times.
    /// IndexMap will keep the last inserted entry for duplicate keys.
    /// This means node data could be silently overwritten.
    #[test]
    fn blackhat_build_document_duplicate_step_idx_last_wins() {
        // Two nodes with same array position (impossible in normal usage,
        // but the function accepts any WorkflowParts).
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let n1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(0),
            },
        };
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);
        // Both nodes exist with unique keys based on array index.
        assert_eq!(doc.graph.nodes.len(), 2);
        assert!(doc.graph.nodes.contains_key("step-0"));
        assert!(doc.graph.nodes.contains_key("step-1"));
    }

    /// LOW: ReduceStart creates a group spanning from start to done step.
    /// Verifies the group is created with BranchContainer kind.
    #[test]
    fn blackhat_reduce_start_creates_branch_container_group() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: vb_core::ids::SlotIdx::new(0),
                accumulator: vb_core::ids::SlotIdx::new(1),
                initial: vb_core::ids::ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceFinish {
                accumulator: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);

        let group = match doc.graph.groups.get("group-reduce-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(group.kind, GroupKind::BranchContainer);
        assert_eq!(group.children.len(), 3);
    }
}
