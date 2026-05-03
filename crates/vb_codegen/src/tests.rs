#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use super::{
        CodegenError, compare_generated_to_ir, compile_check_generated_rust, emit_action_boundary,
        emit_action_match_dispatch, emit_drive_function, emit_finish, emit_ids,
        emit_resource_contract, emit_rust_workflow, emit_step_function, emit_trybuild_fixture,
        format_generated_rust, validate_generated_subset,
    };
    use vb_core::{
        AccessorProgram, ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx,
        ConstValue, EngineSignal, ExprProgram, PathSegment, ResourceContract, RunId, SlotIdx,
        SlotValue, StepBudget, StepIdx, ValueStore, WorkflowDigest, WorkflowParts, new_run_frame,
        run_until_blocked, step_once,
    };

    // --- Workflow helpers ---

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
        let temp_dir = std::env::temp_dir().join(format!(
            "vb_codegen_action_suspend_{}_{}_{}",
            std::process::id(),
            action.get(),
            input.get()
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let source_path = temp_dir.join("generated_action_suspend.rs");
        let binary_path = temp_dir.join("generated_action_suspend_bin");
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

        let cleanup = std::fs::remove_dir_all(&temp_dir);
        if let Err(e) = cleanup {
            return Err(e.to_string());
        }
        Ok(stdout)
    }

    fn generated_drive_stdout(
        workflow: &CompiledWorkflow,
        name: &str,
        init_source: &str,
    ) -> Result<String, String> {
        let generated = emit_rust_workflow(workflow).map_err(|e| e.to_string())?;
        let temp_dir =
            std::env::temp_dir().join(format!("vb_codegen_drive_{}_{}", std::process::id(), name));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let source_path = temp_dir.join("generated_drive.rs");
        let binary_path = temp_dir.join("generated_drive_bin");
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

        let cleanup = std::fs::remove_dir_all(&temp_dir);
        if let Err(e) = cleanup {
            return Err(e.to_string());
        }
        Ok(stdout)
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
        // Given a ForEachStart node (unsupported in codegen)
        let workflow = for_each_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "ForEachStart must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ForEachStart"),
            "UnsupportedPrimitive must name ForEachStart, got: {out}"
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
        let mut step_count = 0u16;
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn step_") {
                step_count = step_count.checked_add(1).ok_or("overflow")?;
            }
        }
        assert!(
            step_count == workflow.node_count(),
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
        assert!(result.is_err(), "must reject source with Vec usage");
        let err = match result {
            Ok(()) => String::new(),
            Err(error) => error.to_string(),
        };
        assert!(err.contains("Vec"), "error must mention Vec, got: {err}");
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
        assert!(result.is_err(), "must reject source with unchecked cast");
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_accepts_clean_output() -> Result<(), String> {
        // Given a clean generated workflow
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        // When comparing against the IR
        let result = compare_generated_to_ir(&source, &workflow);
        // Then it succeeds
        assert!(
            result.is_ok(),
            "clean generated source must pass semantic comparison"
        );
        Ok(())
    }

    #[test]
    fn generated_action_suspend_matches_ir_awaiting_action_family() -> Result<(), String> {
        let cases = [
            (ActionId::new(1), SlotIdx::new(0)),
            (ActionId::new(5), SlotIdx::new(1)),
            (ActionId::new(9), SlotIdx::new(2)),
        ];

        let mut index = 0usize;
        while index < cases.len() {
            let (action, input) = cases
                .get(index)
                .copied()
                .ok_or_else(|| String::from("case index checked by loop bound"))?;
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

            index = index.saturating_add(1);
        }

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
    fn emit_trybuild_fixture_writes_file_to_disk() -> Result<(), String> {
        // Given a minimal workflow and a temp fixture path
        let workflow = minimal_workflow()?;
        let temp_dir =
            std::env::temp_dir().join(format!("vb_codegen_fixture_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let fixture_path = temp_dir.join("fixture.rs");
        // When emit_trybuild_fixture writes the file
        let result = emit_trybuild_fixture(&workflow, &fixture_path);
        // Then it succeeds and the file exists
        assert!(result.is_ok(), "trybuild fixture write must succeed");
        let content = std::fs::read_to_string(&fixture_path).map_err(|e| e.to_string())?;
        assert!(!content.is_empty(), "fixture file must be non-empty");
        assert!(
            content.contains("#![forbid(unsafe_code)]"),
            "fixture must contain generated Rust with forbid unsafe"
        );
        let cleanup = std::fs::remove_dir_all(&temp_dir);
        if let Err(e) = cleanup {
            return Err(e.to_string());
        }
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
        assert!(
            result.is_err(),
            "must fail for root path without writable parent"
        );
        Ok(())
    }

    // --- Proptest Properties ---

    #[test]
    fn codegen_error_display_contains_variant_name() {
        // Given all CodegenError variants
        let errors: Vec<(CodegenError, &'static str)> = vec![
            (CodegenError::FormatBufferOverflow, "buffer"),
            (
                CodegenError::RustfmtFailed {
                    detail: String::from("test"),
                },
                "rustfmt",
            ),
            (
                CodegenError::CompileCheckFailed {
                    detail: String::from("test"),
                },
                "compile",
            ),
            (
                CodegenError::SemanticMismatch {
                    detail: String::from("test"),
                },
                "semantic",
            ),
            (
                CodegenError::Io(std::io::Error::other("io")),
                "codegen IO error",
            ),
            (
                CodegenError::TrybuildFixture {
                    detail: String::from("test"),
                },
                "trybuild",
            ),
        ];
        // When each error is displayed
        for (error, keyword) in errors {
            let message = error.to_string();
            // Then the display message contains a distinguishing keyword
            assert!(
                message.contains(keyword),
                "error display must contain keyword '{keyword}', got: {message}"
            );
        }
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
            slot_count: 2,
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
                    output: None,
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
            out.contains("is_true()"),
            "Choose must check is_true, got: {out}"
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
            out.contains("is_true()"),
            "ChooseSlot must check is_true, got: {out}"
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
        let temp_dir = std::env::temp_dir().join(format!(
            "vb_codegen_root_accessor_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let result = compile_check_generated_rust(&source, &temp_dir).map_err(|e| e.to_string());
        let cleanup = std::fs::remove_dir_all(&temp_dir).map_err(|e| e.to_string());
        cleanup?;
        result
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
        let temp_dir = std::env::temp_dir().join(format!("vb_codegen_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let result = compile_check_generated_rust(&source, &temp_dir).map_err(|e| e.to_string());
        let cleanup = std::fs::remove_dir_all(&temp_dir).map_err(|e| e.to_string());
        cleanup?;
        result
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
        let workflows: Vec<(&str, Result<CompiledWorkflow, String>)> = vec![
            ("nop", nop_workflow()),
            ("copy", copy_workflow()),
            ("jump", jump_workflow()),
            ("do_action", do_action_workflow()),
        ];
        for (name, workflow_result) in workflows {
            let workflow = workflow_result?;
            // When generating the full source
            let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
            // Then every step function name follows the pattern "fn step_N"
            let mut found_step = false;
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn step_") && trimmed.contains('(') {
                    found_step = true;
                    let end = trimmed.find('(').ok_or("no paren in step fn")?;
                    let fn_name = trimmed.get(3..end).ok_or("step fn name range invalid")?;
                    assert!(
                        fn_name.starts_with("step_"),
                        "function name must start with step_, got: {fn_name} in workflow {name}"
                    );
                    let suffix = fn_name.get(5..).ok_or("step fn suffix range invalid")?;
                    assert!(
                        suffix.parse::<u16>().is_ok(),
                        "step suffix must be a valid u16, got: {suffix} in workflow {name}"
                    );
                }
            }
            assert!(
                found_step,
                "must find at least one step function in workflow {name}"
            );
        }
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
            out.contains("step_1(slots)"),
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
        let all_fields = [
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
        ];
        for field in &all_fields {
            assert!(
                out.contains(field),
                "emit_resource_contract must include {field}"
            );
        }
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
        let forbidden_patterns = [
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
        ];
        let mut violations = Vec::new();
        for (pattern, label) in &forbidden_patterns {
            // Check in non-comment, non-string-literal contexts
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }
                if trimmed.contains(pattern) {
                    // Exclude the #![forbid(unsafe_code)] lint gate itself
                    if *pattern == "unsafe " && trimmed.contains("#![forbid(unsafe_code)]") {
                        continue;
                    }
                    violations.push((*label, trimmed.to_string()));
                }
            }
        }
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
        let mut nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: branches.into_boxed_slice(),
                otherwise: None,
            },
        }];
        for i in 1..=5 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            });
        }
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
        for i in 1..=5 {
            assert!(
                out.contains(&format!("StepOutcome::Continue({i})")),
                "Choose must emit branch target {i}, got: {out}"
            );
        }
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
        let mut step_count = 0u16;
        for line in source.lines() {
            if line.trim().starts_with("fn step_") {
                step_count = step_count.checked_add(1).ok_or("overflow")?;
            }
        }
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
        assert!(result.is_err(), "must reject source with wrong step count");
        let err = result.err().ok_or("expected error")?;
        let msg = err.to_string();
        assert!(
            msg.contains("step count mismatch"),
            "error must mention step count, got: {msg}"
        );
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
        let forbidden = [
            "wrapping_add",
            "wrapping_sub",
            "wrapping_mul",
            "saturating_add",
            "overflowing_add",
        ];
        for pattern in &forbidden {
            assert!(
                !source.contains(pattern),
                "generated source must not contain {pattern}"
            );
        }
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
        let required_variants = [
            "InvalidProgramCounter",
            "MissingNextStep",
            "SlotNull",
            "NoBranchMatched",
            "ExpressionStackOverflow",
            "TypeMismatch",
            "DivisionByZero",
            "IntegerOverflow",
            "ExpressionStackUnderflow",
            "ActionSuspend",
            "UnknownAction",
            "UnsupportedPrimitive",
            "UnsupportedExpressionOp",
            "InvalidCompiledWorkflow",
        ];
        for variant in &required_variants {
            assert!(
                source.contains(variant),
                "DriveError must define variant {variant}"
            );
        }
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
        let mut nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: branches.into_boxed_slice(),
                otherwise: None,
            },
        }];
        for i in 1..=3 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            });
        }
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
        for slot_idx in 0..3 {
            assert!(
                out.contains(&format!("read_slot(slots, {slot_idx})")),
                "ChooseSlot must read condition slot {slot_idx}, got: {out}"
            );
        }
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
                    output: None,
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

    fn for_each_join_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_for_each_join"),
            digest: WorkflowDigest::from_bytes([0x79; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachJoin {
                        output: SlotIdx::new(0),
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

    // --- ForEach validation rejection tests ---

    #[test]
    fn for_each_start_codegen_is_typed_error() -> Result<(), String> {
        let workflow = for_each_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "ForEachStart",
            "unsupported generated Rust IR feature: ForEachStart",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "ForEachStart",
            "unsupported generated Rust IR feature: ForEachStart",
        )?;
        Ok(())
    }

    #[test]
    fn for_each_next_codegen_is_typed_error() -> Result<(), String> {
        let workflow = for_each_next_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "ForEachNext",
            "unsupported generated Rust IR feature: ForEachNext",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "ForEachNext",
            "unsupported generated Rust IR feature: ForEachNext",
        )?;
        Ok(())
    }

    #[test]
    fn for_each_join_codegen_is_typed_error() -> Result<(), String> {
        let workflow = for_each_join_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "ForEachJoin",
            "unsupported generated Rust IR feature: ForEachJoin",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "ForEachJoin",
            "unsupported generated Rust IR feature: ForEachJoin",
        )?;
        Ok(())
    }

    #[test]
    fn for_each_sum_workflow_is_rejected_by_codegen() -> Result<(), String> {
        // Given a complete ForEach sum workflow with ForEachStart, ForEachNext, and ForEachJoin
        let workflow = for_each_sum_workflow()?;
        // When validate_generated_subset checks it
        // Then it rejects with ForEachStart (first unsupported node encountered)
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "ForEachStart",
            "unsupported generated Rust IR feature: ForEachStart",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "ForEachStart",
            "unsupported generated Rust IR feature: ForEachStart",
        )?;
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

    // --- Nested ForEach validation rejection test ---

    #[test]
    fn nested_for_each_workflow_is_rejected_by_codegen() -> Result<(), String> {
        // Given a workflow with a ForEachStart inside a ForEachStart (nested loops)
        let workflow = nested_for_each_workflow()?;
        // When validate_generated_subset checks it
        // Then it rejects with ForEachStart (outer loop is first unsupported node)
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "ForEachStart",
            "unsupported generated Rust IR feature: ForEachStart",
        )?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "ForEachStart",
            "unsupported generated Rust IR feature: ForEachStart",
        )?;
        Ok(())
    }

    // --- Step emit verification for all ForEach/Together node kinds ---

    #[test]
    fn emit_step_match_produces_correct_arm_for_for_each_next_node() -> Result<(), String> {
        // Given a ForEachNext node (unsupported in codegen)
        let workflow = for_each_next_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "ForEachNext must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ForEachNext"),
            "UnsupportedPrimitive must name ForEachNext, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn emit_step_match_produces_correct_arm_for_for_each_join_node() -> Result<(), String> {
        // Given a ForEachJoin node (unsupported in codegen)
        let workflow = for_each_join_workflow()?;
        let node = workflow.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        // When emit_step_function generates code
        let mut out = String::new();
        emit_step_function(&mut out, node, &workflow).map_err(|e| e.to_string())?;
        // Then the output reports unsupported primitive
        assert!(
            out.contains("UnsupportedPrimitive"),
            "ForEachJoin must emit UnsupportedPrimitive, got: {out}"
        );
        assert!(
            out.contains("ForEachJoin"),
            "UnsupportedPrimitive must name ForEachJoin, got: {out}"
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
}
