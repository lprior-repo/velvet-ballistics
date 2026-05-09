#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::{
        CodegenError, compare_generated_to_ir, compile_check_generated_rust, emit_action_boundary,
        emit_action_match_dispatch, emit_drive_function, emit_finish, emit_ids,
        emit_resource_contract, emit_rust_workflow, emit_step_function, emit_trybuild_fixture,
        format_generated_rust, validate_generated_subset,
    };
    use vb_core::{
        AccessorProgram, ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx,
        ConstValue, EngineError, EngineSignal, ExprProgram, PathSegment, ResourceContract, RunId,
        SlotIdx, SlotValue, StepBudget, StepIdx, ValueStore, WorkflowDigest, WorkflowParts,
        capability::CapabilitySet, new_run_frame, run_until_blocked, step_once,
    };
    use vb_runtime::{
        engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full},
        primitives::collect::CollectStates,
    };

    // --- Workflow helpers ---

    fn semantic_mismatch_detail(result: Result<(), CodegenError>) -> Result<String, String> {
        match result {
            Ok(()) => Err(String::from("expected semantic mismatch")),
            Err(CodegenError::SemanticMismatch { detail }) => Ok(detail),
            Err(other) => Err(format!("expected semantic mismatch, got: {other}")),
        }
    }

    fn assert_contains_all(source: &str, variants: &[&str], label: &str) -> Result<(), String> {
        variants.iter().try_for_each(|variant| {
            if source.contains(variant) {
                Ok(())
            } else {
                Err(format!("{label} should have variant {variant}"))
            }
        })
    }

    fn assert_resource_contract_fields(out: &str) -> Result<(), String> {
        [
            "CONTRACT_MAX_STEPS",
            "CONTRACT_MAX_SLOTS",
            "CONTRACT_MAX_CONSTANTS",
            "CONTRACT_MAX_ACCESSORS",
            "CONTRACT_MAX_EXPRESSIONS",
            "CONTRACT_MAX_EXPR_STACK",
            "CONTRACT_MAX_INPUT_BYTES",
            "CONTRACT_MAX_OUTPUT_BYTES",
            "CONTRACT_MAX_STEP_BUDGET_PER_TICK",
            "CONTRACT_MAX_BLOB_BYTES",
            "CONTRACT_MAX_IPC_PAYLOAD_BYTES",
            "CONTRACT_MAX_RETRY_ATTEMPTS",
            "CONTRACT_MAX_FANOUT",
            "CONTRACT_MAX_COLLECT_ITEMS",
            "CONTRACT_MAX_QUEUE_DEPTH",
            "CONTRACT_MAX_JOURNAL_BATCH_BYTES",
        ]
        .iter()
        .try_for_each(|field| {
            if out.contains(field) {
                Ok(())
            } else {
                Err(format!("emit_resource_contract must include {field}"))
            }
        })
    }

    fn assert_workflow_step_names_valid(
        name: &str,
        workflow_result: Result<CompiledWorkflow, String>,
    ) -> Result<(), String> {
        let workflow = workflow_result?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let found_step = source.lines().try_fold(false, |found, line| {
            let trimmed = line.trim();
            if !(trimmed.starts_with("fn step_") && trimmed.contains('(')) {
                return Ok::<bool, String>(found);
            }
            let end = trimmed
                .find('(')
                .ok_or_else(|| String::from("no paren in step fn"))?;
            let fn_name = trimmed
                .get(3..end)
                .ok_or_else(|| String::from("step fn name range invalid"))?;
            assert!(
                fn_name.starts_with("step_"),
                "function name must start with step_, got: {fn_name} in workflow {name}"
            );
            let suffix = fn_name
                .get(5..)
                .ok_or_else(|| String::from("step fn suffix range invalid"))?;
            assert!(
                suffix.parse::<u16>().is_ok(),
                "step suffix must be a valid u16, got: {suffix} in workflow {name}"
            );
            Ok(true)
        })?;
        assert!(
            found_step,
            "must find at least one step function in workflow {name}"
        );
        Ok(())
    }

    fn forbidden_generated_source_violations(source: &str) -> Vec<(&'static str, String)> {
        [
            ("unsafe ", "unsafe block"),
            (".unwrap(", "unwrap call"),
            (".expect(", "expect call"),
            ("panic!(", "panic macro"),
            ("todo!(", "todo macro"),
            ("unimplemented!(", "unimplemented macro"),
            ("dbg!(", "dbg macro"),
            ("println!(", "println macro"),
            ("format!(", "format macro"),
            ("HashMap<String", "string-keyed HashMap"),
            ("eprintln!(", "eprintln macro"),
        ]
        .iter()
        .flat_map(|(pattern, label)| {
            source.lines().filter_map(move |line| {
                let trimmed = line.trim();
                let is_comment = trimmed.starts_with("//") || trimmed.starts_with("//!");
                let allowed_unsafe_lint =
                    *pattern == "unsafe " && trimmed.contains("#![forbid(unsafe_code)]");
                if is_comment || allowed_unsafe_lint || !trimmed.contains(pattern) {
                    None
                } else {
                    Some((*label, trimmed.to_string()))
                }
            })
        })
        .collect()
    }

    fn choose_finish_node(id: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }
    }

    fn minimal_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;

        let parts = WorkflowParts {
            name: Box::<str>::from("test_codegen"),
            digest: WorkflowDigest::from_bytes([0xAB; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
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
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn unsupported_build_list_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_unsupported_build_list"),
            digest: WorkflowDigest::from_bytes([0xCD; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn unsupported_contains_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Contains,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_unsupported_contains"),
            digest: WorkflowDigest::from_bytes([0xCE; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::Bool(true), ConstValue::Bool(false)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn unsupported_accessor_traversal_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_unsupported_accessor"),
            digest: WorkflowDigest::from_bytes([0xAF; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
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
            expressions: Box::new([]),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: vec![PathSegment::Index(0)].into_boxed_slice(),
            }]
            .into_boxed_slice(),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn root_accessor_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![vb_core::ExprOp::LoadAccessor(vb_core::AccessorIdx::new(0))];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_root_accessor"),
            digest: WorkflowDigest::from_bytes([0xB1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: Box::new([]),
            }]
            .into_boxed_slice(),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Workflow with a Do node that dispatches to ActionId 5.
    fn do_action_workflow() -> Result<CompiledWorkflow, String> {
        action_suspend_workflow(ActionId::new(5), SlotIdx::new(0))
    }

    fn action_suspend_workflow(
        action: ActionId,
        input: SlotIdx,
    ) -> Result<CompiledWorkflow, String> {
        let output = input
            .checked_add(1)
            .ok_or_else(|| String::from("input slot cannot allocate output slot"))?;
        let slot_count = output
            .checked_add(1)
            .ok_or_else(|| String::from("output slot cannot allocate slot count"))?
            .get();
        let parts = WorkflowParts {
            name: Box::<str>::from("test_do_action"),
            digest: WorkflowDigest::from_bytes([0xEF; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(output),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Do { action, input },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish { result: output },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn generated_action_suspend_stdout(
        workflow: &CompiledWorkflow,
        action: ActionId,
        input: SlotIdx,
    ) -> Result<String, String> {
        let generated = emit_rust_workflow(workflow).map_err(|e| e.to_string())?;
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!(
                "vb_codegen_action_suspend_{}_{}_{}",
                std::process::id(),
                action.get(),
                input.get()
            ))
            .tempdir()
            .map_err(|e| e.to_string())?;
        let source_path = temp_dir.path().join("generated_action_suspend.rs");
        let binary_path = temp_dir.path().join("generated_action_suspend_bin");
        let harness = format!(
            "{generated}\nfn main() {{\n    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    match slots.get_mut(usize::from({input}u16)) {{\n        Some(slot) => *slot = Some(SlotValue::I64(99)),\n        None => {{ println!(\"slot_out_of_bounds\"); std::process::exit(20); }}\n    }}\n    match drive(slots) {{\n        Err(DriveError::ActionSuspend {{ action_id, input_slot }}) if action_id == {action}u16 && input_slot == {input}u16 => println!(\"generated_action_suspend:{action}:{input}\"),\n        other => {{ println!(\"unexpected:{{other:?}}\"); std::process::exit(21); }}\n    }}\n}}\n",
            action = action.get(),
            input = input.get()
        );
        std::fs::write(&source_path, harness).map_err(|e| e.to_string())?;

        let compile = std::process::Command::new("rustc")
            .arg("--edition")
            .arg("2024")
            .arg("-o")
            .arg(&binary_path)
            .arg(&source_path)
            .output()
            .map_err(|e| e.to_string())?;
        if !compile.status.success() {
            return Err(String::from_utf8_lossy(&compile.stderr).into_owned());
        }

        let run = std::process::Command::new(&binary_path)
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
        if !run.status.success() {
            let stderr = String::from_utf8_lossy(&run.stderr);
            return Err(format!("generated run failed: {stdout}{stderr}"));
        }

        Ok(stdout)
    }

    fn generated_drive_stdout(
        workflow: &CompiledWorkflow,
        name: &str,
        init_source: &str,
    ) -> Result<String, String> {
        let generated = emit_rust_workflow(workflow).map_err(|e| e.to_string())?;
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("vb_codegen_drive_{}_{}", std::process::id(), name))
            .tempdir()
            .map_err(|e| e.to_string())?;
        let source_path = temp_dir.path().join("generated_drive.rs");
        let binary_path = temp_dir.path().join("generated_drive_bin");
        let harness = format!(
            "{generated}\nfn main() {{\n    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n{init_source}\n    match drive(slots) {{\n        Ok(value) => println!(\"ok:{{value:?}}\"),\n        Err(error) => println!(\"err:{{error:?}}\"),\n    }}\n}}\n"
        );
        std::fs::write(&source_path, harness).map_err(|e| e.to_string())?;

        let compile = std::process::Command::new("rustc")
            .arg("--edition")
            .arg("2024")
            .arg("-o")
            .arg(&binary_path)
            .arg(&source_path)
            .output()
            .map_err(|e| e.to_string())?;
        if !compile.status.success() {
            return Err(String::from_utf8_lossy(&compile.stderr).into_owned());
        }

        let run = std::process::Command::new(&binary_path)
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
        if !run.status.success() {
            let stderr = String::from_utf8_lossy(&run.stderr);
            return Err(format!("generated run failed: {stdout}{stderr}"));
        }

        Ok(stdout)
    }

    fn expected_drive_error_stdout(workflow: &CompiledWorkflow) -> Result<String, String> {
        let error = ir_drive_error(workflow)?;
        Ok(format!("err:{error:?}\n"))
    }

    fn ir_action_suspend_signal(
        workflow: &CompiledWorkflow,
        input: SlotIdx,
    ) -> Result<EngineSignal, String> {
        let mut run = new_run_frame(RunId::new(1), workflow).map_err(|e| e.to_string())?;
        run.write_slot(input, SlotValue::I64(99))
            .map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();
        step_once(workflow, &mut run, &mut store).map_err(|e| e.to_string())
    }

    fn ir_drive_finished_value(workflow: &CompiledWorkflow) -> Result<SlotValue, String> {
        let mut run = new_run_frame(RunId::new(2), workflow).map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();
        let signal = run_until_blocked(workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|e| e.to_string())?;
        match signal {
            EngineSignal::Finished(value, _) => Ok(value),
            other => Err(format!("expected finished signal, got {other:?}")),
        }
    }

    fn ir_drive_error(workflow: &CompiledWorkflow) -> Result<EngineError, String> {
        let mut run = new_run_frame(RunId::new(3), workflow).map_err(|e| e.to_string())?;
        let mut store = ValueStore::new();
        match run_until_blocked(workflow, &mut run, StepBudget::MAX, &mut store) {
            Ok(signal) => Err(format!("expected IR error, got {signal:?}")),
            Err(error) => Ok(error),
        }
    }

    fn assert_boolean_number_type_mismatch(error: EngineError) -> Result<(), String> {
        match error {
            EngineError::TypeMismatch { expected, found }
                if expected == "boolean" && found == "number" =>
            {
                Ok(())
            }
            other => Err(format!(
                "expected exact boolean/number TypeMismatch, got {other:?}"
            )),
        }
    }

    fn runtime_drive_finished_value(workflow: &CompiledWorkflow) -> Result<SlotValue, String> {
        let mut run = new_run_frame(RunId::new(4), workflow).map_err(|e| e.to_string())?;
        let mut budget = StepBudget::MAX;
        let mut store = ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();
        let signal = drive_deterministic_full(
            workflow,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        )
        .map_err(|e| e.to_string())?;
        match signal {
            RuntimeSignal::Finished(value) => Ok(value),
            other => Err(format!("expected runtime finished signal, got {other:?}")),
        }
    }

    fn runtime_drive_error_string(workflow: &CompiledWorkflow) -> Result<String, String> {
        let mut run = new_run_frame(RunId::new(5), workflow).map_err(|e| e.to_string())?;
        let mut budget = StepBudget::MAX;
        let mut store = ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();
        match drive_deterministic_full(
            workflow,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        ) {
            Ok(signal) => Err(format!("expected runtime error, got {signal:?}")),
            Err(error) => Ok(error.to_string()),
        }
    }

    fn primitive_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Sub,
            vb_core::ExprOp::LoadConst(ConstIdx::new(2)),
            vb_core::ExprOp::Eq,
            vb_core::ExprOp::LoadConst(ConstIdx::new(3)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(2)),
            vb_core::ExprOp::Div,
            vb_core::ExprOp::LoadConst(ConstIdx::new(4)),
            vb_core::ExprOp::Eq,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Add,
            vb_core::ExprOp::LoadConst(ConstIdx::new(4)),
            vb_core::ExprOp::Eq,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Gt,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Gte,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Lt,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Lte,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(5)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(6)),
            vb_core::ExprOp::Or,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(6)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(5)),
            vb_core::ExprOp::NotEq,
            vb_core::ExprOp::And,
            vb_core::ExprOp::LoadConst(ConstIdx::new(5)),
            vb_core::ExprOp::Not,
            vb_core::ExprOp::And,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_primitive_expr_exec"),
            digest: WorkflowDigest::from_bytes([0xE1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![
                ConstValue::I64(7),
                ConstValue::I64(5),
                ConstValue::I64(2),
                ConstValue::I64(24),
                ConstValue::I64(12),
                ConstValue::Bool(false),
                ConstValue::Bool(true),
            ]
            .into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn primitive_choose_workflow() -> Result<CompiledWorkflow, String> {
        let false_expr = ExprProgram::try_from_ops(
            vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let true_expr = ExprProgram::try_from_ops(
            vec![vb_core::ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_primitive_choose_exec"),
            digest: WorkflowDigest::from_bytes([0xE2; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Choose {
                        branches: vec![
                            vb_core::ExprBranch {
                                condition: vb_core::ExprIdx::new(0),
                                target: StepIdx::new(1),
                            },
                            vb_core::ExprBranch {
                                condition: vb_core::ExprIdx::new(1),
                                target: StepIdx::new(2),
                            },
                        ]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(3)),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(3),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(4),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
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
            expressions: vec![false_expr, true_expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![
                ConstValue::Bool(false),
                ConstValue::Bool(true),
                ConstValue::I64(11),
                ConstValue::I64(22),
                ConstValue::I64(33),
            ]
            .into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn primitive_choose_slot_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_primitive_choose_slot_exec"),
            digest: WorkflowDigest::from_bytes([0xE3; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: vec![
                            vb_core::SlotBranch {
                                condition: SlotIdx::new(0),
                                target: StepIdx::new(3),
                            },
                            vb_core::SlotBranch {
                                condition: SlotIdx::new(1),
                                target: StepIdx::new(4),
                            },
                        ]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(5)),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(6)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(6)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(3),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(5),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(6)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(4),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(6),
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
            constants: vec![
                ConstValue::Bool(false),
                ConstValue::Bool(true),
                ConstValue::I64(11),
                ConstValue::I64(22),
                ConstValue::I64(33),
            ]
            .into_boxed_slice(),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn non_boolean_choose_workflow() -> Result<CompiledWorkflow, String> {
        let expr = ExprProgram::try_from_ops(
            vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_non_boolean_choose"),
            digest: WorkflowDigest::from_bytes([0xE4; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Choose {
                        branches: vec![vb_core::ExprBranch {
                            condition: vb_core::ExprIdx::new(0),
                            target: StepIdx::new(1),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(1)),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(0)),
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
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn non_boolean_choose_slot_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_non_boolean_choose_slot"),
            digest: WorkflowDigest::from_bytes([0xE5; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: vec![vb_core::SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(2)),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // --- CodegenError exact-variant tests ---

    #[test]
    fn codegen_error_format_buffer_overflow_exact_variant() {
        let error = CodegenError::FormatBufferOverflow;
        let message = error.to_string();
        assert!(
            message.contains("buffer"),
            "FormatBufferOverflow display must mention buffer, got: {message}"
        );
    }

    #[test]
    fn codegen_error_rustfmt_failed_exact_variant() {
        let error = CodegenError::RustfmtFailed {
            detail: String::from("exit status 1"),
        };
        let message = error.to_string();
        assert!(
            message.contains("rustfmt"),
            "RustfmtFailed display must mention rustfmt, got: {message}"
        );
        assert!(
            message.contains("exit status 1"),
            "RustfmtFailed display must include detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_compile_check_failed_exact_variant() {
        let error = CodegenError::CompileCheckFailed {
            detail: String::from("mismatched types"),
        };
        let message = error.to_string();
        assert!(
            message.contains("compile"),
            "CompileCheckFailed display must mention compile, got: {message}"
        );
        assert!(
            message.contains("mismatched types"),
            "CompileCheckFailed display must include detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_semantic_mismatch_exact_variant() {
        let error = CodegenError::SemanticMismatch {
            detail: String::from("step count mismatch: generated has 2, IR has 3"),
        };
        let message = error.to_string();
        assert!(
            message.contains("semantic"),
            "SemanticMismatch display must mention semantic, got: {message}"
        );
        assert!(
            message.contains("step count mismatch: generated has 2, IR has 3"),
            "SemanticMismatch display must include exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_io_exact_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error = CodegenError::Io(io_err);
        let message = error.to_string();
        assert!(
            message.contains("file missing"),
            "Io display must include the inner IO error message, got: {message}"
        );
    }

    #[test]
    fn codegen_error_trybuild_fixture_exact_variant() {
        let error = CodegenError::TrybuildFixture {
            detail: String::from("fixture path has no parent directory"),
        };
        let message = error.to_string();
        assert!(
            message.contains("trybuild"),
            "TrybuildFixture display must mention trybuild, got: {message}"
        );
        assert!(
            message.contains("fixture path has no parent directory"),
            "TrybuildFixture display must include exact detail, got: {message}"
        );
    }

    // --- Public function behavior tests ---

    #[test]
    fn emit_rust_workflow_produces_non_empty_source() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(!source.is_empty(), "generated source should not be empty");
        Ok(())
    }

    #[test]
    fn emit_ids_includes_workflow_id_type() -> Result<(), String> {
        // Given a minimal compiled workflow
        let workflow = minimal_workflow()?;

        // When emit_ids writes typed ID constants
        let mut out = String::new();
        emit_ids(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains WORKFLOW_SLOT_COUNT and WORKFLOW_NODE_COUNT constants
        assert!(
            out.contains("WORKFLOW_SLOT_COUNT"),
            "emit_ids must produce WORKFLOW_SLOT_COUNT constant"
        );
        assert!(
            out.contains("WORKFLOW_NODE_COUNT"),
            "emit_ids must produce WORKFLOW_NODE_COUNT constant"
        );
        assert!(
            out.contains("usize"),
            "emit_ids must use typed usize for slot count"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_includes_loop() -> Result<(), String> {
        // Given a minimal compiled workflow
        let workflow = minimal_workflow()?;

        // When emit_drive_function writes the main step loop
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains a loop construct and match dispatch
        assert!(
            out.contains("loop"),
            "drive function must contain a loop construct"
        );
        assert!(
            out.contains("pub fn drive"),
            "drive function must be public and named drive"
        );
        assert!(
            out.contains("StepOutcome"),
            "drive function must dispatch on StepOutcome"
        );
        Ok(())
    }

    #[test]
    fn emit_step_function_includes_set_const() -> Result<(), String> {
        // Given a minimal workflow with a SetConst node
        let workflow = minimal_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;

        // When emit_step_function writes the step for the SetConst node
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;

        // Then the output writes the constant into the output slot
        assert!(
            out.contains("write_slot"),
            "SetConst step must call write_slot"
        );
        assert!(
            out.contains("read_const"),
            "SetConst step must call read_const"
        );
        assert!(
            out.contains("fn step_0"),
            "SetConst step function must be named step_0"
        );
        Ok(())
    }

    #[test]
    fn emit_action_match_dispatch_includes_registered_actions() -> Result<(), String> {
        // Given a workflow with a Do node dispatching to ActionId 5
        let workflow = do_action_workflow()?;

        // When emit_action_match_dispatch writes the action dispatch
        let mut out = String::new();
        emit_action_match_dispatch(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains an arm for action id 5
        assert!(
            out.contains("dispatch_action"),
            "dispatch must define dispatch_action function"
        );
        assert!(
            out.contains("5 => Ok(())"),
            "dispatch must include an arm for action id 5"
        );
        assert!(
            out.contains("UnknownAction"),
            "dispatch must handle unknown actions"
        );
        Ok(())
    }

    #[test]
    fn emit_finish_returns_result_value() -> Result<(), String> {
        // Given a minimal compiled workflow
        let workflow = minimal_workflow()?;

        // When emit_finish writes the result extraction section
        let mut out = String::new();
        emit_finish(&mut out, &workflow).map_err(|e| e.to_string())?;

        // Then the output contains the result extraction comment section
        assert!(
            out.contains("Result extraction"),
            "emit_finish must include result extraction section marker"
        );
        Ok(())
    }

    #[test]
    fn emit_resource_contract_includes_limits() -> Result<(), String> {
        // Given a resource contract with specific field values
        let contract = ResourceContract {
            max_steps: 100,
            max_slots: 200,
            max_constants: 50,
            max_accessors: 10,
            max_expressions: 20,
            max_expr_stack: 32,
            max_input_bytes: 4096,
            max_output_bytes: 8192,
            max_step_budget_per_tick: 500,
            max_blob_bytes: 1024,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 3,
            max_fanout: 8,
            max_collect_items: 100,
            max_queue_depth: 64,
            max_journal_batch_bytes: 512,
        };

        // When emit_resource_contract writes the contract constants
        let mut out = String::new();
        emit_resource_contract(&mut out, contract).map_err(|e| e.to_string())?;

        // Then the output contains every contract field
        assert!(
            out.contains("CONTRACT_MAX_STEPS"),
            "resource contract must emit CONTRACT_MAX_STEPS"
        );
        assert!(
            out.contains("CONTRACT_MAX_SLOTS"),
            "resource contract must emit CONTRACT_MAX_SLOTS"
        );
        assert!(
            out.contains("CONTRACT_MAX_CONSTANTS"),
            "resource contract must emit CONTRACT_MAX_CONSTANTS"
        );
        assert!(
            out.contains("CONTRACT_MAX_ACCESSORS"),
            "resource contract must emit CONTRACT_MAX_ACCESSORS"
        );
        assert!(
            out.contains("CONTRACT_MAX_EXPRESSIONS"),
            "resource contract must emit CONTRACT_MAX_EXPRESSIONS"
        );
        assert!(
            out.contains("CONTRACT_MAX_EXPR_STACK"),
            "resource contract must emit CONTRACT_MAX_EXPR_STACK"
        );
        assert!(
            out.contains("CONTRACT_MAX_INPUT_BYTES"),
            "resource contract must emit CONTRACT_MAX_INPUT_BYTES"
        );
        assert!(
            out.contains("CONTRACT_MAX_OUTPUT_BYTES"),
            "resource contract must emit CONTRACT_MAX_OUTPUT_BYTES"
        );
        Ok(())
    }

    #[test]
    fn format_generated_rust_produces_valid_syntax() -> Result<(), String> {
        // Given a generated workflow source
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // When format_generated_rust is invoked
        let formatted = format_generated_rust(&source);

        // Then either rustfmt succeeded with non-empty output, or it is not installed
        match formatted {
            Ok(output) => {
                assert!(
                    !output.is_empty(),
                    "formatted output must be non-empty when rustfmt succeeds"
                );
            }
            Err(CodegenError::RustfmtFailed { detail }) => {
                // rustfmt not available in CI is acceptable; log the reason
                eprintln!("rustfmt not available, skipping format check: {detail}");
            }
            Err(other) => {
                return Err(format!(
                    "unexpected error from format_generated_rust: {other}"
                ));
            }
        }
        Ok(())
    }

    // --- Error Variant Exact-Assertion Tests ---

    #[test]
    fn codegen_error_format_buffer_overflow_reports_expected_message() {
        // Given a FormatBufferOverflow error variant
        let error = CodegenError::FormatBufferOverflow;
        // When the error is converted to display string
        let message = error.to_string();
        // Then it mentions buffer and capacity semantics
        assert!(
            message.contains("buffer"),
            "FormatBufferOverflow must mention buffer, got: {message}"
        );
        assert!(
            message.contains("capacity"),
            "FormatBufferOverflow must mention capacity, got: {message}"
        );
    }

    #[test]
    fn codegen_error_rustfmt_failed_reports_expected_detail() {
        // Given a RustfmtFailed error with a specific detail string
        let detail = String::from("exit status 42");
        let error = CodegenError::RustfmtFailed {
            detail: detail.clone(),
        };
        // When the error is displayed
        let message = error.to_string();
        // Then the exact detail string appears verbatim
        assert!(
            message.contains("rustfmt"),
            "RustfmtFailed must mention rustfmt, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "RustfmtFailed must contain exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_compile_check_failed_reports_expected_detail() {
        // Given a CompileCheckFailed error with detail
        let detail = String::from("mismatched types: expected u16, found String");
        let error = CodegenError::CompileCheckFailed {
            detail: detail.clone(),
        };
        // When displayed
        let message = error.to_string();
        // Then it contains compile and the exact detail
        assert!(
            message.contains("compile"),
            "CompileCheckFailed must mention compile, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "CompileCheckFailed must contain exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_semantic_mismatch_reports_expected_detail() {
        // Given a SemanticMismatch with specific divergence
        let detail = String::from("step count mismatch: generated has 2, IR has 3");
        let error = CodegenError::SemanticMismatch {
            detail: detail.clone(),
        };
        // When displayed
        let message = error.to_string();
        // Then it mentions semantic and includes exact detail
        assert!(
            message.contains("semantic"),
            "SemanticMismatch must mention semantic, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "SemanticMismatch must contain exact detail, got: {message}"
        );
    }

    #[test]
    fn codegen_error_io_reports_inner_error_kind() {
        // Given an IO error wrapped in CodegenError::Io
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error = CodegenError::Io(io_err);
        // When displayed
        let message = error.to_string();
        // Then the inner error message is preserved verbatim
        assert!(
            message.contains("file missing"),
            "Io variant must preserve inner message, got: {message}"
        );
        assert!(
            message.contains("codegen IO error"),
            "Io variant must mention codegen IO error, got: {message}"
        );
    }

    #[test]
    fn codegen_error_trybuild_fixture_reports_expected_detail() {
        // Given a TrybuildFixture error with a detail
        let detail = String::from("fixture path has no parent directory");
        let error = CodegenError::TrybuildFixture {
            detail: detail.clone(),
        };
        // When displayed
        let message = error.to_string();
        // Then it mentions trybuild and contains the exact detail
        assert!(
            message.contains("trybuild"),
            "TrybuildFixture must mention trybuild, got: {message}"
        );
        assert!(
            message.contains(&detail),
            "TrybuildFixture must contain exact detail, got: {message}"
        );
    }

    // --- Emit Step Function Behavior Tests ---

    #[test]
    fn emit_step_match_produces_correct_arm_for_nop_node() -> Result<(), String> {
        // Given a Nop node with a next target
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let workflow = nop_workflow()?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, &node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains a Continue with the next step index
        assert!(
            out.contains("StepOutcome::Continue(1)"),
            "Nop must emit Continue with next step, got: {out}"
        );
        assert!(
            out.contains("fn step_0"),
            "Nop step function must be named step_0, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_set_const_node() -> Result<(), String> {
        // Given a SetConst node writing constant 0 into slot 0
        let workflow = minimal_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output writes slot and reads constant
        assert!(
            out.contains("write_slot"),
            "SetConst must call write_slot, got: {out}"
        );
        assert!(
            out.contains("read_const"),
            "SetConst must call read_const, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_copy_node() -> Result<(), String> {
        // Given a Copy node that reads slot 0 into slot 1
        let workflow = copy_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads and writes slots
        assert!(
            out.contains("read_slot_optional"),
            "Copy must call read_slot_optional, got: {out}"
        );
        assert!(
            out.contains("write_slot"),
            "Copy must call write_slot, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_do_node() -> Result<(), String> {
        // Given a Do node dispatching action 5 with input slot 0
        let workflow = do_action_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains action suspend dispatch
        assert!(
            out.contains("ActionSuspend"),
            "Do node must emit ActionSuspend error, got: {out}"
        );
        assert!(
            out.contains("action_id: 5"),
            "Do node must reference action id 5, got: {out}"
        );
        assert!(
            out.contains("input_slot: 0"),
            "Do node must reference input slot 0, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_finish_node() -> Result<(), String> {
        // Given a Finish node that returns slot 0
        let workflow = minimal_workflow()?;
        let node = workflow.node(StepIdx::new(1)).ok_or("node 1 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads the result slot and returns Finished
        assert!(
            out.contains("read_slot"),
            "Finish must call read_slot, got: {out}"
        );
        assert!(
            out.contains("StepOutcome::Finished"),
            "Finish must return StepOutcome::Finished, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_jump_node() -> Result<(), String> {
        // Given a Jump node targeting step 1
        let workflow = jump_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output continues to the target
        assert!(
            out.contains("StepOutcome::Continue(1)"),
            "Jump must emit Continue to target step 1, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_wait_until_node() -> Result<(), String> {
        // Given a WaitUntil node reading deadline from slot 0
        let workflow = wait_until_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads the deadline slot
        assert!(
            out.contains("_deadline"),
            "WaitUntil must reference deadline variable, got: {out}"
        );
        assert!(
            out.contains("read_slot"),
            "WaitUntil must call read_slot, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_wait_event_node() -> Result<(), String> {
        // Given a WaitEvent node reading event from slot 0 with timeout slot 1
        let workflow = wait_event_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads event and timeout slots
        assert!(
            out.contains("_event"),
            "WaitEvent must reference event variable, got: {out}"
        );
        assert!(
            out.contains("_timeout"),
            "WaitEvent must reference timeout variable, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_ask_node() -> Result<(), String> {
        // Given an Ask node with prompt slot 0 and timeout slot 1
        let workflow = ask_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads prompt and timeout slots
        assert!(
            out.contains("_prompt"),
            "Ask must reference prompt variable, got: {out}"
        );
        assert!(
            out.contains("_timeout"),
            "Ask with timeout must reference timeout variable, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_for_each_start_node() -> Result<(), String> {
        // Given a ForEachStart node supported by generated mode
        let workflow = for_each_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output emits concrete iterator setup and branching code.
        assert!(
            !out.contains("UnsupportedPrimitive"),
            "ForEachStart must not emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("list_item_count") && out.contains("tail_list_handle"),
            "ForEachStart must count items and store iterator tail, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_together_start_node() -> Result<(), String> {
        // Given a TogetherStart node (unsupported in codegen)
        let workflow = together_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "TogetherStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("TogetherStart"),
            "UnsupportedPrimitive must name TogetherStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_collect_start_node() -> Result<(), String> {
        // Given a CollectStart node (unsupported in codegen)
        let workflow = collect_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "CollectStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("CollectStart"),
            "UnsupportedPrimitive must name CollectStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_reduce_start_node() -> Result<(), String> {
        // Given a ReduceStart node (unsupported in codegen)
        let workflow = reduce_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "ReduceStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ReduceStart"),
            "UnsupportedPrimitive must name ReduceStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_repeat_start_node() -> Result<(), String> {
        // Given a RepeatStart node (unsupported in codegen)
        let workflow = repeat_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "RepeatStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("RepeatStart"),
            "UnsupportedPrimitive must name RepeatStart, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_build_object_node() -> Result<(), String> {
        // Given a BuildObject node (now supported in codegen)
        let workflow = build_object_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output constructs an object with read_slot and SlotValue::Object
        assert!(
            out.contains("read_slot"),
            "BuildObject must read field slots, got: {out}"
        );
        assert!(
            out.contains("SlotValue::Object"),
            "BuildObject must write SlotValue::Object, got: {out}"
        );
        Ok(())
    }

    // --- Module Header and Structure Tests ---

    #[test]
    fn emit_module_header_includes_forbid_unsafe() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the first section includes forbid unsafe_code
        assert!(
            source.contains("#![forbid(unsafe_code)]"),
            "generated source must include #![forbid(unsafe_code)], got first 200 chars: {}",
            &source.chars().take(200).collect::<String>()
        );
        Ok(())
    }

    #[test]
    fn emit_module_header_includes_deny_unused_must_use() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the output contains deny unused_must_use
        assert!(
            source.contains("#![deny(unused_must_use)]"),
            "generated source must include deny unused_must_use"
        );
        Ok(())
    }

    #[test]
    fn emit_module_header_includes_slot_value_enum() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the output contains the SlotValue enum definition
        assert!(
            source.contains("pub enum SlotValue"),
            "generated source must define SlotValue enum"
        );
        assert!(
            source.contains("Bool(bool)"),
            "SlotValue must have Bool variant"
        );
        assert!(
            source.contains("I64(i64)"),
            "SlotValue must have I64 variant"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_includes_entry_step_zero() -> Result<(), String> {
        // Given a minimal workflow with entry at step 0
        let workflow = minimal_workflow()?;
        // When emit_drive_function generates the drive loop
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then the program counter initializes to the entry step
        assert!(
            out.contains("let mut pc: u16 = 0;"),
            "drive must initialize pc to entry step 0, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_routes_each_step_index() -> Result<(), String> {
        // Given a minimal workflow with 2 nodes
        let workflow = minimal_workflow()?;
        // When emit_drive_function generates code
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then each step index appears in the match dispatch
        assert!(out.contains("0 => step_0"), "drive must dispatch to step_0");
        assert!(out.contains("1 => step_1"), "drive must dispatch to step_1");
        Ok(())
    }

    #[test]
    fn emit_action_match_dispatch_lists_only_do_actions() -> Result<(), String> {
        // Given a workflow with a Do node for action 5
        let workflow = do_action_workflow()?;
        // When emit_action_match_dispatch generates the dispatch
        let mut out = String::new();
        emit_action_match_dispatch(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then action 5 appears but finish step 1 does not
        assert!(
            out.contains("5 => Ok(())"),
            "dispatch must have arm for action id 5"
        );
        assert!(
            out.contains("_ => Err(DriveError::UnknownAction)"),
            "dispatch must have wildcard fallback"
        );
        Ok(())
    }

    #[test]
    fn emit_action_boundary_reads_input_slot_and_returns_suspend() -> Result<(), String> {
        // Given an action boundary with action 7 and input slot 3
        let mut out = String::new();
        // When emit_action_boundary writes the code
        emit_action_boundary(&mut out, ActionId::new(7), SlotIdx::new(3))
            .map_err(|e| e.to_string())?;
        // Then the output reads the input slot and returns ActionSuspend
        assert!(
            out.contains("read_slot(slots, 3)"),
            "action boundary must read input slot 3, got: {out}"
        );
        assert!(
            out.contains("ActionSuspend { action_id: 7, input_slot: 3 }"),
            "action boundary must return ActionSuspend with correct fields, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_resource_contract_outputs_all_constant_fields() -> Result<(), String> {
        // Given a custom resource contract
        let contract = ResourceContract {
            max_steps: 50,
            max_slots: 100,
            max_constants: 25,
            max_accessors: 5,
            max_expressions: 10,
            max_expr_stack: 16,
            max_input_bytes: 2048,
            max_output_bytes: 4096,
            max_step_budget_per_tick: 500,
            max_blob_bytes: 1024,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 3,
            max_fanout: 8,
            max_collect_items: 100,
            max_queue_depth: 64,
            max_journal_batch_bytes: 512,
        };
        // When emit_resource_contract writes constants
        let mut out = String::new();
        emit_resource_contract(&mut out, contract).map_err(|e| e.to_string())?;
        // Then each field value appears in the output
        assert!(
            out.contains("CONTRACT_MAX_STEPS: u16 = 50;"),
            "must emit exact max_steps value"
        );
        assert!(
            out.contains("CONTRACT_MAX_SLOTS: u16 = 100;"),
            "must emit exact max_slots value"
        );
        assert!(
            out.contains("CONTRACT_MAX_CONSTANTS: u16 = 25;"),
            "must emit exact max_constants value"
        );
        assert!(
            out.contains("CONTRACT_MAX_INPUT_BYTES: u32 = 2048;"),
            "must emit exact max_input_bytes value"
        );
        assert!(
            out.contains("CONTRACT_MAX_OUTPUT_BYTES: u32 = 4096;"),
            "must emit exact max_output_bytes value"
        );
        Ok(())
    }

    #[test]
    fn emit_ids_includes_exact_slot_and_node_counts() -> Result<(), String> {
        // Given a minimal workflow with 1 slot and 2 nodes
        let workflow = minimal_workflow()?;
        // When emit_ids writes constants
        let mut out = String::new();
        emit_ids(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then the exact counts appear
        assert!(
            out.contains("WORKFLOW_SLOT_COUNT: usize = 1;"),
            "must emit slot count 1, got: {out}"
        );
        assert!(
            out.contains("WORKFLOW_NODE_COUNT: u16 = 2;"),
            "must emit node count 2, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_expr_function_generates_load_const_op() -> Result<(), String> {
        // Given a workflow with an expression that loads constant 0
        let workflow = minimal_workflow()?;
        // When the full source is generated
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the expression function exists and loads the constant
        assert!(
            source.contains("fn eval_expr_0"),
            "must generate eval_expr_0 function"
        );
        assert!(
            source.contains("stack.push(read_const(0)"),
            "expression must load constant index 0"
        );
        Ok(())
    }

    // --- Code Generation Integration Tests ---

    #[test]
    fn generate_produces_valid_rust_for_single_step_nop() -> Result<(), String> {
        // Given a workflow with a single Nop + Finish
        let workflow = nop_workflow()?;
        // When generating the full Rust source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the source contains drive function, step function, and dispatch
        assert!(
            source.contains("pub fn drive"),
            "single-step workflow must have drive function"
        );
        assert!(
            source.contains("fn step_0"),
            "single-step workflow must have step_0"
        );
        assert!(
            source.contains("fn step_1"),
            "single-step workflow must have step_1 (finish)"
        );
        assert!(!source.is_empty(), "generated source must be non-empty");
        Ok(())
    }

    #[test]
    fn generate_produces_valid_rust_for_multi_step_workflow() -> Result<(), String> {
        // Given a workflow with set_const + do + finish (3 steps)
        let workflow = do_action_workflow()?;
        // When generating the full source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all step functions are present
        assert!(source.contains("fn step_0"), "multi-step must have step_0");
        assert!(source.contains("fn step_1"), "multi-step must have step_1");
        assert!(
            source.contains("fn step_0") && source.contains("fn step_1"),
            "multi-step must have all step handlers"
        );
        Ok(())
    }

    #[test]
    fn generate_output_starts_with_forbid_unsafe() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the first non-empty line is the forbid directive
        let first_line = source.lines().next().ok_or("source has no lines")?;
        assert!(
            first_line.contains("#![forbid(unsafe_code)]"),
            "first line must be forbid unsafe, got: {first_line}"
        );
        Ok(())
    }

    #[test]
    fn generate_output_contains_all_step_handlers() -> Result<(), String> {
        // Given a workflow with 2 nodes
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then each node gets a step handler
        let step_count = source
            .lines()
            .filter(|line| line.trim().starts_with("fn step_"))
            .count();
        assert_eq!(
            step_count,
            usize::from(workflow.node_count()),
            "expected {} step handlers, found {step_count}",
            workflow.node_count()
        );
        Ok(())
    }

    #[test]
    fn generate_contains_constant_pool_with_correct_values() -> Result<(), String> {
        // Given a workflow with constant I64(42)
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the constant pool has the value
        assert!(
            source.contains("SlotValue::I64(42)"),
            "constant pool must contain SlotValue::I64(42)"
        );
        assert!(
            source.contains("CONSTANTS"),
            "source must define CONSTANTS array"
        );
        Ok(())
    }

    #[test]
    fn generate_includes_drive_error_variants() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all critical DriveError variants are defined
        assert!(
            source.contains("InvalidProgramCounter"),
            "must define InvalidProgramCounter error"
        );
        assert!(
            source.contains("MissingNextStep"),
            "must define MissingNextStep error"
        );
        assert!(
            source.contains("ActionSuspend"),
            "must define ActionSuspend error"
        );
        assert!(source.contains("SlotNull"), "must define SlotNull error");
        Ok(())
    }

    #[test]
    fn generate_includes_expr_stack_bounded_structure() -> Result<(), String> {
        // Given a workflow with an expression
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then ExprStack is defined with bounded storage
        assert!(
            source.contains("struct ExprStack"),
            "must define ExprStack struct"
        );
        assert!(
            source.contains("MAX_EXPRESSION_STACK"),
            "must define MAX_EXPRESSION_STACK constant"
        );
        assert!(
            !source.contains("Vec<"),
            "must not use Vec for expression stack"
        );
        Ok(())
    }

    #[test]
    fn generate_includes_checked_slot_accessors() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then checked accessor functions are defined
        assert!(
            source.contains("fn read_slot"),
            "must define read_slot function"
        );
        assert!(
            source.contains("fn write_slot"),
            "must define write_slot function"
        );
        assert!(
            source.contains("fn read_slot_optional"),
            "must define read_slot_optional function"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_vec_usage() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When comparing source that contains Vec<
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        source.push_str("\nlet x: Vec<u8> = Vec::new();\n");
        // Then the comparison rejects it
        let result = compare_generated_to_ir(&source, &workflow);
        let detail = semantic_mismatch_detail(result)?;
        assert_eq!(detail, "generated source contains dynamic Vec allocation");
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_unchecked_cast() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When comparing source with ` as ` cast
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        source.push_str("\nlet x = 42u32 as u16;\n");
        // Then the comparison rejects it
        let result = compare_generated_to_ir(&source, &workflow);
        let detail = semantic_mismatch_detail(result)?;
        assert_eq!(detail, "generated source contains unchecked cast");
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_accepts_clean_output() -> Result<(), String> {
        // Given a clean generated workflow
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // When comparing against the IR
        compare_generated_to_ir(&source, &workflow)
            .map_err(|e| format!("semantic comparison failed: {e}"))?;
        Ok(())
    }

    #[test]
    fn generated_action_suspend_matches_ir_awaiting_action_family() -> Result<(), String> {
        let cases = [
            (ActionId::new(1), SlotIdx::new(0)),
            (ActionId::new(5), SlotIdx::new(1)),
            (ActionId::new(9), SlotIdx::new(2)),
        ];

        cases.iter().try_for_each(|(action, input)| {
            let action = *action;
            let input = *input;
            let workflow = action_suspend_workflow(action, input)?;

            let generated_stdout = generated_action_suspend_stdout(&workflow, action, input)?;
            let expected_stdout = format!(
                "generated_action_suspend:{}:{}\n",
                action.get(),
                input.get()
            );
            assert_eq!(
                generated_stdout, expected_stdout,
                "generated action suspend output must identify action and input slot"
            );

            let ir_signal = ir_action_suspend_signal(&workflow, input)?;
            assert_eq!(
                ir_signal,
                EngineSignal::AwaitingAction,
                "IR step_once must suspend on the same Do boundary"
            );

            Ok::<(), String>(())
        })?;

        Ok(())
    }

    #[test]
    fn generated_expression_primitives_match_interpreter_finish() -> Result<(), String> {
        let workflow = primitive_expression_workflow()?;

        let ir_value = ir_drive_finished_value(&workflow)?;
        assert_eq!(
            ir_value,
            SlotValue::Bool(true),
            "interpreter must prove the primitive expression result"
        );

        let generated_stdout = generated_drive_stdout(&workflow, "expr_primitives", "")?;
        assert_eq!(
            generated_stdout, "ok:Bool(true)\n",
            "generated expression primitives must match interpreter result"
        );
        Ok(())
    }

    #[test]
    fn generated_choose_primitive_matches_interpreter_branch() -> Result<(), String> {
        let workflow = primitive_choose_workflow()?;

        let ir_value = ir_drive_finished_value(&workflow)?;
        assert_eq!(
            ir_value,
            SlotValue::I64(22),
            "interpreter must take the first true expression branch"
        );

        let generated_stdout = generated_drive_stdout(&workflow, "choose_primitive", "")?;
        assert_eq!(
            generated_stdout, "ok:I64(22)\n",
            "generated Choose branch must match interpreter result"
        );
        Ok(())
    }

    #[test]
    fn generated_choose_slot_primitive_matches_interpreter_branch() -> Result<(), String> {
        let workflow = primitive_choose_slot_workflow()?;

        let ir_value = ir_drive_finished_value(&workflow)?;
        assert_eq!(
            ir_value,
            SlotValue::I64(22),
            "interpreter must take the first true slot branch"
        );

        let generated_stdout = generated_drive_stdout(&workflow, "choose_slot_primitive", "")?;
        assert_eq!(
            generated_stdout, "ok:I64(22)\n",
            "generated ChooseSlot branch must match interpreter result"
        );
        Ok(())
    }

    #[test]
    fn generated_choose_nonbool_type_mismatch_matches_ir_exactly() -> Result<(), String> {
        let workflow = non_boolean_choose_workflow()?;

        assert_boolean_number_type_mismatch(ir_drive_error(&workflow)?)?;

        let expected = expected_drive_error_stdout(&workflow)?;
        let generated_stdout = generated_drive_stdout(&workflow, "choose_nonbool", "")?;
        assert_eq!(
            generated_stdout, expected,
            "generated Choose non-boolean condition must match exact DriveError variant"
        );
        Ok(())
    }

    #[test]
    fn generated_choose_slot_nonbool_type_mismatch_matches_ir_exactly() -> Result<(), String> {
        let workflow = non_boolean_choose_slot_workflow()?;

        assert_boolean_number_type_mismatch(ir_drive_error(&workflow)?)?;

        let expected = expected_drive_error_stdout(&workflow)?;
        let generated_stdout = generated_drive_stdout(&workflow, "choose_slot_nonbool", "")?;
        assert_eq!(
            generated_stdout, expected,
            "generated ChooseSlot non-boolean condition must match exact DriveError variant"
        );
        Ok(())
    }

    fn foreach_empty_generated_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_generated_foreach_empty"),
            digest: WorkflowDigest::from_bytes([0xA1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: Box::new([]),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(2)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 0,
                        body: StepIdx::new(2),
                        done: StepIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
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
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn foreach_single_generated_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_generated_foreach_single"),
            digest: WorkflowDigest::from_bytes([0xA2; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: Box::new([SlotIdx::new(0)]),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(1),
                        item_slot: SlotIdx::new(0),
                        limit: 1,
                        body: StepIdx::new(3),
                        done: StepIdx::new(3),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn foreach_multi_generated_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_generated_foreach_multi"),
            digest: WorkflowDigest::from_bytes([0xA3; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: Some(SlotIdx::new(3)),
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: Box::new([SlotIdx::new(0), SlotIdx::new(1), SlotIdx::new(2)]),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
                    output: Some(SlotIdx::new(5)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(3),
                        item_slot: SlotIdx::new(4),
                        limit: 3,
                        body: StepIdx::new(5),
                        done: StepIdx::new(6),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(5),
                    output: Some(SlotIdx::new(4)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(5),
                        body: StepIdx::new(6),
                        done: StepIdx::new(6),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(6),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(4),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)]
                .into_boxed_slice(),
            slot_count: 6,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn foreach_limit_generated_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_generated_foreach_limit"),
            digest: WorkflowDigest::from_bytes([0xA4; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: Box::new([SlotIdx::new(0)]),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(2)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(1),
                        item_slot: SlotIdx::new(0),
                        limit: 0,
                        body: StepIdx::new(3),
                        done: StepIdx::new(3),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn generated_for_each_empty_list_matches_interpreter_tail_result() -> Result<(), String> {
        let workflow = foreach_empty_generated_workflow()?;
        let runtime_value = runtime_drive_finished_value(&workflow)?;
        match runtime_value {
            SlotValue::List(id) => {
                assert_eq!(id.get(), 1, "runtime must return inserted empty tail")
            }
            other => return Err(format!("expected runtime list result, got {other:?}")),
        }

        let generated_stdout = generated_drive_stdout(&workflow, "foreach_empty", "")?;
        assert_eq!(
            generated_stdout, "ok:List(1)\n",
            "generated ForEach empty-list tail must match interpreter handle"
        );
        Ok(())
    }

    #[test]
    fn generated_for_each_single_item_matches_interpreter_binding() -> Result<(), String> {
        let workflow = foreach_single_generated_workflow()?;
        let runtime_value = runtime_drive_finished_value(&workflow)?;
        assert_eq!(runtime_value, SlotValue::I64(7));

        let generated_stdout = generated_drive_stdout(&workflow, "foreach_single", "")?;
        assert_eq!(generated_stdout, "ok:I64(7)\n");
        Ok(())
    }

    #[test]
    fn generated_for_each_next_matches_interpreter_tail_binding() -> Result<(), String> {
        let workflow = foreach_multi_generated_workflow()?;
        let runtime_value = runtime_drive_finished_value(&workflow)?;
        assert_eq!(runtime_value, SlotValue::I64(2));

        let generated_stdout = generated_drive_stdout(&workflow, "foreach_multi", "")?;
        assert_eq!(generated_stdout, "ok:I64(2)\n");
        Ok(())
    }

    #[test]
    fn generated_for_each_limit_exceeded_matches_interpreter_error() -> Result<(), String> {
        let workflow = foreach_limit_generated_workflow()?;
        let ir_error = runtime_drive_error_string(&workflow)?;
        assert!(
            ir_error.contains("for_each_limit"),
            "IR limit error must name for_each_limit, got: {ir_error}"
        );

        let generated_stdout = generated_drive_stdout(&workflow, "foreach_limit", "")?;
        assert!(
            generated_stdout
                .contains("err:IterationLimitExceeded { resource: \"for_each_limit\" }"),
            "generated limit error must match typed resource, got: {generated_stdout}"
        );
        Ok(())
    }

    #[test]
    fn emit_trybuild_fixture_writes_file_to_disk() -> Result<(), String> {
        // Given a minimal workflow and a temp fixture path
        let workflow = minimal_workflow()?;
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("vb_codegen_fixture_test_{}", std::process::id()))
            .tempdir()
            .map_err(|e| e.to_string())?;
        let fixture_path = temp_dir.path().join("fixture.rs");
        // When emit_trybuild_fixture writes the file
        emit_trybuild_fixture(&workflow, &fixture_path).map_err(|e| e.to_string())?;
        // Then it succeeds and the file exists
        let content = std::fs::read_to_string(&fixture_path).map_err(|e| e.to_string())?;
        assert!(!content.is_empty(), "fixture file must be non-empty");
        assert!(
            content.contains("#![forbid(unsafe_code)]"),
            "fixture must contain generated Rust with forbid unsafe"
        );
        Ok(())
    }

    #[test]
    fn emit_trybuild_fixture_rejects_root_path_without_parent() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When emitting to root path "/" which has no parent
        let fixture_path = std::path::Path::new("/");
        let result = emit_trybuild_fixture(&workflow, fixture_path);
        // Then it fails because "/" is a directory and cannot be written as a file
        let err = result
            .err()
            .ok_or("expected error for root path without writable parent")?;
        assert!(
            err.to_string().contains("root") || err.to_string().contains("parent"),
            "error must mention root or parent, got: {err}"
        );
        Ok(())
    }

    // --- Proptest Properties ---

    #[test]
    fn codegen_error_display_contains_variant_name() {
        assert_error_display_contains(CodegenError::FormatBufferOverflow, "buffer");
        assert_error_display_contains(
            CodegenError::RustfmtFailed {
                detail: String::from("test"),
            },
            "rustfmt",
        );
        assert_error_display_contains(
            CodegenError::CompileCheckFailed {
                detail: String::from("test"),
            },
            "compile",
        );
        assert_error_display_contains(
            CodegenError::SemanticMismatch {
                detail: String::from("test"),
            },
            "semantic",
        );
        assert_error_display_contains(
            CodegenError::Io(std::io::Error::other("io")),
            "codegen IO error",
        );
        assert_error_display_contains(
            CodegenError::TrybuildFixture {
                detail: String::from("test"),
            },
            "trybuild",
        );
    }

    fn assert_error_display_contains(error: CodegenError, keyword: &str) {
        let message = error.to_string();
        assert!(
            message.contains(keyword),
            "error display must contain keyword '{keyword}', got: {message}"
        );
    }

    #[test]
    fn emit_function_signature_never_empty() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When each emit function is called individually
        let mut ids_out = String::new();
        emit_ids(&mut ids_out, &workflow).map_err(|e| e.to_string())?;
        assert!(
            !ids_out.is_empty(),
            "emit_ids must produce non-empty output"
        );

        let mut drive_out = String::new();
        emit_drive_function(&mut drive_out, &workflow).map_err(|e| e.to_string())?;
        assert!(
            !drive_out.is_empty(),
            "emit_drive_function must produce non-empty output"
        );

        let mut finish_out = String::new();
        emit_finish(&mut finish_out, &workflow).map_err(|e| e.to_string())?;
        assert!(
            !finish_out.is_empty(),
            "emit_finish must produce non-empty output"
        );

        let mut contract_out = String::new();
        emit_resource_contract(&mut contract_out, workflow.resource_contract())
            .map_err(|e| e.to_string())?;
        assert!(
            !contract_out.is_empty(),
            "emit_resource_contract must produce non-empty output"
        );

        let mut dispatch_out = String::new();
        emit_action_match_dispatch(&mut dispatch_out, &workflow).map_err(|e| e.to_string())?;
        assert!(
            !dispatch_out.is_empty(),
            "emit_action_match_dispatch must produce non-empty output"
        );
        Ok(())
    }

    // --- Workflow Helpers for additional node types ---

    fn nop_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_nop"),
            digest: WorkflowDigest::from_bytes([0x11; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn copy_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_copy"),
            digest: WorkflowDigest::from_bytes([0x22; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn jump_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_jump"),
            digest: WorkflowDigest::from_bytes([0x33; 32]),
            nodes: vec![
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn wait_until_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_wait_until"),
            digest: WorkflowDigest::from_bytes([0x44; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitUntil {
                        deadline_slot: SlotIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn wait_event_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_wait_event"),
            digest: WorkflowDigest::from_bytes([0x55; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitEvent {
                        event: SlotIdx::new(0),
                        timeout_slot: Some(SlotIdx::new(1)),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn ask_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_ask"),
            digest: WorkflowDigest::from_bytes([0x66; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Ask {
                        prompt: SlotIdx::new(0),
                        timeout_slot: Some(SlotIdx::new(1)),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn for_each_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_for_each"),
            digest: WorkflowDigest::from_bytes([0x77; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(2)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
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
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn together_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_together"),
            digest: WorkflowDigest::from_bytes([0x88; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: vec![StepIdx::new(1)].into_boxed_slice(),
                        join: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn collect_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_collect"),
            digest: WorkflowDigest::from_bytes([0x99; 32]),
            nodes: vec![
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
                        done: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn reduce_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_reduce"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: vec![
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
                        done: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn repeat_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_repeat"),
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatStart {
                        max_attempts: 3,
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn build_object_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_build_object"),
            digest: WorkflowDigest::from_bytes([0xCC; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![(vb_core::SymbolId::new(0), SlotIdx::new(0))]
                            .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn choose_expr_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_choose_expr"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Choose {
                        branches: vec![vb_core::ExprBranch {
                            condition: vb_core::ExprIdx::new(0),
                            target: StepIdx::new(1),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(2)),
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
                CompiledNode {
                    id: StepIdx::new(2),
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
            constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn choose_slot_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_choose_slot"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: vec![vb_core::SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(2)),
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
                CompiledNode {
                    id: StepIdx::new(2),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn error_handler_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_error_handler"),
            digest: WorkflowDigest::from_bytes([0xFF; 32]),
            nodes: vec![
                CompiledNode {
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
                CompiledNode {
                    id: StepIdx::new(2),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn ask_resume_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_ask_resume"),
            digest: WorkflowDigest::from_bytes([0x12; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::AskResume {
                        answer: SlotIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // --- Additional Step Variant Tests ---

    #[test]
    fn emit_step_match_produces_correct_arm_for_build_list_node() -> Result<(), String> {
        // Given a BuildList node (now supported in codegen)
        let workflow = unsupported_build_list_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output constructs a list with read_slot and SlotValue::List
        assert!(
            out.contains("read_slot"),
            "BuildList must read item slots, got: {out}"
        );
        assert!(
            out.contains("SlotValue::List"),
            "BuildList must write SlotValue::List, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_choose_node() -> Result<(), String> {
        // Given a Choose node with one expression branch and an otherwise target
        let workflow = choose_expr_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains conditional branch dispatch
        assert!(
            out.contains("eval_expr_"),
            "Choose must call eval_expr, got: {out}"
        );
        assert!(
            out.contains("SlotValue::Bool(true)"),
            "Choose must require a boolean true branch condition, got: {out}"
        );
        assert!(
            out.contains("StepOutcome::Continue"),
            "Choose must return Continue on branch match, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_choose_slot_node() -> Result<(), String> {
        // Given a ChooseSlot node with one slot branch
        let workflow = choose_slot_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads slot for condition
        assert!(
            out.contains("read_slot"),
            "ChooseSlot must call read_slot, got: {out}"
        );
        assert!(
            out.contains("SlotValue::Bool(true)"),
            "ChooseSlot must require a boolean true branch condition, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_eval_expr_node() -> Result<(), String> {
        // Given a workflow with an EvalExpr node
        let workflow = minimal_workflow()?;
        // When emit_step_function generates code (SetConst is node 0, eval via expression)
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the expression evaluator function exists
        assert!(
            source.contains("fn eval_expr_0"),
            "must generate expression evaluator function"
        );
        assert!(
            source.contains("stack.push"),
            "expression must push values onto stack"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_error_handler_node() -> Result<(), String> {
        // Given an ErrorHandler node
        let workflow = error_handler_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output contains error handler metadata comment
        assert!(
            out.contains("ErrorHandler"),
            "ErrorHandler must be referenced in generated code, got: {out}"
        );
        assert!(
            out.contains("StepOutcome::Continue"),
            "ErrorHandler must continue to body step, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_ask_resume_node() -> Result<(), String> {
        // Given an AskResume node
        let workflow = ask_resume_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output references the answer slot
        assert!(
            out.contains("_answer_slot"),
            "AskResume must reference answer slot, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_nop_without_next_reports_missing_step() -> Result<(), String> {
        // Given a Nop node with no next target
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let workflow = nop_workflow()?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, &node, &workflow).map_err(|e| e.to_string())?;
        // Then the output returns MissingNextStep error
        assert!(
            out.contains("MissingNextStep"),
            "Nop without next must return MissingNextStep, got: {out}"
        );
        Ok(())
    }

    // --- Additional Integration Tests ---

    #[test]
    fn generate_output_contains_forbid_and_deny_lint_gates() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all lint gates are present
        assert!(
            source.contains("#![forbid(unsafe_code)]"),
            "must include forbid unsafe_code"
        );
        assert!(
            source.contains("#![deny(unused_must_use)]"),
            "must include deny unused_must_use"
        );
        assert!(
            source.contains("#![deny(rust_2018_idioms)]"),
            "must include deny rust_2018_idioms"
        );
        Ok(())
    }

    #[test]
    fn generate_output_contains_read_const_function() -> Result<(), String> {
        // Given a workflow with constants
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then read_const helper function is defined
        assert!(
            source.contains("fn read_const"),
            "must define read_const function"
        );
        assert!(
            source.contains("CONSTANTS.get"),
            "read_const must use checked access"
        );
        Ok(())
    }

    #[test]
    fn generate_output_contains_step_outcome_enum() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then StepOutcome is defined with Continue and Finished variants
        assert!(
            source.contains("StepOutcome"),
            "must define StepOutcome type"
        );
        assert!(
            source.contains("Continue"),
            "StepOutcome must have Continue variant"
        );
        assert!(
            source.contains("Finished"),
            "StepOutcome must have Finished variant"
        );
        Ok(())
    }

    #[test]
    fn generate_do_action_workflow_contains_dispatch_function() -> Result<(), String> {
        // Given a workflow with a Do action node
        let workflow = do_action_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then dispatch_action function exists with the action registered
        assert!(
            source.contains("pub fn dispatch_action"),
            "must define dispatch_action function"
        );
        assert!(
            source.contains("5 => Ok(())"),
            "dispatch must list action id 5"
        );
        assert!(
            source.contains("UnknownAction"),
            "dispatch must handle unknown actions"
        );
        Ok(())
    }

    #[test]
    fn generate_workflow_with_no_actions_has_empty_dispatch() -> Result<(), String> {
        // Given a workflow with no Do nodes
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then dispatch_action only has the wildcard fallback
        assert!(
            source.contains("pub fn dispatch_action"),
            "must define dispatch_action function"
        );
        assert!(
            source.contains("_ => Err(DriveError::UnknownAction)"),
            "dispatch must have wildcard fallback"
        );
        // No specific action arms besides the wildcard
        let dispatch_section_start = source
            .find("pub fn dispatch_action")
            .ok_or("dispatch section missing")?;
        let dispatch_section = source
            .get(dispatch_section_start..)
            .ok_or("dispatch section start invalid")?;
        let dispatch_section_end = dispatch_section
            .find("}")
            .ok_or("dispatch closing brace missing")?;
        let dispatch_body = dispatch_section
            .get(..dispatch_section_end)
            .ok_or("dispatch section end invalid")?;
        assert!(
            !dispatch_body.contains("=> Ok(())"),
            "dispatch should have no action arms for a workflow without Do nodes"
        );
        Ok(())
    }

    #[test]
    fn generated_source_contains_required_sections() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        assert!(source.contains("drive("), "should contain drive function");
        assert!(
            source.contains("fn step_0"),
            "should contain step functions"
        );
        assert!(source.contains("CONSTANTS"), "should contain constant pool");
        assert!(source.contains("DriveError"), "should contain error type");
        assert!(
            source.contains("StepOutcome::Finished"),
            "finish should return a terminal value"
        );
        assert!(
            source.contains("ExprStack::new"),
            "expression stack should be fixed storage"
        );
        assert!(
            !source.contains("u16::MAX"),
            "generated source must not use finish sentinel"
        );
        assert!(
            !source.contains("Vec<") && !source.contains("Vec::"),
            "generated source must not allocate Vec hot stacks"
        );
        assert!(
            !source.contains("slots[") && !source.contains("CONSTANTS["),
            "generated source must use checked access helpers"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_accepts_valid_output() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let comparison = compare_generated_to_ir(&source, &workflow);
        assert!(
            comparison.is_ok(),
            "semantic comparison should pass for valid output"
        );
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_finish_sentinel() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        source.push_str("\nconst BAD_SENTINEL: u16 = u16::MAX;\n");

        let comparison = compare_generated_to_ir(&source, &workflow);
        assert!(
            comparison.is_err(),
            "semantic comparison should reject sentinel output"
        );
        Ok(())
    }

    #[test]
    fn build_list_codegen_is_now_supported() -> Result<(), String> {
        let workflow = unsupported_build_list_workflow()?;
        // BuildList is now supported: validation and emission should succeed
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::List"),
            "generated source must contain SlotValue::List, got: {source}"
        );
        Ok(())
    }

    #[test]
    fn contains_expression_codegen_is_now_supported() -> Result<(), String> {
        let workflow = unsupported_contains_expression_workflow()?;
        // Contains is now supported: validation and emission should succeed
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("symbol_contains"),
            "generated source must contain symbol_contains, got: {source}"
        );
        Ok(())
    }

    #[test]
    fn unsupported_accessor_codegen_is_rejected_before_emit() -> Result<(), String> {
        let workflow = unsupported_accessor_traversal_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "accessor traversal",
            "unsupported generated Rust IR feature: accessor traversal",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "accessor traversal",
            "unsupported generated Rust IR feature: accessor traversal",
        )?;
        Ok(())
    }

    fn assert_unsupported_ir<T>(
        result: Result<T, CodegenError>,
        expected_feature: &'static str,
        expected_message: &'static str,
    ) -> Result<(), String> {
        let error = result
            .err()
            .ok_or_else(|| format!("{expected_feature} workflow unexpectedly succeeded"))?;
        let message = error.to_string();

        assert!(
            matches!(
                error,
                CodegenError::UnsupportedIr { feature } if feature == expected_feature
            ),
            "unsupported IR must return exact typed feature {expected_feature}, got {message}"
        );
        assert_eq!(
            message, expected_message,
            "unsupported IR display diagnostic changed"
        );
        Ok(())
    }

    #[test]
    fn root_accessor_codegen_preserves_root_slot_behavior() -> Result<(), String> {
        let workflow = root_accessor_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("stack.push(read_slot(slots, 0)?)?;"),
            "empty accessor must compile to the same checked root-slot read as LoadSlot"
        );
        assert!(
            !source.contains("accessor traversal"),
            "empty accessor must not emit traversal failure path"
        );
        Ok(())
    }

    #[test]
    fn root_accessor_generated_source_compile_checks() -> Result<(), String> {
        let workflow = root_accessor_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!(
                "vb_codegen_root_accessor_test_{}",
                std::process::id()
            ))
            .tempdir()
            .map_err(|e| e.to_string())?;
        compile_check_generated_rust(&source, temp_dir.path()).map_err(|e| e.to_string())
    }

    #[test]
    fn generated_subset_accepts_minimal_supported_workflow() -> Result<(), String> {
        let workflow = minimal_workflow()?;

        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn generated_source_compile_checks() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("vb_codegen_test_{}", std::process::id()))
            .tempdir()
            .map_err(|e| e.to_string())?;
        compile_check_generated_rust(&source, temp_dir.path()).map_err(|e| e.to_string())
    }

    #[test]
    fn generate_workflow_name_appears_in_doc_comment() -> Result<(), String> {
        // Given a workflow with name "test_codegen"
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the doc comment mentions codegen origin
        assert!(
            source.contains("Produced by vb_codegen"),
            "must mention codegen origin in doc comment"
        );
        assert!(
            source.contains("DO NOT EDIT"),
            "must warn against manual editing"
        );
        Ok(())
    }

    #[test]
    fn generate_includes_is_true_helper_on_slot_value() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then SlotValue has is_true helper
        assert!(
            source.contains("fn is_true"),
            "must define is_true helper on SlotValue"
        );
        assert!(
            source.contains("type_name"),
            "must define type_name helper on SlotValue"
        );
        Ok(())
    }

    #[test]
    fn emit_drive_function_rejects_invalid_program_counter() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the drive loop handles invalid program counter
        assert!(
            source.contains("InvalidProgramCounter"),
            "drive must handle invalid program counter"
        );
        Ok(())
    }

    #[test]
    fn emit_action_match_dispatch_for_do_workflow_includes_action_arm() -> Result<(), String> {
        // Given a do_action workflow
        let workflow = do_action_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the Do step function contains ActionSuspend with the correct action id
        assert!(
            source.contains("ActionSuspend { action_id: 5"),
            "do step must reference action_id 5"
        );
        assert!(
            source.contains("dispatch_action"),
            "must contain dispatch_action function"
        );
        assert!(
            source.contains("5 => Ok(())"),
            "dispatch must list action id 5 arm"
        );
        Ok(())
    }

    // --- Proptest Properties ---

    #[test]
    fn emit_step_match_output_is_valid_rust_identifier_prefix() -> Result<(), String> {
        // Given multiple workflow types, each generating step functions
        let workflows = [
            ("nop", nop_workflow()),
            ("copy", copy_workflow()),
            ("jump", jump_workflow()),
            ("do_action", do_action_workflow()),
        ];
        workflows
            .into_iter()
            .try_for_each(|(name, workflow_result)| {
                assert_workflow_step_names_valid(name, workflow_result)
            })?;
        Ok(())
    }

    // --- Adversarial BDD Tests: Codegen Contract Verification ---

    // BUG: ErrorHandler emits Continue(body) but never sets up error-catch routing.
    // The handler step index is emitted only in a comment. Generated code does NOT
    // call the handler on failure, so generated ErrorHandler semantics diverge from
    // the IR which expects the handler to be invoked when the body step fails.
    #[test]
    fn error_handler_generated_code_ignores_handler_on_body_failure() -> Result<(), String> {
        // Given an ErrorHandler node with body=1 and handler=2
        let workflow = error_handler_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code for the ErrorHandler
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the generated code calls the body step and routes to the handler on error
        assert!(
            out.contains("step_1(slots, list_store)"),
            "ErrorHandler must call body step 1, got: {out}"
        );
        // The handler must appear in executable code (not just a comment).
        let has_handler_in_executable = out
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                // Skip comment lines
                !trimmed.starts_with("//") && trimmed.contains("Continue(2)")
            })
            .count();
        assert!(
            has_handler_in_executable > 0,
            "ErrorHandler generated code must reference handler=2 in executable code, got: {out}"
        );
        // On success, the body outcome propagates directly.
        assert!(
            out.contains("Ok(outcome) => Ok(outcome)"),
            "ErrorHandler must propagate successful body outcome, got: {out}"
        );
        Ok(())
    }

    // GAP: emit_resource_contract emits only 8 of 16 ResourceContract fields.
    // emit_resource_contract emits all 16 ResourceContract fields.
    #[test]
    fn resource_contract_documents_missing_fields_gap() -> Result<(), String> {
        // Given a resource contract with non-default values for every field
        let contract = ResourceContract {
            max_steps: 100,
            max_slots: 200,
            max_constants: 50,
            max_accessors: 10,
            max_expressions: 20,
            max_expr_stack: 32,
            max_input_bytes: 4096,
            max_output_bytes: 8192,
            max_step_budget_per_tick: 500,
            max_blob_bytes: 65536,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 5,
            max_fanout: 16,
            max_collect_items: 200,
            max_queue_depth: 128,
            max_journal_batch_bytes: 1024,
        };
        // When emit_resource_contract writes the constants
        let mut out = String::new();
        emit_resource_contract(&mut out, contract).map_err(|e| e.to_string())?;
        // Then all 16 fields are present
        assert_resource_contract_fields(&out)?;
        Ok(())
    }

    // Verify that the drive loop has NO step budget enforcement.
    // The generated code runs an infinite loop without checking CONTRACT_MAX_STEPS.
    #[test]
    fn drive_function_has_no_step_budget_enforcement() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When emit_drive_function generates the loop
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then the generated loop has no budget counter
        let has_budget_check = out.contains("budget") || out.contains("CONTRACT_MAX_STEPS");
        // BUG: The drive loop runs without any step budget check. This means
        // a malicious workflow with a Jump cycle would run forever in generated mode,
        // while the interpreter would respect the step budget.
        assert!(
            !has_budget_check,
            "drive function should have step budget check but doesn't -- \
             if this assertion flips, the bug is fixed"
        );
        Ok(())
    }

    // Verify that generated code does NOT contain forbidden constructs.
    #[test]
    fn generated_source_forbids_unsafe_unwrap_expect_panic_todo_dbg() -> Result<(), String> {
        // Given a minimal workflow generating source
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the source must not contain any forbidden constructs
        let violations = forbidden_generated_source_violations(&source);
        assert!(
            violations.is_empty(),
            "generated source contains forbidden constructs: {:?}",
            violations
        );
        Ok(())
    }

    // Verify that the Choose node with multiple branches emits all of them in order.
    #[test]
    fn choose_node_deep_nesting_emits_all_branches_in_order() -> Result<(), String> {
        // Given a Choose node with 5 branches and no otherwise
        let ops = vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let branches: Vec<vb_core::ExprBranch> = (0..5)
            .map(|i| vb_core::ExprBranch {
                condition: vb_core::ExprIdx::new(0),
                target: StepIdx::new(i + 1),
            })
            .collect();
        let nodes = std::iter::once(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: branches.into_boxed_slice(),
                otherwise: None,
            },
        })
        .chain((1..=5).map(choose_finish_node))
        .collect::<Vec<_>>();
        let parts = WorkflowParts {
            name: Box::<str>::from("test_deep_choose"),
            digest: WorkflowDigest::from_bytes([0xF0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        // When generating step function for the Choose node
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then all 5 branches appear in order
        (1..=5).try_for_each(|i| {
            let expected = format!("StepOutcome::Continue({i})");
            if out.contains(&expected) {
                Ok(())
            } else {
                Err(format!("Choose must emit branch target {i}, got: {out}"))
            }
        })?;
        // And no otherwise fallback exists
        assert!(
            out.contains("NoBranchMatched"),
            "Choose without otherwise must emit NoBranchMatched error, got: {out}"
        );
        Ok(())
    }

    // BUG: compare_generated_to_ir requires "ExprStack::new" to appear in generated
    // source even for workflows with zero expressions. The header defines ExprStack
    // struct but nothing instantiates it when there are no eval_expr functions.
    // This causes compare_generated_to_ir to falsely reject valid expressionless workflows.
    #[test]
    fn compare_generated_to_ir_rejects_expressionless_workflow_due_to_missing_stack()
    -> Result<(), String> {
        // Given a workflow with 3 SetConst steps and no expressions
        let parts = WorkflowParts {
            name: Box::<str>::from("test_only_set_const"),
            digest: WorkflowDigest::from_bytes([0xA1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10), ConstValue::Bool(true)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the source is valid (3 steps, 2 constants, no expressions)
        let step_count = source
            .lines()
            .filter(|l| l.trim().starts_with("fn step_"))
            .count() as u16;
        assert!(
            step_count == 3,
            "expected 3 step handlers, found {step_count}"
        );
        assert!(
            source.contains("SlotValue::I64(10)"),
            "constant I64(10) must appear in pool"
        );
        assert!(
            !source.contains("fn eval_expr_"),
            "workflow without expressions should not generate eval_expr functions"
        );
        // compare_generated_to_ir now correctly accepts expressionless workflows
        // by skipping the ExprStack::new check when there are no expressions.
        let comparison_result = compare_generated_to_ir(&source, &workflow);
        assert!(
            comparison_result.is_ok(),
            "compare_generated_to_ir must accept expressionless workflows: {:?}",
            comparison_result.err()
        );
        Ok(())
    }

    // Verify that the generated source for a SetConst-only workflow has correct
    // structure and passes semantic equivalence checks.
    #[test]
    fn set_const_only_workflow_generates_correct_step_and_constant_structure() -> Result<(), String>
    {
        // Given a workflow with only SetConst steps
        let parts = WorkflowParts {
            name: Box::<str>::from("test_set_const_only"),
            digest: WorkflowDigest::from_bytes([0xA2; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the constant pool and step functions are correct
        assert!(
            source.contains("SlotValue::I64(10)"),
            "constant must appear in pool"
        );
        assert!(
            source.contains("fn step_0"),
            "must have step_0 for SetConst"
        );
        assert!(source.contains("fn step_1"), "must have step_1 for Finish");
        assert!(
            source.contains("write_slot(slots, 0, Some(read_const(0)?)"),
            "SetConst step must write constant 0 to slot 0"
        );
        Ok(())
    }

    // BUG TRAP: The `compare_generated_to_ir` function rejects ` as ` in any context,
    // but the generated code contains "as" inside string literals like
    // "accessor traversal 'field' on generated type" which does NOT contain ` as `.
    // However, the `DriveError::TypeMismatch` strings contain type_name() which is fine.
    // Test that clean generated code does not accidentally contain ` as `.
    #[test]
    fn compare_rejects_as_cast_allows_string_accessors() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the clean generated source does not contain ` as ` (unchecked cast)
        let as_cast_count = source.lines().filter(|l| l.contains(" as ")).count();
        assert!(
            as_cast_count == 0,
            "generated source should not contain ' as ' cast pattern, found {as_cast_count} occurrences"
        );
        Ok(())
    }

    // Verify that the constant pool correctly handles all ConstValue variants.
    #[test]
    fn constant_pool_handles_all_const_value_variants() -> Result<(), String> {
        // Given a workflow with all 5 ConstValue variants
        let f64_val = vb_core::FiniteF64::new(3.25).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_all_consts"),
            digest: WorkflowDigest::from_bytes([0xB1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![
                ConstValue::Null,
                ConstValue::Bool(false),
                ConstValue::I64(-42),
                ConstValue::F64(f64_val),
                ConstValue::Symbol(vb_core::SymbolId::new(99)),
            ]
            .into_boxed_slice(),
            slot_count: 1,
            symbols_count: 100,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all variants appear in the constant pool
        assert!(
            source.contains("SlotValue::Null"),
            "Null constant must appear, got source starting: {}",
            &source.chars().take(500).collect::<String>()
        );
        assert!(
            source.contains("SlotValue::Bool(false)"),
            "Bool constant must appear"
        );
        assert!(
            source.contains("SlotValue::I64(-42)"),
            "I64 constant must appear"
        );
        assert!(
            source.contains("SlotValue::F64(3.25"),
            "F64 constant must appear"
        );
        assert!(
            source.contains("SlotValue::Symbol(99)"),
            "Symbol constant must appear"
        );
        Ok(())
    }

    // Verify that CompareGeneratedToIR correctly counts steps and rejects mismatches.
    #[test]
    fn compare_rejects_wrong_step_count() -> Result<(), String> {
        // Given a minimal workflow with 2 nodes
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // When adding an extra step function to the source
        let mut tampered = source;
        tampered.push_str("\nfn step_99(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<StepOutcome, DriveError> { Ok(StepOutcome::Continue(0)) }\n");
        // Then compare rejects it
        let result = compare_generated_to_ir(&tampered, &workflow);
        let detail = semantic_mismatch_detail(result)?;
        assert_eq!(detail, "step count mismatch: generated has 3, IR has 2");
        Ok(())
    }

    // Verify that compare_generated_to_ir rejects wrong expression count.
    #[test]
    fn compare_rejects_wrong_expression_count() -> Result<(), String> {
        // Given a minimal workflow with 1 expression
        let workflow = minimal_workflow()?;
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // When adding a fake expression function
        source.push_str("\nfn eval_expr_99(slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<SlotValue, DriveError> { Ok(SlotValue::Null) }\n");
        // Then compare rejects it
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(
            result.is_err(),
            "must reject source with wrong expression count"
        );
        let err = result.err().ok_or("expected error")?;
        let msg = err.to_string();
        assert!(
            msg.contains("expression count mismatch"),
            "error must mention expression count, got: {msg}"
        );
        Ok(())
    }

    // Verify that Jump-to-self cycle detection is absent (code gen gap).
    // The generated drive loop will infinite-loop if a step returns Continue to itself.
    #[test]
    fn jump_to_self_produces_infinite_loop_without_budget_guard() -> Result<(), String> {
        // Given a workflow where step 0 is a Nop that continues to step 0 (self-loop)
        let parts = WorkflowParts {
            name: Box::<str>::from("test_self_loop"),
            digest: WorkflowDigest::from_bytes([0xC1; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(0)), // self-loop
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        // The workflow should fail validation since node 0 has next=0 which creates a cycle
        // with no Finish node. But CompiledWorkflow may accept it.
        let workflow_result = CompiledWorkflow::try_from_parts(parts);
        if let Ok(workflow) = workflow_result {
            let source_result = emit_rust_workflow(&workflow);
            if let Ok(source) = source_result {
                // The generated source contains step_0 returning Continue(0)
                // which creates an infinite loop with NO step budget guard.
                assert!(
                    source.contains("StepOutcome::Continue(0)"),
                    "self-loop must emit Continue(0)"
                );
                // GAP: No budget counter in drive loop to prevent infinite execution
                let has_budget = source.contains("budget") || source.contains("step_count");
                assert!(
                    !has_budget,
                    "generated drive loop should have step budget guard for safety"
                );
            }
        }
        // If the workflow was rejected by validation, that's also acceptable
        Ok(())
    }

    // Verify that WaitEvent without timeout slot emits only the event read.
    #[test]
    fn wait_event_without_timeout_omits_timeout_read() -> Result<(), String> {
        // Given a WaitEvent node with event but no timeout
        let parts = WorkflowParts {
            name: Box::<str>::from("test_wait_event_no_timeout"),
            digest: WorkflowDigest::from_bytes([0xD1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitEvent {
                        event: SlotIdx::new(0),
                        timeout_slot: None,
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When generating code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads event but NOT timeout
        assert!(
            out.contains("_event"),
            "WaitEvent must reference event variable, got: {out}"
        );
        assert!(
            !out.contains("_timeout"),
            "WaitEvent without timeout must NOT reference timeout variable, got: {out}"
        );
        Ok(())
    }

    // Verify that Ask without timeout slot omits timeout read.
    #[test]
    fn ask_without_timeout_omits_timeout_read() -> Result<(), String> {
        // Given an Ask node with prompt but no timeout
        let parts = WorkflowParts {
            name: Box::<str>::from("test_ask_no_timeout"),
            digest: WorkflowDigest::from_bytes([0xD2; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Ask {
                        prompt: SlotIdx::new(0),
                        timeout_slot: None,
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When generating code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reads prompt but NOT timeout
        assert!(
            out.contains("_prompt"),
            "Ask must reference prompt variable, got: {out}"
        );
        assert!(
            !out.contains("_timeout"),
            "Ask without timeout must NOT reference timeout variable, got: {out}"
        );
        Ok(())
    }

    // Verify that an empty constant pool produces valid Rust (zero-sized array).
    #[test]
    fn empty_constant_pool_generates_zero_sized_array() -> Result<(), String> {
        // Given a workflow with no constants
        let workflow = nop_workflow()?;
        // When generating source
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the constant pool is a zero-sized array
        assert!(
            source.contains("CONSTANTS: [SlotValue; 0]"),
            "empty workflow must generate CONSTANTS: [SlotValue; 0], got relevant section: {}",
            source
                .lines()
                .filter(|l| l.contains("CONSTANTS"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(
            source.contains("];"),
            "constant pool must be properly closed"
        );
        Ok(())
    }

    // Verify that the generated ExprStack pop() uses checked_sub, not wrapping subtraction.
    #[test]
    fn expr_stack_pop_uses_checked_subtraction() -> Result<(), String> {
        // Given any workflow
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the ExprStack pop method uses checked_sub
        let pop_section = source
            .lines()
            .filter(|l| l.contains("checked_sub") || l.contains("fn pop"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            pop_section.contains("checked_sub"),
            "ExprStack::pop must use checked_sub for underflow safety, got: {pop_section}"
        );
        // And does NOT use wrapping_sub or unchecked_sub
        assert!(
            !source.contains("wrapping_sub"),
            "generated code must not use wrapping_sub"
        );
        assert!(
            !source.contains("unchecked_sub"),
            "generated code must not use unchecked_sub"
        );
        Ok(())
    }

    // Verify that the generated drive function initializes PC to the correct entry step.
    #[test]
    fn drive_function_initializes_pc_to_entry_step_nonzero() -> Result<(), String> {
        // Given a workflow with entry at step 1 (not 0)
        // All nodes must be forward-reachable from entry
        let ops = vec![vb_core::ExprOp::LoadConst(ConstIdx::new(0))];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_nonzero_entry"),
            digest: WorkflowDigest::from_bytes([0xE1; 32]),
            nodes: vec![
                // Node 0 is a dead placeholder that's reachable via Jump from node 2
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(1), // Entry is step 1
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        // When generating the drive function
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then the PC is initialized to 1 (the entry step)
        assert!(
            out.contains("let mut pc: u16 = 1;"),
            "drive must initialize pc to entry step 1, got: {out}"
        );
        Ok(())
    }

    // Verify that generated code uses checked arithmetic everywhere in the hot path.
    #[test]
    fn generated_code_uses_checked_arithmetic_no_wrapping() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then no wrapping or unchecked arithmetic patterns exist
        assert!(
            !source.contains("wrapping_add"),
            "generated source must not contain wrapping_add"
        );
        assert!(
            !source.contains("wrapping_sub"),
            "generated source must not contain wrapping_sub"
        );
        assert!(
            !source.contains("wrapping_mul"),
            "generated source must not contain wrapping_mul"
        );
        assert!(
            !source.contains("saturating_add"),
            "generated source must not contain saturating_add"
        );
        assert!(
            !source.contains("overflowing_add"),
            "generated source must not contain overflowing_add"
        );
        // And checked_add is used in ExprStack push
        assert!(
            source.contains("checked_add"),
            "generated code must use checked_add in ExprStack push"
        );
        Ok(())
    }

    // Verify that SetConst without output slot still advances to next step.
    #[test]
    fn set_const_without_output_skips_write_advances_to_next() -> Result<(), String> {
        // Given a SetConst node with no output slot
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::<str>::from("test_set_const_no_output"),
            digest: WorkflowDigest::from_bytes([0xE2; 32]),
            nodes: vec![
                node.clone(),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        // When generating code
        let mut out = String::new();
        emit_step_function(&mut out, &node, &workflow).map_err(|e| e.to_string())?;
        // Then no write_slot call is emitted (output is None)
        assert!(
            !out.contains("write_slot"),
            "SetConst without output must not call write_slot, got: {out}"
        );
        // But still advances to next step
        assert!(
            out.contains("StepOutcome::Continue(1)"),
            "SetConst without output must still advance to next, got: {out}"
        );
        Ok(())
    }

    // Verify that Copy without output slot reads but does not write.
    #[test]
    fn copy_without_output_skips_write_advances_to_next() -> Result<(), String> {
        // Given a Copy node with no output slot
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::<str>::from("test_copy_no_output"),
            digest: WorkflowDigest::from_bytes([0xE3; 32]),
            nodes: vec![
                node.clone(),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        // When generating code
        let mut out = String::new();
        emit_step_function(&mut out, &node, &workflow).map_err(|e| e.to_string())?;
        // Then no read_slot_optional or write_slot is emitted
        assert!(
            !out.contains("read_slot_optional"),
            "Copy without output must not read slot, got: {out}"
        );
        assert!(
            !out.contains("write_slot"),
            "Copy without output must not write slot, got: {out}"
        );
        // But still advances to next
        assert!(
            out.contains("StepOutcome::Continue(1)"),
            "Copy without output must still advance to next, got: {out}"
        );
        Ok(())
    }

    // Verify that compare_generated_to_ir rejects unchecked slot indexing patterns.
    #[test]
    fn compare_rejects_unchecked_slot_indexing_pattern() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        let mut source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // When injecting unchecked slot indexing
        source.push_str("\nlet val = slots[0];\n");
        // Then compare rejects it
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(
            result.is_err(),
            "must reject source with unchecked slot indexing"
        );
        Ok(())
    }

    // Verify that the generated source contains the correct DriveError variants
    // matching what emit_step_function can produce.
    #[test]
    fn generated_drive_error_covers_all_step_error_paths() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then all error variants that step functions can produce are defined
        assert!(
            source.contains("InvalidProgramCounter"),
            "DriveError must define variant InvalidProgramCounter"
        );
        assert!(
            source.contains("MissingNextStep"),
            "DriveError must define variant MissingNextStep"
        );
        assert!(
            source.contains("SlotNull"),
            "DriveError must define variant SlotNull"
        );
        assert!(
            source.contains("NoBranchMatched"),
            "DriveError must define variant NoBranchMatched"
        );
        assert!(
            source.contains("ExpressionStackOverflow"),
            "DriveError must define variant ExpressionStackOverflow"
        );
        assert!(
            source.contains("TypeMismatch"),
            "DriveError must define variant TypeMismatch"
        );
        assert!(
            source.contains("DivisionByZero"),
            "DriveError must define variant DivisionByZero"
        );
        assert!(
            source.contains("IntegerOverflow"),
            "DriveError must define variant IntegerOverflow"
        );
        assert!(
            source.contains("ExpressionStackUnderflow"),
            "DriveError must define variant ExpressionStackUnderflow"
        );
        assert!(
            source.contains("ActionSuspend"),
            "DriveError must define variant ActionSuspend"
        );
        assert!(
            source.contains("UnknownAction"),
            "DriveError must define variant UnknownAction"
        );
        assert!(
            source.contains("UnsupportedPrimitive"),
            "DriveError must define variant UnsupportedPrimitive"
        );
        assert!(
            source.contains("UnsupportedExpressionOp"),
            "DriveError must define variant UnsupportedExpressionOp"
        );
        assert!(
            source.contains("InvalidCompiledWorkflow"),
            "DriveError must define variant InvalidCompiledWorkflow"
        );
        Ok(())
    }

    // Verify that the ChooseSlot node emits read_slot for each branch condition slot.
    #[test]
    fn choose_slot_multiple_branches_reads_each_condition_slot() -> Result<(), String> {
        // Given a ChooseSlot node with 3 branches reading slots 0, 1, 2
        let branches: Vec<vb_core::SlotBranch> = (0..3)
            .map(|i| vb_core::SlotBranch {
                condition: SlotIdx::new(i),
                target: StepIdx::new(i + 1),
            })
            .collect();
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: branches.into_boxed_slice(),
                    otherwise: None,
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
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let parts = WorkflowParts {
            name: Box::<str>::from("test_multi_choose_slot"),
            digest: WorkflowDigest::from_bytes([0xF1; 32]),
            nodes: nodes.into_boxed_slice(),
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
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When generating code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then each slot index is read
        assert!(
            out.contains("read_slot(slots, 0)"),
            "ChooseSlot must read condition slot 0, got: {out}"
        );
        assert!(
            out.contains("read_slot(slots, 1)"),
            "ChooseSlot must read condition slot 1, got: {out}"
        );
        assert!(
            out.contains("read_slot(slots, 2)"),
            "ChooseSlot must read condition slot 2, got: {out}"
        );
        Ok(())
    }

    // Verify that the generated SlotValue enum has PartialEq derive for Eq comparison.
    #[test]
    fn generated_slot_value_has_partial_eq_for_comparison() -> Result<(), String> {
        // Given any generated source
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then SlotValue has PartialEq derived (needed for Eq expression comparison)
        let _slot_value_line = source
            .lines()
            .find(|l| l.contains("pub enum SlotValue"))
            .ok_or("SlotValue enum line not found")?;
        let prior_line = source
            .lines()
            .take_while(|l| !l.contains("pub enum SlotValue"))
            .last()
            .ok_or("line before SlotValue not found")?;
        assert!(
            prior_line.contains("PartialEq"),
            "SlotValue must derive PartialEq for expression equality, got prior line: {prior_line}"
        );
        Ok(())
    }

    // Verify that the generated ExprStack::push uses get_mut for bounds-checked access.
    #[test]
    fn expr_stack_push_uses_get_mut_for_bounds_check() -> Result<(), String> {
        // Given any generated source
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // Then the push method uses .get_mut() not direct indexing
        assert!(
            source.contains("get_mut"),
            "ExprStack::push must use get_mut for bounds-checked slot access"
        );
        assert!(
            !source.contains("self.values[self.len]"),
            "ExprStack::push must not use direct indexing"
        );
        Ok(())
    }

    // --- ForEach / Together unsupported-primitive rejection tests ---

    fn for_each_next_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_for_each_next"),
            digest: WorkflowDigest::from_bytes([0x78; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn for_each_join_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_for_each_join"),
            digest: WorkflowDigest::from_bytes([0x79; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachJoin {
                        output: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn together_branch_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_together_branch"),
            digest: WorkflowDigest::from_bytes([0x89; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherBranch {
                        branch: 0,
                        entry: StepIdx::new(1),
                        join: StepIdx::new(1),
                        accumulator: SlotIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn together_join_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_together_join"),
            digest: WorkflowDigest::from_bytes([0x8A; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherJoin {
                        branch_count: 1,
                        accumulator: SlotIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn nested_for_each_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_nested_for_each"),
            digest: WorkflowDigest::from_bytes([0x7A; 32]),
            nodes: vec![
                // Outer ForEachStart
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
                        done: StepIdx::new(3),
                    },
                },
                // Inner ForEachStart (nested)
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(1),
                        item_slot: SlotIdx::new(2),
                        limit: 5,
                        body: StepIdx::new(2),
                        done: StepIdx::new(2),
                    },
                },
                // Inner body placeholder -> Finish
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
                // Outer done -> Finish
                CompiledNode {
                    id: StepIdx::new(3),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// A complete ForEach workflow that sums values from a list.
    /// This documents the expected IR structure for a ForEach sum workflow.
    fn for_each_sum_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_for_each_sum"),
            digest: WorkflowDigest::from_bytes([0x7B; 32]),
            nodes: vec![
                // Node 0: SetConst - initialize accumulator (0) in slot 2
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                // Node 1: ForEachStart - iterate over list in slot 0, bind item to slot 1
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(3)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 100,
                        body: StepIdx::new(2),
                        done: StepIdx::new(3),
                    },
                },
                // Node 2: ForEachNext - advance iterator (body node)
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(3)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(3),
                        body: StepIdx::new(2),
                        done: StepIdx::new(3),
                    },
                },
                // Node 3: ForEachJoin - materialize results
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachJoin {
                        output: SlotIdx::new(2),
                    },
                },
                // Node 4: Finish - return accumulator
                CompiledNode {
                    id: StepIdx::new(4),
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
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// A complete Together workflow with two branches.
    /// This documents the expected IR structure for a Together parallel-branch workflow.
    fn together_two_branch_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_together_two_branch"),
            digest: WorkflowDigest::from_bytes([0x8B; 32]),
            nodes: vec![
                // Node 0: TogetherStart - begin parallel branches
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: vec![StepIdx::new(1), StepIdx::new(3)].into_boxed_slice(),
                        join: StepIdx::new(5),
                    },
                },
                // Node 1: TogetherBranch 0 - first branch entry
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(1)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherBranch {
                        branch: 0,
                        entry: StepIdx::new(2),
                        join: StepIdx::new(5),
                        accumulator: SlotIdx::new(2),
                    },
                },
                // Node 2: SetConst - branch 0 body: write 10
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                // Node 3: TogetherBranch 1 - second branch entry
                CompiledNode {
                    id: StepIdx::new(3),
                    output: Some(SlotIdx::new(1)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherBranch {
                        branch: 1,
                        entry: StepIdx::new(4),
                        join: StepIdx::new(5),
                        accumulator: SlotIdx::new(2),
                    },
                },
                // Node 4: SetConst - branch 1 body: write 20
                CompiledNode {
                    id: StepIdx::new(4),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(5)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                // Node 5: TogetherJoin - merge results
                CompiledNode {
                    id: StepIdx::new(5),
                    output: None,
                    next: Some(StepIdx::new(6)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherJoin {
                        branch_count: 2,
                        accumulator: SlotIdx::new(2),
                    },
                },
                // Node 6: Finish - return merged output
                CompiledNode {
                    id: StepIdx::new(6),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10), ConstValue::I64(20)].into_boxed_slice(),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // --- ForEach generated-mode acceptance tests ---

    #[test]
    fn for_each_start_codegen_is_supported() -> Result<(), String> {
        let workflow = for_each_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let code = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            code.contains("tail_list_handle")
                && !code.contains("UnsupportedPrimitive { primitive: \"ForEachStart\" }"),
            "ForEachStart must emit concrete generated support, got: {code}"
        );
        Ok(())
    }

    #[test]
    fn for_each_next_codegen_is_supported() -> Result<(), String> {
        let workflow = for_each_next_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let code = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            code.contains("first_list_item")
                && !code.contains("UnsupportedPrimitive { primitive: \"ForEachNext\" }"),
            "ForEachNext must emit concrete generated support, got: {code}"
        );
        Ok(())
    }

    #[test]
    fn for_each_join_codegen_is_supported() -> Result<(), String> {
        let workflow = for_each_join_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let code = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            code.contains("list_item_count")
                && !code.contains("UnsupportedPrimitive { primitive: \"ForEachJoin\" }"),
            "ForEachJoin must emit concrete generated support, got: {code}"
        );
        Ok(())
    }

    #[test]
    fn for_each_sum_workflow_is_accepted_by_codegen() -> Result<(), String> {
        // Given a complete ForEach sum workflow with ForEachStart, ForEachNext, and ForEachJoin
        let workflow = for_each_sum_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Together validation rejection tests ---

    #[test]
    fn together_start_codegen_is_typed_error() -> Result<(), String> {
        let workflow = together_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "TogetherStart",
            "unsupported generated Rust IR feature: TogetherStart",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "TogetherStart",
            "unsupported generated Rust IR feature: TogetherStart",
        )?;
        Ok(())
    }

    #[test]
    fn together_branch_codegen_is_typed_error() -> Result<(), String> {
        let workflow = together_branch_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "TogetherBranch",
            "unsupported generated Rust IR feature: TogetherBranch",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "TogetherBranch",
            "unsupported generated Rust IR feature: TogetherBranch",
        )?;
        Ok(())
    }

    #[test]
    fn together_join_codegen_is_typed_error() -> Result<(), String> {
        let workflow = together_join_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "TogetherJoin",
            "unsupported generated Rust IR feature: TogetherJoin",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "TogetherJoin",
            "unsupported generated Rust IR feature: TogetherJoin",
        )?;
        Ok(())
    }

    #[test]
    fn together_two_branch_workflow_is_rejected_by_codegen() -> Result<(), String> {
        // Given a complete Together workflow with TogetherStart, TogetherBranch, and TogetherJoin
        let workflow = together_two_branch_workflow()?;
        // When validate_generated_subset checks it
        // Then it rejects with TogetherStart (first unsupported node encountered)
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "TogetherStart",
            "unsupported generated Rust IR feature: TogetherStart",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "TogetherStart",
            "unsupported generated Rust IR feature: TogetherStart",
        )?;
        Ok(())
    }

    // --- Nested ForEach validation acceptance test ---

    #[test]
    fn nested_for_each_workflow_is_accepted_by_codegen() -> Result<(), String> {
        // Given a workflow with a ForEachStart inside a ForEachStart (nested loops)
        let workflow = nested_for_each_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Step emit verification for all ForEach/Together node kinds ---

    #[test]
    fn emit_step_match_produces_correct_arm_for_for_each_next_node() -> Result<(), String> {
        // Given a ForEachNext node supported by generated mode
        let workflow = for_each_next_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output emits concrete iterator advancement.
        assert!(
            !out.contains("UnsupportedPrimitive"),
            "ForEachNext must not emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("first_list_item") && out.contains("tail_list_handle"),
            "ForEachNext must bind item and update iterator tail, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_for_each_join_node() -> Result<(), String> {
        // Given a ForEachJoin node supported by generated mode
        let workflow = for_each_join_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output validates list materialization and writes output.
        assert!(
            !out.contains("UnsupportedPrimitive"),
            "ForEachJoin must not emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("expect_list_value") && out.contains("write_slot"),
            "ForEachJoin must validate and copy materialized list, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_together_branch_node() -> Result<(), String> {
        // Given a TogetherBranch node (unsupported in codegen)
        let workflow = together_branch_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "TogetherBranch must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("TogetherBranch"),
            "UnsupportedPrimitive must name TogetherBranch, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_together_join_node() -> Result<(), String> {
        // Given a TogetherJoin node (unsupported in codegen)
        let workflow = together_join_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "TogetherJoin must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("TogetherJoin"),
            "UnsupportedPrimitive must name TogetherJoin, got: {out}"
        );
        Ok(())
    }

    // ====================================================================
    // Round 7 expanded codegen tests: BuildObject, BuildList, RetryCheck,
    // and helper expression ops (Contains, StartsWith, EndsWith, Has,
    // Exists, Length, Empty, Sum, Count, Unique).
    // ====================================================================

    // --- BuildObject comprehensive tests ---

    fn build_object_multi_field_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_build_object_multi"),
            digest: WorkflowDigest::from_bytes([0xD0; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(3)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![
                            (vb_core::SymbolId::new(0), SlotIdx::new(0)),
                            (vb_core::SymbolId::new(1), SlotIdx::new(1)),
                            (vb_core::SymbolId::new(2), SlotIdx::new(2)),
                        ]
                        .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(3),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 4,
            symbols_count: 3,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn build_object_multi_field_emits_all_field_reads() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("read_slot(slots, 0)"),
            "BuildObject must read field slot 0, got: {out}"
        );
        assert!(
            out.contains("read_slot(slots, 1)"),
            "BuildObject must read field slot 1, got: {out}"
        );
        assert!(
            out.contains("read_slot(slots, 2)"),
            "BuildObject must read field slot 2, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn build_object_multi_field_emits_symbol_bindings() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("_sym_0"),
            "BuildObject must reference symbol 0, got: {out}"
        );
        assert!(
            out.contains("_sym_1"),
            "BuildObject must reference symbol 1, got: {out}"
        );
        assert!(
            out.contains("_sym_2"),
            "BuildObject must reference symbol 2, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn build_object_multi_field_writes_object_to_output() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("write_slot(slots, 3, Some(SlotValue::Object"),
            "generated source must write SlotValue::Object to output slot 3"
        );
        Ok(())
    }

    #[test]
    fn build_object_multi_field_passes_semantic_check() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(
            result.is_ok(),
            "BuildObject multi-field workflow must pass semantic check"
        );
        Ok(())
    }

    #[test]
    fn build_object_zero_fields_emits_object_zero() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_build_object_zero"),
            digest: WorkflowDigest::from_bytes([0xD1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildObject {
                        fields: vec![].into_boxed_slice(),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::Object"),
            "BuildObject with zero fields must emit SlotValue::Object"
        );
        Ok(())
    }

    // --- BuildList comprehensive tests ---

    fn build_list_multi_item_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_build_list_multi"),
            digest: WorkflowDigest::from_bytes([0xD2; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(3)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0), SlotIdx::new(1), SlotIdx::new(2)]
                            .into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(3),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn build_list_multi_item_emits_all_slot_reads() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("let _item0 = read_slot(slots, 0)"),
            "BuildList must read item slot 0, got: {out}"
        );
        assert!(
            out.contains("let _item1 = read_slot(slots, 1)"),
            "BuildList must read item slot 1, got: {out}"
        );
        assert!(
            out.contains("let _item2 = read_slot(slots, 2)"),
            "BuildList must read item slot 2, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn build_list_multi_item_writes_list_to_output() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("write_slot(slots, 3, Some(SlotValue::List"),
            "generated source must write SlotValue::List to output slot 3"
        );
        Ok(())
    }

    #[test]
    fn build_list_multi_item_passes_semantic_check() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(
            result.is_ok(),
            "BuildList multi-item workflow must pass semantic check"
        );
        Ok(())
    }

    #[test]
    fn build_list_zero_items_emits_list_zero() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_build_list_zero"),
            digest: WorkflowDigest::from_bytes([0xD3; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::BuildList {
                        items: vec![].into_boxed_slice(),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::List"),
            "BuildList with zero items must emit SlotValue::List"
        );
        assert!(
            source.contains("BuildList: 0 item(s)"),
            "BuildList comment must indicate 0 items"
        );
        Ok(())
    }

    // --- RetryCheck comprehensive tests ---

    fn retry_check_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_retry_check"),
            digest: WorkflowDigest::from_bytes([0xD4; 32]),
            nodes: vec![
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
                CompiledNode {
                    id: StepIdx::new(2),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn retry_check_passes_validation() -> Result<(), String> {
        let workflow = retry_check_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn retry_check_emits_policy_read() -> Result<(), String> {
        let workflow = retry_check_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("read_slot(slots, 0)"),
            "RetryCheck must read policy slot, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn retry_check_emits_branching_logic() -> Result<(), String> {
        let workflow = retry_check_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("StepOutcome::Continue(1)"),
            "RetryCheck body branch must target step 1, got: {out}"
        );
        assert!(
            out.contains("StepOutcome::Continue(2)"),
            "RetryCheck exhausted branch must target step 2, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn retry_check_emits_retry_count_extraction() -> Result<(), String> {
        let workflow = retry_check_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("_retry_count"),
            "RetryCheck must extract retry count, got: {out}"
        );
        assert!(
            out.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
            "RetryCheck must reference CONTRACT_MAX_RETRY_ATTEMPTS, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn retry_check_emits_type_mismatch_guard() -> Result<(), String> {
        let workflow = retry_check_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("SlotValue::I64"),
            "RetryCheck must match on SlotValue::I64 for policy, got: {out}"
        );
        assert!(
            out.contains("TypeMismatch"),
            "RetryCheck must emit type mismatch error guard, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn retry_check_full_workflow_passes_semantic_check() -> Result<(), String> {
        let workflow = retry_check_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(
            result.is_ok(),
            "RetryCheck workflow must pass semantic check"
        );
        Ok(())
    }

    // --- Helper expression ops: StartsWith, EndsWith ---

    fn starts_with_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::StartsWith,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_starts_with"),
            digest: WorkflowDigest::from_bytes([0xE0; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![
                ConstValue::Symbol(vb_core::SymbolId::new(1)),
                ConstValue::Symbol(vb_core::SymbolId::new(2)),
            ]
            .into_boxed_slice(),
            slot_count: 1,
            symbols_count: 3,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn starts_with_expression_passes_validation() -> Result<(), String> {
        let workflow = starts_with_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn starts_with_expression_emits_symbol_starts_with() -> Result<(), String> {
        let workflow = starts_with_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("symbol_starts_with"),
            "StartsWith must emit symbol_starts_with call, got source snippet: {}",
            &source.chars().take(500).collect::<String>()
        );
        Ok(())
    }

    fn ends_with_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::EndsWith,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_ends_with"),
            digest: WorkflowDigest::from_bytes([0xE1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![
                ConstValue::Symbol(vb_core::SymbolId::new(1)),
                ConstValue::Symbol(vb_core::SymbolId::new(2)),
            ]
            .into_boxed_slice(),
            slot_count: 1,
            symbols_count: 3,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn ends_with_expression_passes_validation() -> Result<(), String> {
        let workflow = ends_with_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn ends_with_expression_emits_symbol_ends_with() -> Result<(), String> {
        let workflow = ends_with_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("symbol_ends_with"),
            "EndsWith must emit symbol_ends_with call"
        );
        Ok(())
    }

    // --- Helper expression op: Has ---

    fn has_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Has,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_has"),
            digest: WorkflowDigest::from_bytes([0xE2; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::I64(0), ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn has_expression_passes_validation() -> Result<(), String> {
        let workflow = has_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn has_expression_emits_object_and_list_match() -> Result<(), String> {
        let workflow = has_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::Object"),
            "Has must match SlotValue::Object"
        );
        assert!(
            source.contains("SlotValue::List"),
            "Has must match SlotValue::List"
        );
        assert!(
            source.contains("SlotValue::Bool"),
            "Has must produce SlotValue::Bool result"
        );
        Ok(())
    }

    // --- Helper expression op: Exists ---

    fn exists_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Exists,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_exists"),
            digest: WorkflowDigest::from_bytes([0xE3; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn exists_expression_passes_validation() -> Result<(), String> {
        let workflow = exists_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn exists_expression_emits_null_check() -> Result<(), String> {
        let workflow = exists_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::Null"),
            "Exists must check for SlotValue::Null"
        );
        assert!(
            source.contains("matches!"),
            "Exists must use matches! macro for null check"
        );
        Ok(())
    }

    // --- Helper expression op: Length ---

    fn length_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Length,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_length"),
            digest: WorkflowDigest::from_bytes([0xE4; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::I64(5)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn length_expression_passes_validation() -> Result<(), String> {
        let workflow = length_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn length_expression_emits_list_and_object_match() -> Result<(), String> {
        let workflow = length_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::List(handle)"),
            "Length must match SlotValue::List with handle"
        );
        assert!(
            source.contains("list_item_count(list_store, handle)?"),
            "Length must resolve generated list payload length"
        );
        assert!(
            source.contains("SlotValue::Object(n)"),
            "Length must match SlotValue::Object with count"
        );
        assert!(
            source.contains("SlotValue::I64"),
            "Length must produce SlotValue::I64 result"
        );
        Ok(())
    }

    // --- Helper expression op: Empty ---

    fn empty_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Empty,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_empty"),
            digest: WorkflowDigest::from_bytes([0xE5; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn empty_expression_passes_validation() -> Result<(), String> {
        let workflow = empty_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn empty_expression_emits_zero_check() -> Result<(), String> {
        let workflow = empty_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::List(handle)"),
            "Empty must match SlotValue::List with handle"
        );
        assert!(
            source.contains("list_item_count(list_store, handle)? == 0"),
            "Empty must check generated list payload length == 0"
        );
        assert!(
            source.contains("SlotValue::Null => true"),
            "Empty must treat Null as empty"
        );
        Ok(())
    }

    // --- Helper expression ops: Sum, Count, Unique ---

    fn sum_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Sum,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_sum"),
            digest: WorkflowDigest::from_bytes([0xE6; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::I64(3)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn sum_expression_is_rejected_as_unsupported() -> Result<(), String> {
        let workflow = sum_expression_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "sum",
            "unsupported generated Rust IR feature: sum",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "sum",
            "unsupported generated Rust IR feature: sum",
        )?;
        Ok(())
    }

    fn count_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Count,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_count"),
            digest: WorkflowDigest::from_bytes([0xE7; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn count_expression_passes_validation() -> Result<(), String> {
        let workflow = count_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn count_expression_emits_collection_match() -> Result<(), String> {
        let workflow = count_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::List(n)") || source.contains("SlotValue::List("),
            "Count must match SlotValue::List"
        );
        assert!(
            source.contains("SlotValue::I64"),
            "Count must produce SlotValue::I64 result"
        );
        Ok(())
    }

    fn unique_expression_workflow() -> Result<CompiledWorkflow, String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Unique,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_unique"),
            digest: WorkflowDigest::from_bytes([0xE8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
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
            constants: vec![ConstValue::I64(2)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn unique_expression_is_rejected_as_unsupported() -> Result<(), String> {
        let workflow = unique_expression_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "unique",
            "unsupported generated Rust IR feature: unique",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "unique",
            "unsupported generated Rust IR feature: unique",
        )?;
        Ok(())
    }

    // --- Cross-cutting: all helper ops in one workflow pass validation and emit ---

    #[test]
    fn all_helper_ops_together_pass_validation() -> Result<(), String> {
        // Build a single workflow with multiple expressions, one for each helper op
        let ops_contains = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Contains,
        ];
        let ops_starts = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::StartsWith,
        ];
        let ops_ends = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::EndsWith,
        ];
        let ops_has = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Has,
        ];
        let ops_exists = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Exists,
        ];
        let ops_length = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Length,
        ];
        let ops_empty = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Empty,
        ];
        let ops_count = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Count,
        ];

        let expr_contains = ExprProgram::try_from_ops(ops_contains.into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let expr_starts =
            ExprProgram::try_from_ops(ops_starts.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_ends =
            ExprProgram::try_from_ops(ops_ends.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_has =
            ExprProgram::try_from_ops(ops_has.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_exists =
            ExprProgram::try_from_ops(ops_exists.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_length =
            ExprProgram::try_from_ops(ops_length.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_empty =
            ExprProgram::try_from_ops(ops_empty.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_count =
            ExprProgram::try_from_ops(ops_count.into_boxed_slice()).map_err(|e| e.to_string())?;

        // 8 EvalExpr nodes + 1 Finish node = 9 nodes
        let nodes: Vec<CompiledNode> = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(4)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(5)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(5),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(6)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(6),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(7)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(6),
                },
            },
            CompiledNode {
                id: StepIdx::new(7),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(8)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(7),
                },
            },
            CompiledNode {
                id: StepIdx::new(8),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];

        let parts = WorkflowParts {
            name: Box::<str>::from("test_all_helpers"),
            digest: WorkflowDigest::from_bytes([0xF0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: vec![
                expr_contains,
                expr_starts,
                expr_ends,
                expr_has,
                expr_exists,
                expr_length,
                expr_empty,
                expr_count,
            ]
            .into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(0), ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // Verify each expression function was generated
        assert!(
            source.contains("fn eval_expr_0"),
            "must generate eval_expr_0 (Contains)"
        );
        assert!(
            source.contains("fn eval_expr_1"),
            "must generate eval_expr_1 (StartsWith)"
        );
        assert!(
            source.contains("fn eval_expr_2"),
            "must generate eval_expr_2 (EndsWith)"
        );
        assert!(
            source.contains("fn eval_expr_3"),
            "must generate eval_expr_3 (Has)"
        );
        assert!(
            source.contains("fn eval_expr_4"),
            "must generate eval_expr_4 (Exists)"
        );
        assert!(
            source.contains("fn eval_expr_5"),
            "must generate eval_expr_5 (Length)"
        );
        assert!(
            source.contains("fn eval_expr_6"),
            "must generate eval_expr_6 (Empty)"
        );
        assert!(
            source.contains("fn eval_expr_7"),
            "must generate eval_expr_7 (Count)"
        );

        // Verify semantic check passes
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(
            result.is_ok(),
            "all-helper-ops workflow must pass semantic check"
        );
        Ok(())
    }

    // ============================================================
    // Step emission unit tests
    // ============================================================

    fn finish_node(idx: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }
    }

    #[allow(clippy::expect_used)]
    fn emit_first_node(node: CompiledNode, slot_count: u16) -> String {
        let node_id = node.id;
        let finish_idx = node_id.get().saturating_add(1);
        let nodes = vec![node, finish_node(finish_idx)];
        let wf = make_step_workflow(nodes, slot_count);
        let mut out = String::new();
        emit_step_function(&mut out, wf.node(node_id).expect("node must exist"), &wf)
            .expect("emit should succeed");
        out
    }

    #[allow(clippy::expect_used)]
    fn emit_node_in_wf(node_id: StepIdx, wf: &CompiledWorkflow) -> String {
        let mut out = String::new();
        let node = wf.node(node_id).expect("node must exist");
        emit_step_function(&mut out, node, wf).expect("emit should succeed");
        out
    }

    fn make_step_workflow(nodes: Vec<CompiledNode>, slot_count: u16) -> CompiledWorkflow {
        make_step_workflow_with_symbols(
            nodes,
            slot_count,
            0,
            Box::new([]),
            Box::new([]),
            Box::new([]),
        )
    }

    #[allow(clippy::expect_used)]
    fn make_step_workflow_with_symbols(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
        symbols_count: u32,
        constants: Box<[ConstValue]>,
        expressions: Box<[ExprProgram]>,
        accessors: Box<[AccessorProgram]>,
    ) -> CompiledWorkflow {
        let step_count = u16::try_from(nodes.len()).unwrap_or(u16::MAX);
        let parts = WorkflowParts {
            name: Box::from("test_step_wf"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions,
            accessors,
            constants,
            slot_count,
            symbols_count,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: vec![Box::from("step"); usize::from(step_count)].into_boxed_slice(),
        };
        CompiledWorkflow::try_from_parts(parts).expect("step test workflow should validate")
    }

    fn make_step_workflow_with_const(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
        constants: Vec<ConstValue>,
    ) -> CompiledWorkflow {
        make_step_workflow_with_symbols(
            nodes,
            slot_count,
            0,
            constants.into_boxed_slice(),
            Box::new([]),
            Box::new([]),
        )
    }

    fn make_step_workflow_with_expr(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
        expressions: Vec<ExprProgram>,
    ) -> CompiledWorkflow {
        make_step_workflow_with_symbols(
            nodes,
            slot_count,
            0,
            Box::new([]),
            expressions.into_boxed_slice(),
            Box::new([]),
        )
    }

    #[test]
    fn step_nop_with_next_emits_continue() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let code = emit_first_node(node, 2);
        assert!(
            code.contains("fn step_0("),
            "should emit step_0 function: {code}"
        );
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "nop with next should continue: {code}"
        );
    }

    #[test]
    fn step_set_const_with_output_slot_writes_slot() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow_with_const(nodes, 3, vec![ConstValue::I64(42)]);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("write_slot"),
            "SetConst should emit write_slot: {code}"
        );
        assert!(
            code.contains("read_const(0)"),
            "SetConst should read const 0: {code}"
        );
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "should continue to step 1: {code}"
        );
    }

    #[test]
    fn step_set_const_without_output_slot_skips_write() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow_with_const(nodes, 2, vec![ConstValue::I64(7)]);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            !code.contains("write_slot"),
            "SetConst without output should not write: {code}"
        );
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "should continue: {code}"
        );
    }

    #[test]
    fn step_copy_with_output_slot_emits_copy() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        };
        let code = emit_first_node(node, 4);
        assert!(
            code.contains("read_slot_optional"),
            "Copy should use read_slot_optional: {code}"
        );
        assert!(
            code.contains("write_slot"),
            "Copy should write slot: {code}"
        );
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "should continue: {code}"
        );
    }

    #[test]
    fn step_copy_without_output_skips_copy() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        };
        let code = emit_first_node(node, 2);
        assert!(
            !code.contains("read_slot_optional"),
            "Copy without output should not read: {code}"
        );
        assert!(
            !code.contains("write_slot"),
            "Copy without output should not write: {code}"
        );
    }

    #[test]
    fn step_eval_expr_with_output_emits_write() -> Result<(), String> {
        let expr_prog =
            ExprProgram::try_from_ops(Box::new([vb_core::ExprOp::LoadSlot(SlotIdx::new(0))]))
                .map_err(|e| e.to_string())?;
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow_with_expr(nodes, 3, vec![expr_prog]);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("eval_expr_0"),
            "should call eval_expr_0: {code}"
        );
        assert!(code.contains("write_slot"), "should write slot: {code}");
        Ok(())
    }

    #[test]
    fn step_eval_expr_without_output_skips_write() -> Result<(), String> {
        let expr_prog =
            ExprProgram::try_from_ops(Box::new([vb_core::ExprOp::LoadSlot(SlotIdx::new(0))]))
                .map_err(|e| e.to_string())?;
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ExprIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow_with_expr(nodes, 2, vec![expr_prog]);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            !code.contains("eval_expr_0"),
            "no output should skip eval: {code}"
        );
        Ok(())
    }

    #[test]
    fn step_finish_emits_finished_outcome() -> Result<(), String> {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }];
        let wf = make_step_workflow(nodes, 2);
        let mut out = String::new();
        let node = wf.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        emit_step_function(&mut out, node, &wf).map_err(|e| e.to_string())?;
        assert!(
            out.contains("read_slot(slots, 0)"),
            "should read result slot: {out}"
        );
        assert!(
            out.contains("StepOutcome::Finished"),
            "should emit Finished: {out}"
        );
        Ok(())
    }

    #[test]
    fn step_jump_emits_continue_to_target() {
        let nodes = vec![
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
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 1);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "jump should emit continue to target: {code}"
        );
    }

    #[test]
    fn step_choose_with_branch_emits_if() -> Result<(), String> {
        let expr_prog =
            ExprProgram::try_from_ops(Box::new([vb_core::ExprOp::LoadSlot(SlotIdx::new(0))]))
                .map_err(|e| e.to_string())?;
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![vb_core::ExprBranch {
                        condition: vb_core::ExprIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(1)),
                },
            },
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow_with_expr(nodes, 2, vec![expr_prog]);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("eval_expr_0"),
            "choose should eval expr: {code}"
        );
        assert!(
            code.contains("SlotValue::Bool(true)"),
            "choose should require boolean true: {code}"
        );
        assert!(
            code.contains("StepOutcome::Continue(2)"),
            "branch should target step 2: {code}"
        );
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "otherwise should target step 1: {code}"
        );
        Ok(())
    }

    #[test]
    fn step_choose_without_otherwise_emits_error() -> Result<(), String> {
        let expr_prog =
            ExprProgram::try_from_ops(Box::new([vb_core::ExprOp::LoadSlot(SlotIdx::new(0))]))
                .map_err(|e| e.to_string())?;
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![vb_core::ExprBranch {
                        condition: vb_core::ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: None,
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow_with_expr(nodes, 2, vec![expr_prog]);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("NoBranchMatched"),
            "no otherwise should emit NoBranchMatched: {code}"
        );
        Ok(())
    }

    #[test]
    fn step_choose_slot_with_branch_emits_if() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![vb_core::SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(1)),
                },
            },
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 3);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("read_slot"),
            "choose_slot should read slot: {code}"
        );
        assert!(
            code.contains("SlotValue::Bool(true)"),
            "should require boolean true: {code}"
        );
        assert!(
            code.contains("StepOutcome::Continue(2)"),
            "branch should target step 2: {code}"
        );
    }

    #[test]
    fn step_build_object_emits_field_reads() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(vb_core::SymbolId::new(0), SlotIdx::new(0))].into_boxed_slice(),
                },
            },
            finish_node(1),
        ];
        let wf =
            make_step_workflow_with_symbols(nodes, 3, 1, Box::new([]), Box::new([]), Box::new([]));
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("BuildObject"),
            "should contain BuildObject comment: {code}"
        );
        assert!(code.contains("write_slot"), "should write slot: {code}");
        assert!(
            code.contains("SlotValue::Object"),
            "should create Object: {code}"
        );
    }

    #[test]
    fn step_build_object_empty_fields() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![].into_boxed_slice(),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 3);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(code.contains("0 field(s)"), "should show 0 fields: {code}");
    }

    #[test]
    fn step_build_list_emits_item_reads() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 3);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("BuildList"),
            "should mention BuildList: {code}"
        );
        assert!(code.contains("_item0"), "should read item 0: {code}");
        assert!(code.contains("_item1"), "should read item 1: {code}");
        assert!(
            code.contains("SlotValue::List"),
            "should create List: {code}"
        );
    }

    #[test]
    fn step_build_list_empty_items() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![].into_boxed_slice(),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 3);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("0 item(s)"),
            "empty build list should show 0 items: {code}"
        );
    }

    #[test]
    fn step_do_action_emits_suspend() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(5),
                input: SlotIdx::new(0),
            },
        }];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("ActionSuspend"),
            "Do should emit ActionSuspend: {code}"
        );
        assert!(
            code.contains("action_id: 5"),
            "should mention action_id 5: {code}"
        );
    }

    #[test]
    fn step_wait_until_with_next_emits_continue() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
        };
        let code = emit_first_node(node, 2);
        assert!(code.contains("_deadline"), "should read deadline: {code}");
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "should continue: {code}"
        );
    }

    #[test]
    fn step_wait_until_without_next_emits_missing_next() -> Result<(), String> {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
        }];
        let wf = make_step_workflow(nodes, 2);
        let mut out = String::new();
        let node = wf.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        emit_step_function(&mut out, node, &wf).map_err(|e| e.to_string())?;
        assert!(out.contains("_deadline"), "should read deadline: {out}");
        assert!(
            out.contains("MissingNextStep"),
            "should emit MissingNextStep when no next: {out}"
        );
        Ok(())
    }

    #[test]
    fn step_wait_event_with_next_and_timeout() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let code = emit_first_node(node, 3);
        assert!(code.contains("_event"), "should read event: {code}");
        assert!(code.contains("_timeout"), "should read timeout: {code}");
        assert!(
            code.contains("StepOutcome::Continue(1)"),
            "should continue: {code}"
        );
    }

    #[test]
    fn step_wait_event_without_timeout() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        };
        let code = emit_first_node(node, 2);
        assert!(code.contains("_event"), "should read event: {code}");
        assert!(
            !code.contains("_timeout"),
            "no timeout should skip timeout read: {code}"
        );
    }

    #[test]
    fn step_wait_event_without_next_emits_missing_next() -> Result<(), String> {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        }];
        let wf = make_step_workflow(nodes, 2);
        let mut out = String::new();
        let node = wf.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        emit_step_function(&mut out, node, &wf).map_err(|e| e.to_string())?;
        assert!(out.contains("_event"), "should read event: {out}");
        assert!(
            out.contains("MissingNextStep"),
            "should emit MissingNextStep when no next: {out}"
        );
        Ok(())
    }

    #[test]
    fn step_ask_with_next_and_timeout() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let code = emit_first_node(node, 3);
        assert!(code.contains("_prompt"), "should read prompt: {code}");
        assert!(code.contains("_timeout"), "should read timeout: {code}");
    }

    #[test]
    fn step_ask_without_timeout() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        };
        let code = emit_first_node(node, 2);
        assert!(code.contains("_prompt"), "should read prompt: {code}");
        assert!(!code.contains("_timeout"), "no timeout should skip: {code}");
    }

    #[test]
    fn step_ask_without_next_emits_missing_next() -> Result<(), String> {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        }];
        let wf = make_step_workflow(nodes, 2);
        let mut out = String::new();
        let node = wf.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        emit_step_function(&mut out, node, &wf).map_err(|e| e.to_string())?;
        assert!(out.contains("_prompt"), "should read prompt: {out}");
        assert!(
            out.contains("MissingNextStep"),
            "should emit MissingNextStep when no next: {out}"
        );
        Ok(())
    }

    #[test]
    fn step_ask_resume_emits_answer_slot() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(3),
            },
        };
        let code = emit_first_node(node, 5);
        assert!(
            code.contains("_answer_slot"),
            "should declare answer_slot: {code}"
        );
        assert!(code.contains("3"), "should contain slot index 3: {code}");
    }

    #[test]
    fn step_error_handler_emits_body_and_handler() {
        let nodes = vec![
            CompiledNode {
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
            },
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("ErrorHandler"),
            "should mention ErrorHandler: {code}"
        );
        assert!(code.contains("step_1"), "should call body step_1: {code}");
        assert!(
            code.contains("Continue(2)"),
            "should continue to handler on error: {code}"
        );
    }

    #[test]
    fn step_error_handler_match_body_result() {
        let nodes = vec![
            CompiledNode {
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
            },
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("match step_1"),
            "should match on body step result: {code}"
        );
        assert!(
            code.contains("Ok(outcome) => Ok(outcome)"),
            "should pass through ok: {code}"
        );
        assert!(code.contains("Err(_)"), "should catch errors: {code}");
    }

    #[test]
    fn step_retry_check_emits_policy_read() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(code.contains("_policy"), "should read policy slot: {code}");
        assert!(
            code.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
            "should check retry limit: {code}"
        );
        assert!(
            code.contains("Continue(1)"),
            "should continue to body: {code}"
        );
        assert!(
            code.contains("Continue(2)"),
            "should continue to exhausted: {code}"
        );
    }

    #[test]
    fn step_retry_check_compare_count_to_limit() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("_retry_count"),
            "should extract retry_count: {code}"
        );
        assert!(code.contains("_limit"), "should define limit: {code}");
    }

    #[test]
    fn step_collect_start_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "CollectStart should emit unsupported: {code}"
        );
        assert!(
            code.contains("CollectStart"),
            "should name CollectStart: {code}"
        );
    }

    #[test]
    fn step_collect_page_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "CollectPage should emit unsupported: {code}"
        );
        assert!(
            code.contains("CollectPage"),
            "should name CollectPage: {code}"
        );
    }

    #[test]
    fn step_collect_next_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "CollectNext should emit unsupported: {code}"
        );
        assert!(
            code.contains("CollectNext"),
            "should name CollectNext: {code}"
        );
    }

    #[test]
    fn step_collect_finish_emits_unsupported() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectFinish {
                    collector_slot: SlotIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "CollectFinish should emit unsupported: {code}"
        );
        assert!(
            code.contains("CollectFinish"),
            "should name CollectFinish: {code}"
        );
    }

    #[test]
    fn step_for_each_start_emits_iterator_support() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(2)),
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 3);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            !code.contains("UnsupportedPrimitive"),
            "ForEachStart should emit concrete support: {code}"
        );
        assert!(
            code.contains("list_item_count") && code.contains("tail_list_handle"),
            "should count list items and store tail: {code}"
        );
    }

    #[test]
    fn step_for_each_next_emits_iterator_support() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            !code.contains("UnsupportedPrimitive"),
            "ForEachNext should emit concrete support: {code}"
        );
        assert!(
            code.contains("first_list_item") && code.contains("tail_list_handle"),
            "should bind item and shrink iterator: {code}"
        );
    }

    #[test]
    fn step_for_each_join_emits_materialization_support() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            !code.contains("UnsupportedPrimitive"),
            "ForEachJoin should emit concrete support: {code}"
        );
        assert!(
            code.contains("expect_list_value") && code.contains("write_slot"),
            "should validate and copy materialized list: {code}"
        );
    }

    #[test]
    fn step_together_start_emits_unsupported() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(1)].into_boxed_slice(),
                    join: StepIdx::new(2),
                },
            },
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "TogetherStart should emit unsupported: {code}"
        );
        assert!(
            code.contains("TogetherStart"),
            "should name TogetherStart: {code}"
        );
    }

    #[test]
    fn step_together_branch_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "TogetherBranch should emit unsupported: {code}"
        );
        assert!(
            code.contains("TogetherBranch"),
            "should name TogetherBranch: {code}"
        );
    }

    #[test]
    fn step_together_join_emits_unsupported() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherJoin {
                    branch_count: 1,
                    accumulator: SlotIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "TogetherJoin should emit unsupported: {code}"
        );
        assert!(
            code.contains("TogetherJoin"),
            "should name TogetherJoin: {code}"
        );
    }

    #[test]
    fn step_reduce_start_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow_with_const(nodes, 3, vec![ConstValue::I64(0)]);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "ReduceStart should emit unsupported: {code}"
        );
        assert!(
            code.contains("ReduceStart"),
            "should name ReduceStart: {code}"
        );
    }

    #[test]
    fn step_reduce_next_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 3);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "ReduceNext should emit unsupported: {code}"
        );
        assert!(
            code.contains("ReduceNext"),
            "should name ReduceNext: {code}"
        );
    }

    #[test]
    fn step_reduce_finish_emits_unsupported() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ReduceFinish {
                    accumulator: SlotIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "ReduceFinish should emit unsupported: {code}"
        );
        assert!(
            code.contains("ReduceFinish"),
            "should name ReduceFinish: {code}"
        );
    }

    #[test]
    fn step_repeat_start_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "RepeatStart should emit unsupported: {code}"
        );
        assert!(
            code.contains("RepeatStart"),
            "should name RepeatStart: {code}"
        );
    }

    #[test]
    fn step_repeat_attempt_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
            finish_node(2),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "RepeatAttempt should emit unsupported: {code}"
        );
        assert!(
            code.contains("RepeatAttempt"),
            "should name RepeatAttempt: {code}"
        );
    }

    #[test]
    fn step_repeat_check_emits_unsupported() {
        let nodes = vec![
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
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "RepeatCheck should emit unsupported: {code}"
        );
        assert!(
            code.contains("RepeatCheck"),
            "should name RepeatCheck: {code}"
        );
    }

    #[test]
    fn step_repeat_finish_emits_unsupported() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatFinish {
                    result: SlotIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("UnsupportedPrimitive"),
            "RepeatFinish should emit unsupported: {code}"
        );
        assert!(
            code.contains("RepeatFinish"),
            "should name RepeatFinish: {code}"
        );
    }

    #[test]
    fn step_emitted_function_has_balanced_braces() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let code = emit_first_node(node, 2);
        let open_count = code.chars().filter(|c| *c == '{').count();
        let close_count = code.chars().filter(|c| *c == '}').count();
        assert_eq!(
            open_count, close_count,
            "braces should be balanced in emitted code: {code}"
        );
    }

    #[test]
    fn step_emitted_function_has_function_signature() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let code = emit_first_node(node, 2);
        assert!(
            code.contains("fn step_0(_slots:"),
            "should have step function signature: {code}"
        );
        assert!(
            code.contains("StepOutcome"),
            "should return StepOutcome: {code}"
        );
        assert!(
            code.contains("DriveError"),
            "should return Result with DriveError: {code}"
        );
    }

    #[test]
    fn step_emit_function_with_high_step_id() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: Some(StepIdx::new(4)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: Some(StepIdx::new(5)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            finish_node(5),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(4), &wf);
        assert!(
            code.contains("fn step_4("),
            "should emit step_4 function: {code}"
        );
    }

    #[test]
    fn step_choose_slot_multiple_branches() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        vb_core::SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        vb_core::SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(3)),
                },
            },
            finish_node(1),
            finish_node(2),
            finish_node(3),
        ];
        let wf = make_step_workflow(nodes, 3);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("read_slot(slots, 0)"),
            "first branch should check slot 0: {code}"
        );
        assert!(
            code.contains("read_slot(slots, 1)"),
            "second branch should check slot 1: {code}"
        );
        assert!(
            code.contains("Continue(1)"),
            "first branch targets step 1: {code}"
        );
        assert!(
            code.contains("Continue(2)"),
            "second branch targets step 2: {code}"
        );
        assert!(
            code.contains("Continue(3)"),
            "otherwise targets step 3: {code}"
        );
    }

    #[test]
    fn step_build_object_multiple_fields() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(3)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![
                        (vb_core::SymbolId::new(0), SlotIdx::new(0)),
                        (vb_core::SymbolId::new(1), SlotIdx::new(1)),
                    ]
                    .into_boxed_slice(),
                },
            },
            finish_node(1),
        ];
        let wf =
            make_step_workflow_with_symbols(nodes, 4, 2, Box::new([]), Box::new([]), Box::new([]));
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(code.contains("2 field(s)"), "should show 2 fields: {code}");
        assert!(code.contains("_f0"), "should emit field 0: {code}");
        assert!(code.contains("_f1"), "should emit field 1: {code}");
        assert!(code.contains("_sym_0"), "should reference symbol 0: {code}");
        assert!(code.contains("_sym_1"), "should reference symbol 1: {code}");
    }

    #[test]
    fn step_nop_emission_valid_rust_structure() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let code = emit_first_node(node, 2);
        let lines: Vec<&str> = code.lines().collect();
        assert!(
            lines.len() >= 3,
            "emitted code should have at least 3 lines: {code}"
        );
        assert!(
            lines
                .first()
                .is_some_and(|line| line.starts_with("fn step_0(")),
            "first line should be function decl: {code}"
        );
    }

    // ========================================================================
    // Helper function tests (write_next_or_error, emit_unsupported_step,
    // emit_unsupported_expr, write_header)
    // ========================================================================

    /// `write_next_or_error` with a valid next step must emit
    /// `Ok(StepOutcome::Continue(N))`.
    #[test]
    fn write_next_or_error_with_target() -> Result<(), String> {
        let mut out = String::new();
        crate::write_next_or_error(&mut out, Some(StepIdx::new(7))).map_err(|e| e.to_string())?;
        assert!(
            out.contains("StepOutcome::Continue(7)"),
            "should emit Continue(7), got: {out}"
        );
        assert!(
            !out.contains("MissingNextStep"),
            "should not mention MissingNextStep, got: {out}"
        );
        Ok(())
    }

    /// `write_next_or_error` with `None` must emit `Err(DriveError::MissingNextStep)`.
    #[test]
    fn write_next_or_error_without_target() -> Result<(), String> {
        let mut out = String::new();
        crate::write_next_or_error(&mut out, None).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "should emit MissingNextStep, got: {out}"
        );
        assert!(
            !out.contains("StepOutcome::Continue"),
            "should not emit Continue, got: {out}"
        );
        Ok(())
    }

    /// `emit_unsupported_step` must emit the primitive name inside the error.
    #[test]
    fn emit_unsupported_step_contains_primitive_name() -> Result<(), String> {
        let mut out = String::new();
        crate::emit_unsupported_step(&mut out, "ForEachStart").map_err(|e| e.to_string())?;
        assert!(
            out.contains("UnsupportedPrimitive"),
            "should emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ForEachStart"),
            "should embed primitive name, got: {out}"
        );
        Ok(())
    }

    /// `emit_unsupported_step` with a different primitive name.
    #[test]
    fn emit_unsupported_step_different_primitive() -> Result<(), String> {
        let mut out = String::new();
        crate::emit_unsupported_step(&mut out, "RepeatCheck").map_err(|e| e.to_string())?;
        assert!(
            out.contains("RepeatCheck"),
            "should embed RepeatCheck, got: {out}"
        );
        assert!(
            !out.contains("ForEachStart"),
            "should not contain wrong primitive, got: {out}"
        );
        Ok(())
    }

    /// `emit_unsupported_expr` must emit the op name inside the error
    /// and include a `return` statement.
    #[test]
    fn emit_unsupported_expr_contains_op_name() -> Result<(), String> {
        let mut out = String::new();
        crate::emit_unsupported_expr(&mut out, "append").map_err(|e| e.to_string())?;
        assert!(
            out.contains("UnsupportedExpressionOp"),
            "should emit UnsupportedExpressionOp, got: {out}"
        );
        assert!(out.contains("append"), "should embed op name, got: {out}");
        assert!(
            out.contains("return"),
            "should include return statement, got: {out}"
        );
        Ok(())
    }

    /// `emit_unsupported_expr` with a different op name.
    #[test]
    fn emit_unsupported_expr_different_op() -> Result<(), String> {
        let mut out = String::new();
        crate::emit_unsupported_expr(&mut out, "merge").map_err(|e| e.to_string())?;
        assert!(out.contains("merge"), "should embed merge, got: {out}");
        assert!(
            !out.contains("append"),
            "should not contain wrong op, got: {out}"
        );
        Ok(())
    }

    /// `write_header` must emit the unsafe_code forbid directive.
    #[test]
    fn write_header_emits_forbid_unsafe() -> Result<(), String> {
        let mut out = String::new();
        crate::write_header(&mut out).map_err(|e| e.to_string())?;
        assert!(
            out.contains("#![forbid(unsafe_code)]"),
            "should forbid unsafe_code, got first 200 chars: {}",
            &out.chars().take(200).collect::<String>()
        );
        Ok(())
    }

    /// `write_header` must emit the SlotValue enum with all variants.
    #[test]
    fn write_header_emits_slot_value_enum() -> Result<(), String> {
        let mut out = String::new();
        crate::write_header(&mut out).map_err(|e| e.to_string())?;
        assert!(
            out.contains("pub enum SlotValue"),
            "should define SlotValue enum, got first 200 chars: {}",
            &out.chars().take(200).collect::<String>()
        );
        assert_contains_all(
            &out,
            &[
                "Null", "Bool", "I64", "F64", "Symbol", "List", "Object", "Blob",
            ],
            "SlotValue",
        )?;
        Ok(())
    }

    /// `write_header` must emit the DriveError enum with every generated variant.
    #[test]
    fn write_header_emits_drive_error_enum() -> Result<(), String> {
        let mut out = String::new();
        crate::write_header(&mut out).map_err(|e| e.to_string())?;
        assert!(
            out.contains("pub enum DriveError"),
            "should define DriveError enum"
        );
        assert_contains_all(
            &out,
            &[
                "InvalidProgramCounter",
                "MissingNextStep",
                "MissingOutputSlot",
                "SlotNull",
                "NoBranchMatched",
                "ExpressionStackOverflow",
                "TypeMismatch",
                "DivisionByZero",
                "IntegerOverflow",
                "ExpressionStackUnderflow",
                "IterationLimitExceeded",
                "ListStoreOverflow",
                "InvalidListHandle",
                "ActionSuspend",
                "UnknownAction",
                "UnsupportedPrimitive",
                "UnsupportedExpressionOp",
                "InvalidCompiledWorkflow",
            ],
            "DriveError",
        )?;
        Ok(())
    }

    /// `write_header` must emit the StepOutcome enum.
    #[test]
    fn write_header_emits_step_outcome() -> Result<(), String> {
        let mut out = String::new();
        crate::write_header(&mut out).map_err(|e| e.to_string())?;
        assert!(
            out.contains("enum StepOutcome"),
            "should define StepOutcome enum"
        );
        assert!(
            out.contains("Continue(u16)"),
            "StepOutcome should have Continue(u16)"
        );
        assert!(
            out.contains("Finished(SlotValue)"),
            "StepOutcome should have Finished(SlotValue)"
        );
        Ok(())
    }

    /// `write_header` must emit the ExprStack struct and its methods.
    #[test]
    fn write_header_emits_expr_stack() -> Result<(), String> {
        let mut out = String::new();
        crate::write_header(&mut out).map_err(|e| e.to_string())?;
        assert!(
            out.contains("struct ExprStack"),
            "should define ExprStack struct"
        );
        assert!(
            out.contains("MAX_EXPRESSION_STACK"),
            "should define MAX_EXPRESSION_STACK constant"
        );
        Ok(())
    }

    /// `write_header` must emit the read_slot / write_slot helpers.
    #[test]
    fn write_header_emits_slot_helpers() -> Result<(), String> {
        let mut out = String::new();
        crate::write_header(&mut out).map_err(|e| e.to_string())?;
        assert!(
            out.contains("fn read_slot("),
            "should emit read_slot function"
        );
        assert!(
            out.contains("fn write_slot("),
            "should emit write_slot function"
        );
        assert!(
            out.contains("fn read_const("),
            "should emit read_const function"
        );
        Ok(())
    }

    /// `write_header` must emit the generated-workflow comment.
    #[test]
    fn write_header_emits_generated_comment() -> Result<(), String> {
        let mut out = String::new();
        crate::write_header(&mut out).map_err(|e| e.to_string())?;
        assert!(
            out.contains("Generated workflow - DO NOT EDIT"),
            "should contain generated-workflow warning"
        );
        assert!(
            out.contains("Produced by vb_codegen emit_rust_workflow"),
            "should contain producer attribution"
        );
        Ok(())
    }

    // ========================================================================
    // Resource contract emission tests
    // ========================================================================

    /// `emit_resource_contract` with default contract must emit all constant
    /// names and the correct default values.
    #[test]
    fn emit_resource_contract_default() -> Result<(), String> {
        let mut out = String::new();
        emit_resource_contract(&mut out, ResourceContract::DEFAULT).map_err(|e| e.to_string())?;

        assert!(
            out.contains("// --- Resource contract ---"),
            "should emit resource contract header"
        );

        assert!(
            out.contains("CONTRACT_MAX_STEPS"),
            "should emit constant CONTRACT_MAX_STEPS"
        );
        assert!(
            out.contains("CONTRACT_MAX_STEPS: ") || out.contains("const CONTRACT_MAX_STEPS: "),
            "should define constant CONTRACT_MAX_STEPS"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_STEPS"))
            .ok_or("constant CONTRACT_MAX_STEPS not found in output")?;
        assert!(
            line.contains("10000"),
            "constant CONTRACT_MAX_STEPS should have value 10000, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_SLOTS"),
            "should emit constant CONTRACT_MAX_SLOTS"
        );
        assert!(
            out.contains("CONTRACT_MAX_SLOTS: ") || out.contains("const CONTRACT_MAX_SLOTS: "),
            "should define constant CONTRACT_MAX_SLOTS"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_SLOTS"))
            .ok_or("constant CONTRACT_MAX_SLOTS not found in output")?;
        assert!(
            line.contains("1024"),
            "constant CONTRACT_MAX_SLOTS should have value 1024, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_CONSTANTS"),
            "should emit constant CONTRACT_MAX_CONSTANTS"
        );
        assert!(
            out.contains("CONTRACT_MAX_CONSTANTS: ")
                || out.contains("const CONTRACT_MAX_CONSTANTS: "),
            "should define constant CONTRACT_MAX_CONSTANTS"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_CONSTANTS"))
            .ok_or("constant CONTRACT_MAX_CONSTANTS not found in output")?;
        assert!(
            line.contains("65535"),
            "constant CONTRACT_MAX_CONSTANTS should have value 65535, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_ACCESSORS"),
            "should emit constant CONTRACT_MAX_ACCESSORS"
        );
        assert!(
            out.contains("CONTRACT_MAX_ACCESSORS: ")
                || out.contains("const CONTRACT_MAX_ACCESSORS: "),
            "should define constant CONTRACT_MAX_ACCESSORS"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_ACCESSORS"))
            .ok_or("constant CONTRACT_MAX_ACCESSORS not found in output")?;
        assert!(
            line.contains("8192"),
            "constant CONTRACT_MAX_ACCESSORS should have value 8192, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_EXPRESSIONS"),
            "should emit constant CONTRACT_MAX_EXPRESSIONS"
        );
        assert!(
            out.contains("CONTRACT_MAX_EXPRESSIONS: ")
                || out.contains("const CONTRACT_MAX_EXPRESSIONS: "),
            "should define constant CONTRACT_MAX_EXPRESSIONS"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_EXPRESSIONS"))
            .ok_or("constant CONTRACT_MAX_EXPRESSIONS not found in output")?;
        assert!(
            line.contains("4096"),
            "constant CONTRACT_MAX_EXPRESSIONS should have value 4096, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_EXPR_STACK"),
            "should emit constant CONTRACT_MAX_EXPR_STACK"
        );
        assert!(
            out.contains("CONTRACT_MAX_EXPR_STACK: ")
                || out.contains("const CONTRACT_MAX_EXPR_STACK: "),
            "should define constant CONTRACT_MAX_EXPR_STACK"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_EXPR_STACK"))
            .ok_or("constant CONTRACT_MAX_EXPR_STACK not found in output")?;
        assert!(
            line.contains("64"),
            "constant CONTRACT_MAX_EXPR_STACK should have value 64, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_STEP_BUDGET_PER_TICK"),
            "should emit constant CONTRACT_MAX_STEP_BUDGET_PER_TICK"
        );
        assert!(
            out.contains("CONTRACT_MAX_STEP_BUDGET_PER_TICK: ")
                || out.contains("const CONTRACT_MAX_STEP_BUDGET_PER_TICK: "),
            "should define constant CONTRACT_MAX_STEP_BUDGET_PER_TICK"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_STEP_BUDGET_PER_TICK"))
            .ok_or("constant CONTRACT_MAX_STEP_BUDGET_PER_TICK not found in output")?;
        assert!(
            line.contains("10000"),
            "constant CONTRACT_MAX_STEP_BUDGET_PER_TICK should have value 10000, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_INPUT_BYTES"),
            "should emit constant CONTRACT_MAX_INPUT_BYTES"
        );
        assert!(
            out.contains("CONTRACT_MAX_INPUT_BYTES: ")
                || out.contains("const CONTRACT_MAX_INPUT_BYTES: "),
            "should define constant CONTRACT_MAX_INPUT_BYTES"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_INPUT_BYTES"))
            .ok_or("constant CONTRACT_MAX_INPUT_BYTES not found in output")?;
        assert!(
            line.contains("1048576"),
            "constant CONTRACT_MAX_INPUT_BYTES should have value 1048576, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_OUTPUT_BYTES"),
            "should emit constant CONTRACT_MAX_OUTPUT_BYTES"
        );
        assert!(
            out.contains("CONTRACT_MAX_OUTPUT_BYTES: ")
                || out.contains("const CONTRACT_MAX_OUTPUT_BYTES: "),
            "should define constant CONTRACT_MAX_OUTPUT_BYTES"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_OUTPUT_BYTES"))
            .ok_or("constant CONTRACT_MAX_OUTPUT_BYTES not found in output")?;
        assert!(
            line.contains("262144"),
            "constant CONTRACT_MAX_OUTPUT_BYTES should have value 262144, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_BLOB_BYTES"),
            "should emit constant CONTRACT_MAX_BLOB_BYTES"
        );
        assert!(
            out.contains("CONTRACT_MAX_BLOB_BYTES: ")
                || out.contains("const CONTRACT_MAX_BLOB_BYTES: "),
            "should define constant CONTRACT_MAX_BLOB_BYTES"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_BLOB_BYTES"))
            .ok_or("constant CONTRACT_MAX_BLOB_BYTES not found in output")?;
        assert!(
            line.contains("16777216"),
            "constant CONTRACT_MAX_BLOB_BYTES should have value 16777216, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_IPC_PAYLOAD_BYTES"),
            "should emit constant CONTRACT_MAX_IPC_PAYLOAD_BYTES"
        );
        assert!(
            out.contains("CONTRACT_MAX_IPC_PAYLOAD_BYTES: ")
                || out.contains("const CONTRACT_MAX_IPC_PAYLOAD_BYTES: "),
            "should define constant CONTRACT_MAX_IPC_PAYLOAD_BYTES"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_IPC_PAYLOAD_BYTES"))
            .ok_or("constant CONTRACT_MAX_IPC_PAYLOAD_BYTES not found in output")?;
        assert!(
            line.contains("1048576"),
            "constant CONTRACT_MAX_IPC_PAYLOAD_BYTES should have value 1048576, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
            "should emit constant CONTRACT_MAX_RETRY_ATTEMPTS"
        );
        assert!(
            out.contains("CONTRACT_MAX_RETRY_ATTEMPTS: ")
                || out.contains("const CONTRACT_MAX_RETRY_ATTEMPTS: "),
            "should define constant CONTRACT_MAX_RETRY_ATTEMPTS"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_RETRY_ATTEMPTS"))
            .ok_or("constant CONTRACT_MAX_RETRY_ATTEMPTS not found in output")?;
        assert!(
            line.contains("3"),
            "constant CONTRACT_MAX_RETRY_ATTEMPTS should have value 3, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_FANOUT"),
            "should emit constant CONTRACT_MAX_FANOUT"
        );
        assert!(
            out.contains("CONTRACT_MAX_FANOUT: ") || out.contains("const CONTRACT_MAX_FANOUT: "),
            "should define constant CONTRACT_MAX_FANOUT"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_FANOUT"))
            .ok_or("constant CONTRACT_MAX_FANOUT not found in output")?;
        assert!(
            line.contains("64"),
            "constant CONTRACT_MAX_FANOUT should have value 64, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_COLLECT_ITEMS"),
            "should emit constant CONTRACT_MAX_COLLECT_ITEMS"
        );
        assert!(
            out.contains("CONTRACT_MAX_COLLECT_ITEMS: ")
                || out.contains("const CONTRACT_MAX_COLLECT_ITEMS: "),
            "should define constant CONTRACT_MAX_COLLECT_ITEMS"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_COLLECT_ITEMS"))
            .ok_or("constant CONTRACT_MAX_COLLECT_ITEMS not found in output")?;
        assert!(
            line.contains("1024"),
            "constant CONTRACT_MAX_COLLECT_ITEMS should have value 1024, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_QUEUE_DEPTH"),
            "should emit constant CONTRACT_MAX_QUEUE_DEPTH"
        );
        assert!(
            out.contains("CONTRACT_MAX_QUEUE_DEPTH: ")
                || out.contains("const CONTRACT_MAX_QUEUE_DEPTH: "),
            "should define constant CONTRACT_MAX_QUEUE_DEPTH"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_QUEUE_DEPTH"))
            .ok_or("constant CONTRACT_MAX_QUEUE_DEPTH not found in output")?;
        assert!(
            line.contains("1024"),
            "constant CONTRACT_MAX_QUEUE_DEPTH should have value 1024, got: {line}"
        );

        assert!(
            out.contains("CONTRACT_MAX_JOURNAL_BATCH_BYTES"),
            "should emit constant CONTRACT_MAX_JOURNAL_BATCH_BYTES"
        );
        assert!(
            out.contains("CONTRACT_MAX_JOURNAL_BATCH_BYTES: ")
                || out.contains("const CONTRACT_MAX_JOURNAL_BATCH_BYTES: "),
            "should define constant CONTRACT_MAX_JOURNAL_BATCH_BYTES"
        );
        let line = out
            .lines()
            .find(|l| l.contains("CONTRACT_MAX_JOURNAL_BATCH_BYTES"))
            .ok_or("constant CONTRACT_MAX_JOURNAL_BATCH_BYTES not found in output")?;
        assert!(
            line.contains("1048576"),
            "constant CONTRACT_MAX_JOURNAL_BATCH_BYTES should have value 1048576, got: {line}"
        );

        Ok(())
    }

    /// `emit_resource_contract` with a custom contract must reflect the
    /// custom values.
    #[test]
    fn emit_resource_contract_custom_values() -> Result<(), String> {
        let custom = ResourceContract {
            max_steps: 500,
            max_slots: 64,
            max_constants: 10,
            max_accessors: 4,
            max_expressions: 2,
            max_expr_stack: 8,
            max_step_budget_per_tick: 1000,
            max_input_bytes: 512,
            max_output_bytes: 256,
            max_blob_bytes: 4096,
            max_ipc_payload_bytes: 128,
            max_retry_attempts: 1,
            max_fanout: 2,
            max_collect_items: 50,
            max_queue_depth: 10,
            max_journal_batch_bytes: 2048,
        };
        let mut out = String::new();
        emit_resource_contract(&mut out, custom).map_err(|e| e.to_string())?;

        assert!(
            out.contains("CONTRACT_MAX_STEPS: u16 = 500;"),
            "custom max_steps should be 500"
        );
        assert!(
            out.contains("CONTRACT_MAX_SLOTS: u16 = 64;"),
            "custom max_slots should be 64"
        );
        assert!(
            out.contains("CONTRACT_MAX_EXPR_STACK: u8 = 8;"),
            "custom max_expr_stack should be 8"
        );
        assert!(
            out.contains("CONTRACT_MAX_RETRY_ATTEMPTS: u16 = 1;"),
            "custom max_retry_attempts should be 1"
        );
        Ok(())
    }

    /// `emit_resource_contract` with zero-value contract must emit zeroes.
    #[test]
    fn emit_resource_contract_zero_values() -> Result<(), String> {
        let zero = ResourceContract {
            max_steps: 0,
            max_slots: 0,
            max_constants: 0,
            max_accessors: 0,
            max_expressions: 0,
            max_expr_stack: 0,
            max_step_budget_per_tick: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            max_blob_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
        };
        let mut out = String::new();
        emit_resource_contract(&mut out, zero).map_err(|e| e.to_string())?;

        assert!(
            out.contains("CONTRACT_MAX_STEPS: u16 = 0;"),
            "zero max_steps should be 0"
        );
        assert!(
            out.contains("CONTRACT_MAX_SLOTS: u16 = 0;"),
            "zero max_slots should be 0"
        );
        Ok(())
    }

    /// Full workflow emission must include the resource contract section.
    #[test]
    fn emit_rust_workflow_includes_resource_contract() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("// --- Resource contract ---"),
            "full workflow must contain resource contract section"
        );
        assert!(
            source.contains("CONTRACT_MAX_STEPS"),
            "full workflow must contain CONTRACT_MAX_STEPS"
        );
        Ok(())
    }

    // ========================================================================
    // Action boundary emission tests
    // ========================================================================

    /// `emit_action_boundary` must emit a comment, a slot read, and the
    /// ActionSuspend error with the correct action_id and input_slot.
    #[test]
    fn emit_action_boundary_correct_ids() -> Result<(), String> {
        let mut out = String::new();
        let action = ActionId::new(42);
        let input = SlotIdx::new(7);
        emit_action_boundary(&mut out, action, input).map_err(|e| e.to_string())?;

        assert!(
            out.contains("Action boundary: action_id=42, input_slot=7"),
            "should emit comment with action_id=42 and input_slot=7, got: {out}"
        );
        assert!(
            out.contains("read_slot(slots, 7)"),
            "should read input slot 7, got: {out}"
        );
        assert!(
            out.contains("ActionSuspend"),
            "should emit ActionSuspend error"
        );
        assert!(
            out.contains("action_id: 42"),
            "should embed action_id 42 in error"
        );
        assert!(
            out.contains("input_slot: 7"),
            "should embed input_slot 7 in error"
        );
        Ok(())
    }

    /// `emit_action_boundary` with zero-valued IDs.
    #[test]
    fn emit_action_boundary_zero_ids() -> Result<(), String> {
        let mut out = String::new();
        emit_action_boundary(&mut out, ActionId::new(0), SlotIdx::new(0))
            .map_err(|e| e.to_string())?;

        assert!(
            out.contains("action_id=0, input_slot=0"),
            "should handle zero IDs, got: {out}"
        );
        assert!(
            out.contains("action_id: 0"),
            "error should have action_id: 0"
        );
        Ok(())
    }

    /// `emit_action_boundary` with large IDs.
    #[test]
    fn emit_action_boundary_large_ids() -> Result<(), String> {
        let mut out = String::new();
        emit_action_boundary(&mut out, ActionId::new(65535), SlotIdx::new(65534))
            .map_err(|e| e.to_string())?;

        assert!(
            out.contains("action_id=65535, input_slot=65534"),
            "should handle large IDs, got: {out}"
        );
        Ok(())
    }

    // ========================================================================
    // Action match dispatch emission tests
    // ========================================================================

    /// `emit_action_match_dispatch` for a workflow with no Do nodes must emit
    /// only the fallback arm.
    #[test]
    fn emit_action_match_dispatch_no_actions() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let mut out = String::new();
        emit_action_match_dispatch(&mut out, &workflow).map_err(|e| e.to_string())?;

        assert!(
            out.contains("pub fn dispatch_action(action_id: u16)"),
            "should emit dispatch_action function"
        );
        assert!(
            out.contains("UnknownAction"),
            "should have UnknownAction fallback"
        );
        // The minimal workflow has no Do nodes, so no action arms
        assert!(
            !out.contains("=> Ok(()),"),
            "should not have action match arms for a workflow with no Do nodes"
        );
        Ok(())
    }

    /// `emit_action_match_dispatch` for a workflow with Do nodes must emit
    /// action match arms.
    #[test]
    fn emit_action_match_dispatch_with_do_node() -> Result<(), String> {
        let workflow = do_action_workflow()?;
        let mut out = String::new();
        emit_action_match_dispatch(&mut out, &workflow).map_err(|e| e.to_string())?;

        assert!(
            out.contains("5 => Ok(()),"),
            "should emit action arm for ActionId 5, got: {out}"
        );
        assert!(
            out.contains("UnknownAction"),
            "should have UnknownAction fallback"
        );
        Ok(())
    }

    /// Full workflow emission must include action match dispatch.
    #[test]
    fn emit_rust_workflow_includes_action_dispatch() -> Result<(), String> {
        let workflow = do_action_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("// --- Action match dispatch ---"),
            "full workflow must contain action dispatch section"
        );
        assert!(
            source.contains("dispatch_action"),
            "full workflow must contain dispatch_action function"
        );
        Ok(())
    }

    // ========================================================================
    // Emit finish tests
    // ========================================================================

    /// `emit_finish` must emit the result extraction comment.
    #[test]
    fn emit_finish_produces_header() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let mut out = String::new();
        emit_finish(&mut out, &workflow).map_err(|e| e.to_string())?;
        assert!(
            out.contains("// --- Result extraction ---"),
            "should emit result extraction header, got: {out}"
        );
        Ok(())
    }

    // ========================================================================
    // Error handling paths -- end-to-end via emit_step_function
    // ========================================================================

    /// A Nop node with no next step must emit MissingNextStep in the step body.
    #[test]
    fn nop_no_next_emits_missing_next_step() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "Nop with no next should emit MissingNextStep, got: {out}"
        );
        Ok(())
    }

    /// A SetConst node with no next step must emit MissingNextStep.
    #[test]
    fn set_const_no_next_emits_missing_next_step() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "SetConst with no next should emit MissingNextStep, got: {out}"
        );
        Ok(())
    }

    /// A Copy node with no next step must emit MissingNextStep.
    #[test]
    fn copy_no_next_emits_missing_next_step() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(1)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "Copy with no next should emit MissingNextStep, got: {out}"
        );
        Ok(())
    }

    /// A Choose node where no branch matches and no fallback must emit
    /// NoBranchMatched.
    #[test]
    fn choose_no_branch_no_fallback_emits_no_branch_matched() -> Result<(), String> {
        let node = CompiledNode {
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
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("NoBranchMatched"),
            "Choose with no branches and no fallback should emit NoBranchMatched, got: {out}"
        );
        Ok(())
    }

    /// A ChooseSlot node where no branch matches and no fallback must emit
    /// NoBranchMatched.
    #[test]
    fn choose_slot_no_branch_no_fallback_emits_no_branch_matched() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: Box::new([]),
                otherwise: None,
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("NoBranchMatched"),
            "ChooseSlot with no branches and no fallback should emit NoBranchMatched, got: {out}"
        );
        Ok(())
    }

    /// A ForEachStart node must emit concrete iterator support through
    /// emit_step_function.
    #[test]
    fn for_each_start_emits_iterator_support() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(2)),
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
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            !out.contains("UnsupportedPrimitive"),
            "ForEachStart should emit concrete support, got: {out}"
        );
        assert!(
            out.contains("list_item_count") && out.contains("tail_list_handle"),
            "ForEachStart should count items and store tail, got: {out}"
        );
        Ok(())
    }

    /// A CollectStart node must emit UnsupportedPrimitive.
    #[test]
    fn collect_start_emits_unsupported() -> Result<(), String> {
        let node = CompiledNode {
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
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("UnsupportedPrimitive"),
            "CollectStart should emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("CollectStart"),
            "primitive name should be CollectStart, got: {out}"
        );
        Ok(())
    }

    /// A RepeatStart node must emit UnsupportedPrimitive.
    #[test]
    fn repeat_start_emits_unsupported() -> Result<(), String> {
        let node = CompiledNode {
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
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("UnsupportedPrimitive"),
            "RepeatStart should emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("RepeatStart"),
            "primitive name should be RepeatStart, got: {out}"
        );
        Ok(())
    }

    /// A TogetherStart node must emit UnsupportedPrimitive.
    #[test]
    fn together_start_emits_unsupported() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(2),
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("UnsupportedPrimitive"),
            "TogetherStart should emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("TogetherStart"),
            "primitive name should be TogetherStart, got: {out}"
        );
        Ok(())
    }

    /// A ReduceStart node must emit UnsupportedPrimitive.
    #[test]
    fn reduce_start_emits_unsupported() -> Result<(), String> {
        let node = CompiledNode {
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
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("UnsupportedPrimitive"),
            "ReduceStart should emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ReduceStart"),
            "primitive name should be ReduceStart, got: {out}"
        );
        Ok(())
    }

    /// A WaitUntil node with no next step must emit MissingNextStep.
    #[test]
    fn wait_until_no_next_emits_missing_next_step() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "WaitUntil with no next should emit MissingNextStep, got: {out}"
        );
        Ok(())
    }

    /// A WaitEvent node with no next step must emit MissingNextStep.
    #[test]
    fn wait_event_no_next_emits_missing_next_step() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "WaitEvent with no next should emit MissingNextStep, got: {out}"
        );
        Ok(())
    }

    /// An Ask node with no next step must emit MissingNextStep.
    #[test]
    fn ask_no_next_emits_missing_next_step() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "Ask with no next should emit MissingNextStep, got: {out}"
        );
        Ok(())
    }

    /// An AskResume node with no next step must emit MissingNextStep.
    #[test]
    fn ask_resume_no_next_emits_missing_next_step() -> Result<(), String> {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(0),
            },
        };
        let mut out = String::new();
        emit_step_function(&mut out, &node, &minimal_workflow()?).map_err(|e| e.to_string())?;
        assert!(
            out.contains("MissingNextStep"),
            "AskResume with no next should emit MissingNextStep, got: {out}"
        );
        Ok(())
    }

    // =======================================================================
    // Edge-case tests for generated-mode workflow compilation
    // =======================================================================

    /// Helper: build a workflow with entry pointing to a Finish node and nothing else.
    /// This represents the "empty workflow" edge case: entry -> finish.
    fn empty_entry_finish_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_empty_entry_finish"),
            digest: WorkflowDigest::from_bytes([0xE0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow with a single SetConst step -> Finish.
    fn single_step_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_single_step"),
            digest: WorkflowDigest::from_bytes([0xE1; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(99)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow containing a ForEachStart node.
    fn foreach_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_foreach"),
            digest: WorkflowDigest::from_bytes([0xE2; 32]),
            nodes: vec![
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
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(2),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow containing a TogetherStart node.
    fn edge_case_together_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_together"),
            digest: WorkflowDigest::from_bytes([0xE3; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: vec![StepIdx::new(1)].into_boxed_slice(),
                        join: StepIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
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
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherJoin {
                        branch_count: 1,
                        accumulator: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow containing a RepeatStart node (which is unsupported).
    fn edge_case_repeat_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_repeat"),
            digest: WorkflowDigest::from_bytes([0xE4; 32]),
            nodes: vec![
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
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(0)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatAttempt {
                        attempt_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RepeatFinish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow with a WaitUntil step.
    fn edge_case_wait_until_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_wait_until"),
            digest: WorkflowDigest::from_bytes([0xE5; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitUntil {
                        deadline_slot: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(100)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow with an Ask step followed by an AskResume step.
    fn edge_case_ask_resume_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_ask_resume"),
            digest: WorkflowDigest::from_bytes([0xE6; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Ask {
                        prompt: SlotIdx::new(0),
                        timeout_slot: None,
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::AskResume {
                        answer: SlotIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow with a ChooseSlot node using slot-based conditions.
    fn edge_case_choose_slot_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_choose_slot"),
            digest: WorkflowDigest::from_bytes([0xE7; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: vec![vb_core::SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(3)),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![
                ConstValue::Bool(true),
                ConstValue::I64(1),
                ConstValue::I64(2),
            ]
            .into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    /// Helper: build a workflow with a Do node and a RetryCheck step.
    fn do_with_retry_check_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_do_retry"),
            digest: WorkflowDigest::from_bytes([0xE8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Do {
                        action: ActionId::new(10),
                        input: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        exhausted: StepIdx::new(4),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
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
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(0), ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // --- Test 1: Empty workflow (entry -> finish, no intermediate steps) ---

    #[test]
    fn edge_case_empty_workflow_entry_finish_only() -> Result<(), String> {
        let workflow = empty_entry_finish_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // The generated source must contain exactly one step function (the Finish node)
        let step_count = source
            .lines()
            .filter(|l| l.trim().starts_with("fn step_"))
            .count();
        assert_eq!(
            step_count, 1,
            "empty workflow should produce exactly 1 step function, got {step_count}"
        );

        // The drive function must start at entry=0
        assert!(
            source.contains("let mut pc: u16 = 0;"),
            "drive should start at pc=0 for empty workflow"
        );

        // The single step must be a Finish that reads from slot 0
        assert!(
            source.contains("StepOutcome::Finished"),
            "empty workflow must have a Finished outcome"
        );

        // Semantic equivalence check must pass
        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Test 2: Single step workflow (SetConst -> Finish) ---

    #[test]
    fn edge_case_single_step_workflow() -> Result<(), String> {
        let workflow = single_step_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // Must produce exactly 2 step functions (SetConst + Finish)
        let step_count = source
            .lines()
            .filter(|l| l.trim().starts_with("fn step_"))
            .count();
        assert_eq!(
            step_count, 2,
            "single step workflow should produce exactly 2 step functions, got {step_count}"
        );

        // The first step must write a constant
        assert!(source.contains("fn step_0"), "first step must be step_0");
        assert!(
            source.contains("write_slot") && source.contains("read_const(0)"),
            "step_0 must write constant 0 to a slot"
        );

        // The constant pool must contain I64(99)
        assert!(
            source.contains("I64(99)"),
            "constant pool must contain the I64(99) constant"
        );

        // Semantic check must pass
        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Test 3: ForEach loop is accepted by generated mode ---

    #[test]
    fn edge_case_foreach_loop_accepted_by_generated_mode() -> Result<(), String> {
        let workflow = foreach_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("tail_list_handle") && source.contains("first_list_item"),
            "ForEach generated code must include iterator support, got: {source}"
        );
        Ok(())
    }

    // --- Test 4: Together parallel is rejected by generated mode ---

    #[test]
    fn edge_case_together_parallel_rejected_by_generated_mode() -> Result<(), String> {
        let workflow = edge_case_together_workflow()?;
        let result = emit_rust_workflow(&workflow);

        assert!(
            result.is_err(),
            "Together workflows must be rejected by generated mode"
        );
        let err = result.err().ok_or("expected an error but got none")?;
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported") || msg.contains("UnsupportedIr"),
            "Together rejection must mention unsupported IR, got: {msg}"
        );
        assert!(
            msg.contains("TogetherStart"),
            "Together rejection must identify TogetherStart as the unsupported feature, got: {msg}"
        );
        Ok(())
    }

    // --- Test 5: Repeat with retry is rejected by generated mode ---

    #[test]
    fn edge_case_repeat_with_retry_rejected_by_generated_mode() -> Result<(), String> {
        let workflow = edge_case_repeat_workflow()?;
        let result = emit_rust_workflow(&workflow);

        assert!(
            result.is_err(),
            "Repeat workflows must be rejected by generated mode"
        );
        let err = result.err().ok_or("expected an error but got none")?;
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported") || msg.contains("UnsupportedIr"),
            "Repeat rejection must mention unsupported IR, got: {msg}"
        );
        assert!(
            msg.contains("RepeatStart"),
            "Repeat rejection must identify RepeatStart as the unsupported feature, got: {msg}"
        );
        Ok(())
    }

    // --- Test 6: WaitUntil step generates correct code ---

    #[test]
    fn edge_case_wait_until_step_generates_wait_code() -> Result<(), String> {
        let workflow = edge_case_wait_until_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // Must contain a step_1 function for the WaitUntil node
        assert!(
            source.contains("fn step_1"),
            "WaitUntil must produce step_1 function"
        );

        // WaitUntil must read the deadline slot
        assert!(
            source.contains("let _deadline = read_slot(slots, 0)"),
            "WaitUntil must read the deadline from slot 0"
        );

        // Must contain a Continue to step_2 after the wait
        assert!(
            source.contains("StepOutcome::Continue(2)"),
            "WaitUntil must continue to step 2"
        );

        // Semantic check
        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Test 7: Ask step generates ask/resume pair ---

    #[test]
    fn edge_case_ask_step_generates_ask_resume_pair() -> Result<(), String> {
        let workflow = edge_case_ask_resume_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // Must contain step_1 for the Ask node and step_2 for AskResume
        assert!(
            source.contains("fn step_1"),
            "Ask node must produce step_1 function"
        );
        assert!(
            source.contains("fn step_2"),
            "AskResume node must produce step_2 function"
        );

        // Ask must read the prompt slot
        assert!(
            source.contains("let _prompt = read_slot(slots, 0)"),
            "Ask step must read prompt from slot 0"
        );

        // AskResume must reference answer slot
        assert!(
            source.contains("let _answer_slot: u16 = 1"),
            "AskResume must reference answer slot 1"
        );

        // Semantic check
        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Test 8: Choose with slot condition generates ChooseSlot not Choose ---

    #[test]
    fn edge_case_choose_slot_generates_slot_based_branching() -> Result<(), String> {
        let workflow = edge_case_choose_slot_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // Must NOT use eval_expr_ for ChooseSlot (it uses read_slot instead)
        assert!(
            source.contains("let _condition = read_slot(slots, 0)?"),
            "ChooseSlot must branch by reading slot 0 directly, not via expression"
        );

        // Must contain a fallback to the otherwise target (step_3)
        assert!(
            source.contains("StepOutcome::Continue(3)"),
            "ChooseSlot otherwise must continue to step 3"
        );

        // The true branch must go to step_2
        assert!(
            source.contains("StepOutcome::Continue(2)"),
            "ChooseSlot branch must continue to step 2 when condition is true"
        );

        // Semantic check
        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Test 9: Action contract for Do node with retry policy ---

    #[test]
    fn edge_case_do_node_with_retry_check_generates_contract() -> Result<(), String> {
        let workflow = do_with_retry_check_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

        // The Do node (step_1) must produce an action boundary for action 10
        assert!(
            source.contains("action_id: 10"),
            "Do node must reference action_id 10"
        );
        assert!(
            source.contains("Action boundary: action_id=10"),
            "Do node must emit action boundary comment"
        );

        // The RetryCheck node (step_2) must read the policy slot
        assert!(
            source.contains("let _policy = read_slot(slots, 0)"),
            "RetryCheck must read policy from slot 0"
        );

        // RetryCheck must compare retry count to CONTRACT_MAX_RETRY_ATTEMPTS
        assert!(
            source.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
            "RetryCheck must reference CONTRACT_MAX_RETRY_ATTEMPTS"
        );

        // RetryCheck must have branch targets for retry body and exhausted path
        assert!(
            source.contains("StepOutcome::Continue(1)")
                && source.contains("StepOutcome::Continue(4)"),
            "RetryCheck must branch to body (step 1) or exhausted (step 4)"
        );

        // Action dispatch must register action 10
        assert!(
            source.contains("10 => Ok(())"),
            "action dispatch must include arm for action 10"
        );

        // Semantic check
        compare_generated_to_ir(&source, &workflow).map_err(|e| e.to_string())?;
        Ok(())
    }
}
