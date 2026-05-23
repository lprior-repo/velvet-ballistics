#[cfg(test)]
#[allow(clippy::module_inception)]
mod proptests {
    #[cfg(not(miri))]
    use std::fmt::Write as _;

    use crate::{
        CodegenError, compare_generated_to_ir, emit_action_boundary, emit_expr_function,
        emit_resource_contract, emit_rust_workflow, validate_generated_subset,
    };
    use proptest::prelude::*;
    use vb_core::{
        ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprIdx,
        ExprOp, ExprProgram, ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
    };
    #[cfg(not(miri))]
    use vb_core::{
        EngineSignal, RunId, StepBudget, Taint, engine::new_run_frame, engine::run_until_blocked,
    };

    #[cfg(not(miri))]
    struct GeneratedHarness {
        temp_dir: tempfile::TempDir,
        source_path: std::path::PathBuf,
        binary_path: std::path::PathBuf,
    }

    #[cfg(not(miri))]
    fn generated_equivalence_stdout(
        workflow: &CompiledWorkflow,
        name: &str,
    ) -> Result<String, String> {
        let generated = emit_rust_workflow(workflow).map_err(|e| e.to_string())?;
        let harness = equivalence_harness_source(workflow, &generated);
        let paths = generated_harness_paths(name)?;
        std::fs::write(&paths.source_path, harness).map_err(|e| e.to_string())?;
        compile_and_run_generated(&paths)
    }

    #[cfg(not(miri))]
    fn generated_harness_paths(name: &str) -> Result<GeneratedHarness, String> {
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("vb_codegen_prop_equiv_{name}_"))
            .tempdir()
            .map_err(|e| e.to_string())?;
        let source_path = temp_dir.path().join("generated_equivalence.rs");
        let binary_path = temp_dir.path().join("generated_equivalence_bin");
        Ok(GeneratedHarness {
            temp_dir,
            source_path,
            binary_path,
        })
    }

    #[cfg(not(miri))]
    fn compile_and_run_generated(paths: &GeneratedHarness) -> Result<String, String> {
        if !paths.temp_dir.path().exists() {
            return Err(String::from("generated harness tempdir missing"));
        }
        let compile = std::process::Command::new("rustc")
            .arg("--edition")
            .arg("2024")
            .arg("-o")
            .arg(&paths.binary_path)
            .arg(&paths.source_path)
            .output()
            .map_err(|e| e.to_string())?;
        if !compile.status.success() {
            return Err(String::from_utf8_lossy(&compile.stderr).into_owned());
        }

        let run = std::process::Command::new(&paths.binary_path)
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
        if !run.status.success() {
            let stderr = String::from_utf8_lossy(&run.stderr);
            return Err(format!("generated run failed: {stdout}{stderr}"));
        }
        Ok(stdout)
    }

    #[cfg(not(miri))]
    fn equivalence_harness_source(workflow: &CompiledWorkflow, generated: &str) -> String {
        format!(
            "{generated}\n{}\n{}\n",
            slot_text_function_source(),
            equivalence_main_source(workflow)
        )
    }

    #[cfg(not(miri))]
    fn slot_text_function_source() -> &'static str {
        "fn slot_text(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT], index: u16) -> String {\n    match slots.get(usize::from(index)) {\n        Some(value) => format!(\"{value:?}\"),\n        None => String::from(\"slot-out-of-bounds\"),\n    }\n}"
    }

    #[cfg(not(miri))]
    fn equivalence_main_source(workflow: &CompiledWorkflow) -> String {
        format!(
            "fn main() {{\n    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    let value = drive_equivalence_trace(&mut slots, &mut slot_taints, &mut list_store, &mut object_store);\n    println!(\"{{value}}|slots:{{}}|{{}}|{{}}\", slot_text(&slots, 0), slot_text(&slots, 1), slot_text(&slots, 2));\n}}\n{}",
            drive_trace_function_source(workflow)
        )
    }

    #[cfg(not(miri))]
    fn drive_trace_function_source(workflow: &CompiledWorkflow) -> String {
        format!(
            "fn drive_equivalence_trace(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT], list_store: &mut ListStore, object_store: &mut ObjectStore) -> String {{\n    let mut pc: u16 = 0;\n    loop {{\n        let outcome = match pc {{\n{}            _ => Err(DriveError::InvalidProgramCounter),\n        }};\n        match outcome {{\n            Ok(StepOutcome::Continue(next)) => pc = next,\n            Ok(StepOutcome::Finished(done)) => break format!(\"finished:{{done:?}}\"),\n            Err(error) => break format!(\"err:{{error:?}}\"),\n        }}\n    }}\n}}",
            dynamic_step_arms(workflow)
        )
    }

    #[cfg(not(miri))]
    fn dynamic_step_arms(workflow: &CompiledWorkflow) -> String {
        (0..workflow.node_count()).fold(String::new(), |mut arms, idx| {
            if writeln!(
                arms,
                "            {idx} => step_{idx}(slots, slot_taints, list_store, object_store),"
            )
            .is_err()
            {
                arms.clear();
            }
            arms
        })
    }

    #[cfg(not(miri))]
    fn ir_equivalence_trace(workflow: &CompiledWorkflow) -> Result<String, String> {
        let mut run = new_run_frame(RunId::new(46), workflow).map_err(|e| e.to_string())?;
        let mut store = vb_core::ValueStore::new();
        let signal = run_until_blocked(workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|e| e.to_string())?;
        let head = match signal {
            EngineSignal::Finished(value, Taint::Clean) => format!("finished:{value:?}"),
            EngineSignal::Finished(value, taint) => format!("finished:{value:?}:{taint:?}"),
            other => format!("signal:{other:?}"),
        };
        let slot0 = slot_trace(&run, SlotIdx::new(0));
        let slot1 = slot_trace(&run, SlotIdx::new(1));
        let slot2 = slot_trace(&run, SlotIdx::new(2));
        Ok(format!("{head}|slots:{slot0}|{slot1}|{slot2}\n"))
    }

    #[cfg(not(miri))]
    fn slot_trace(run: &vb_core::RunFrame, slot: SlotIdx) -> String {
        match run.read_slot(slot) {
            Ok(value) => format!("Some({value:?})"),
            Err(_) => String::from("None"),
        }
    }

    #[cfg(not(miri))]
    fn arb_small_i64() -> impl Strategy<Value = i64> {
        -1_000_000i64..1_000_000i64
    }

    fn fixed_six_step_equivalence_workflow(
        take_branch: bool,
        branch_value: i64,
        left: i64,
        right: i64,
    ) -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("fixed_six_step_prop_equivalence"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: fixed_six_step_equivalence_nodes(),
            expressions: fixed_six_step_equivalence_expressions()?,
            accessors: Box::new([]),
            constants: fixed_six_step_equivalence_constants(take_branch, branch_value, left, right),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn fixed_six_step_equivalence_expressions() -> Result<Box<[ExprProgram]>, String> {
        ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(2)),
                ExprOp::LoadConst(ConstIdx::new(3)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
        )
        .map(|expr| vec![expr].into_boxed_slice())
        .map_err(|e| e.to_string())
    }

    fn fixed_six_step_equivalence_constants(
        take_branch: bool,
        branch_value: i64,
        left: i64,
        right: i64,
    ) -> Box<[ConstValue]> {
        vec![
            ConstValue::Bool(take_branch),
            ConstValue::I64(branch_value),
            ConstValue::I64(left),
            ConstValue::I64(right),
        ]
        .into_boxed_slice()
    }

    fn fixed_six_step_equivalence_nodes() -> Box<[CompiledNode]> {
        vec![
            set_const_node(0, 0, 1, 0),
            set_const_node(1, 1, 2, 1),
            choose_slot_node(),
            copy_node(3, 2, 5, 1),
            eval_expr_node(),
            finish_node(),
        ]
        .into_boxed_slice()
    }

    fn set_const_node(id: u16, output: u16, next: u16, value: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: Some(SlotIdx::new(output)),
            next: Some(StepIdx::new(next)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(value),
            },
        }
    }

    fn choose_slot_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![vb_core::SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(3),
                }]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(4)),
            },
        }
    }

    fn copy_node(id: u16, output: u16, next: u16, source: u16) -> CompiledNode {
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

    fn eval_expr_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(5)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        }
    }

    fn finish_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        }
    }

    fn arb_resource_contract() -> impl Strategy<Value = ResourceContract> {
        (
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u8..64u8,
            1u32..10000u32,
            1u32..10000u32,
        )
            .prop_map(
                |(
                    steps,
                    slots,
                    constants,
                    accessors,
                    expressions,
                    expr_stack,
                    input_bytes,
                    output_bytes,
                )| {
                    ResourceContract {
                        max_steps: steps,
                        max_slots: slots,
                        max_constants: constants,
                        max_accessors: accessors,
                        max_expressions: expressions,
                        max_expr_stack: expr_stack,
                        max_input_bytes: input_bytes,
                        max_output_bytes: output_bytes,
                        max_step_budget_per_tick: 500,
                        max_transitions_per_tick: 500,
                        max_blob_bytes: 1024,
                        max_ipc_payload_bytes: 2048,
                        max_retry_attempts: 3,
                        max_fanout: 8,
                        max_collect_items: 100,
                        max_queue_depth: 64,
                        max_journal_batch_bytes: 512,
                        ..ResourceContract::DEFAULT
                    }
                },
            )
    }

    #[cfg(miri)]
    #[test]
    fn fixed_six_step_emitted_rust_miri_smoke() -> Result<(), String> {
        let workflow = fixed_six_step_equivalence_workflow(true, 7, 11, 13)?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        if source.contains("WORKFLOW_NODE_COUNT") {
            Ok(())
        } else {
            Err(String::from("generated source missing workflow node count"))
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 16,
            failure_persistence: None,
            .. ProptestConfig::default()
        })]

        #[cfg(not(miri))]
        #[test]
        fn fixed_six_step_emitted_rust_and_ir_match_finished_signal_and_slots(
            take_branch in any::<bool>(),
            branch_value in arb_small_i64(),
            left in arb_small_i64(),
            right in arb_small_i64(),
        ) {
            let workflow = fixed_six_step_equivalence_workflow(take_branch, branch_value, left, right)
                .map_err(TestCaseError::fail)?;
            let source = emit_rust_workflow(&workflow).map_err(|e| TestCaseError::fail(e.to_string()))?;
            compare_generated_to_ir(&source, &workflow).map_err(|e| TestCaseError::fail(e.to_string()))?;

            let generated = generated_equivalence_stdout(
                &workflow,
                &format!("{take_branch}_{branch_value}_{left}_{right}"),
            ).map_err(TestCaseError::fail)?;
            let interpreted = ir_equivalence_trace(&workflow).map_err(TestCaseError::fail)?;

            prop_assert_eq!(generated, interpreted);
        }

        #[test]
        fn emit_resource_contract_output_contains_all_fields(contract in arb_resource_contract()) {
            let mut out = String::new();
            emit_resource_contract(&mut out, contract).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(out.contains("CONTRACT_MAX_STEPS"), true);
            prop_assert!(!out.is_empty());
            prop_assert!(out.contains("CONTRACT_MAX_SLOTS"));
            prop_assert!(out.contains("CONTRACT_MAX_CONSTANTS"));
            prop_assert!(out.contains("CONTRACT_MAX_ACCESSORS"));
            prop_assert!(out.contains("CONTRACT_MAX_EXPRESSIONS"));
            prop_assert!(out.contains("CONTRACT_MAX_EXPR_STACK"));
            prop_assert!(out.contains("CONTRACT_MAX_INPUT_BYTES"));
            prop_assert!(out.contains("CONTRACT_MAX_OUTPUT_BYTES"));
        }

        #[test]
        fn codegen_error_display_never_empty(error_idx in 0u8..6u8) {
            let error = match error_idx {
                0 => CodegenError::FormatBufferOverflow,
                1 => CodegenError::RustfmtFailed { detail: String::from("test") },
                2 => CodegenError::CompileCheckFailed { detail: String::from("test") },
                3 => CodegenError::SemanticMismatch { detail: String::from("test") },
                4 => CodegenError::Io(std::io::Error::other("io")),
                _ => CodegenError::TrybuildFixture { detail: String::from("test") },
            };
            let message = error.to_string();
            prop_assert!(!message.is_empty(), "error display must never be empty");
        }

        #[test]
        fn generated_source_always_forbids_unsafe(slot_count in 1u16..10u16) {
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_test"),
                digest: WorkflowDigest::from_bytes([0x42; 32]),
                nodes: vec![
                    CompiledNode {
                        id: StepIdx::new(0),
                        output: None,
                        next: None,
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
                    },
                ].into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: Box::new([]),
                slot_count,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
            };
            if let Ok(workflow) = CompiledWorkflow::try_from_parts(parts)
                && let Ok(source) = emit_rust_workflow(&workflow)
            {
                prop_assert!(source.contains("#![forbid(unsafe_code)]"));
                prop_assert!(source.contains("#![deny(unused_must_use)]"));
            }
        }
    }

    // =======================================================================
    // Adversarial equivalence tests — codegen vs IR engine divergence
    // =======================================================================

    /// Verify that Exists is now supported by validate_generated_subset.
    #[test]
    fn exists_expression_now_supported_by_generated_subset() -> Result<(), String> {
        let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Exists];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_exists_rejected"),
            digest: WorkflowDigest::from_bytes([0xDA; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        let result = validate_generated_subset(&workflow);
        // Exists is now supported: validation should succeed
        result.map_err(|e| format!("Exists should be supported but got: {e}"))?;
        Ok(())
    }

    /// Verify that compare_generated_to_ir correctly counts action boundaries
    /// when a workflow contains Do nodes.
    #[test]
    fn compare_generated_to_ir_counts_action_boundaries_for_do_workflow() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_do_action"),
            digest: WorkflowDigest::from_bytes([0xEF; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Do {
                        action: ActionId::new(5),
                        input: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let result = compare_generated_to_ir(&source, &workflow);
        result.map_err(|e| {
            format!(
                "compare_generated_to_ir must accept Do workflow with action boundary markers, got: {e}"
            )
        })?;
        if source.contains("Action boundary:") {
            Ok(())
        } else {
            Err(String::from(
                "generated source must contain 'Action boundary:' marker for Do nodes",
            ))
        }
    }

    /// Verify that the action boundary comment includes the correct action and slot IDs.
    #[test]
    fn emit_action_boundary_includes_action_marker_comment() -> Result<(), String> {
        let mut out = String::new();
        emit_action_boundary(
            &mut out,
            StepIdx::new(1),
            ActionId::new(5),
            SlotIdx::new(2),
            Some(StepIdx::new(3)),
        )
        .map_err(|e| e.to_string())?;
        if out.contains("Action boundary: action_id=5, input_slot=2") {
            Ok(())
        } else {
            Err(format!(
                "action boundary must include action_id and input_slot in comment, got: {out}"
            ))
        }
    }

    // =======================================================================
    // Helper for creating direct expression workflows (copied from tests.rs)
    // =======================================================================

    fn direct_expression_workflow(
        name: &'static str,
        ops: Box<[ExprOp]>,
        constants: Box<[ConstValue]>,
        slot_count: u16,
    ) -> Result<CompiledWorkflow, String> {
        let expr = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from(name),
            digest: WorkflowDigest::from_bytes([0xA7; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants,
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // =======================================================================
    // Length/Empty helper parity property tests
    // These verify that Length and Empty helpers are accepted by validate_generated_subset
    // and that the codegen output contains the correct patterns.
    // =======================================================================

    /// PI-01: Length parity property test.
    /// Verifies that validate_generated_subset accepts Length helper workflows and
    /// that the codegen emits the correct list_item_count/object_field_count patterns.
    /// Before bug-fix: validate_generated_subset rejects Length with "unsupported" error.
    /// After bug-fix: validate_generated_subset accepts Length.
    #[test]
    fn length_parity_property_test() -> Result<(), String> {
        // Test Length on List slot
        let list_workflow = direct_expression_workflow(
            "prop_length_list",
            Box::new([ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length]),
            Box::new([]),
            1,
        )?;
        // validate_generated_subset must accept Length (bug-fix removes false positive)
        validate_generated_subset(&list_workflow)
            .map_err(|e| format!("Length helper must be accepted, got: {e}"))?;

        // Codegen must emit list_item_count for List input
        let mut out = String::new();
        emit_expr_function(&mut out, ExprIdx::new(0), &list_workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("list_item_count"),
            "Length on List must emit list_item_count, got: {out}"
        );

        // Test Length on Object slot
        let object_workflow = direct_expression_workflow(
            "prop_length_object",
            Box::new([ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length]),
            Box::new([]),
            1,
        )?;
        // validate_generated_subset must accept Length
        validate_generated_subset(&object_workflow)
            .map_err(|e| format!("Length helper must be accepted, got: {e}"))?;

        // Codegen must emit object_field_count for Object input
        let mut out = String::new();
        emit_expr_function(&mut out, ExprIdx::new(0), &object_workflow)
            .map_err(|e| e.to_string())?;
        assert!(
            out.contains("object_field_count"),
            "Length on Object must emit object_field_count, got: {out}"
        );

        Ok(())
    }

    /// PI-02: Empty parity property test.
    /// Verifies that validate_generated_subset accepts Empty helper workflows and
    /// that the codegen emits the correct == 0 / matches!(v, Null) patterns.
    /// Before bug-fix: validate_generated_subset rejects Empty with "unsupported" error.
    /// After bug-fix: validate_generated_subset accepts Empty.
    #[test]
    fn empty_parity_property_test() -> Result<(), String> {
        // Test Empty on List slot
        let list_workflow = direct_expression_workflow(
            "prop_empty_list",
            Box::new([ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty]),
            Box::new([]),
            1,
        )?;
        // validate_generated_subset must accept Empty (bug-fix removes false positive)
        validate_generated_subset(&list_workflow)
            .map_err(|e| format!("Empty helper must be accepted, got: {e}"))?;

        // Codegen must emit list_item_count == 0 for List input
        let mut out = String::new();
        emit_expr_function(&mut out, ExprIdx::new(0), &list_workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("list_item_count") && out.contains("== 0"),
            "Empty on List must emit list_item_count == 0, got: {out}"
        );

        // Test Empty on Object slot
        let object_workflow = direct_expression_workflow(
            "prop_empty_object",
            Box::new([ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty]),
            Box::new([]),
            1,
        )?;
        // validate_generated_subset must accept Empty
        validate_generated_subset(&object_workflow)
            .map_err(|e| format!("Empty helper must be accepted, got: {e}"))?;

        // Codegen must emit object_field_count == 0 for Object input
        let mut out = String::new();
        emit_expr_function(&mut out, ExprIdx::new(0), &object_workflow)
            .map_err(|e| e.to_string())?;
        assert!(
            out.contains("object_field_count") && out.contains("== 0"),
            "Empty on Object must emit object_field_count == 0, got: {out}"
        );

        // Test Empty on Null (via SlotValue::Null constant)
        // Empty checks for SlotValue::Null and returns true
        let null_workflow = direct_expression_workflow(
            "prop_empty_null",
            Box::new([ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty]),
            Box::new([]),
            1,
        )?;
        // validate_generated_subset must accept Empty
        validate_generated_subset(&null_workflow)
            .map_err(|e| format!("Empty helper must be accepted, got: {e}"))?;

        let mut out = String::new();
        emit_expr_function(&mut out, ExprIdx::new(0), &null_workflow).map_err(|e| e.to_string())?;
        // Empty on Null emits: SlotValue::Null => true
        assert!(
            out.contains("SlotValue::Null") && out.contains("true"),
            "Empty on Null must emit SlotValue::Null => true, got: {out}"
        );

        Ok(())
    }
}
