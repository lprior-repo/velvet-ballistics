#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::{
        CodegenError, compare_generated_to_ir, compile_check_generated_rust, emit_action_boundary,
        emit_action_match_dispatch, emit_drive_function, emit_expr_function, emit_finish, emit_ids,
        emit_list_store_contract, emit_resource_contract, emit_rust_workflow, emit_step_function,
        emit_trybuild_fixture, emit_value_store_contract, format_generated_rust,
        validate_generated_subset,
    };
    use vb_core::{
        AccessorProgram, ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx,
        ConstValue, EngineError, EngineSignal, ExprProgram, PathSegment, ResourceContract, RunId,
        SlotIdx, SlotValue, StepBudget, StepIdx, Taint, ValueStore, WorkflowDigest, WorkflowParts,
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
            "{generated}\nfn main() {{\n    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    match slots.get_mut(usize::from({input}u16)) {{\n        Some(slot) => *slot = Some(SlotValue::I64(99)),\n        None => {{ println!(\"slot_out_of_bounds\"); std::process::exit(20); }}\n    }}\n    match drive(slots) {{\n        Err(DriveError::ActionSuspend {{ action_id, input_slot, .. }}) if action_id == {action}u16 && input_slot == {input}u16 => println!(\"generated_action_suspend:{action}:{input}\"),\n        other => {{ println!(\"unexpected:{{other:?}}\"); std::process::exit(21); }}\n    }}\n}}\n",
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

    fn generated_state_run_stdout(
        workflow: &CompiledWorkflow,
        name: &str,
        init_source: &str,
    ) -> Result<String, String> {
        let mut body = String::from(
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n",
        );
        body.push_str(init_source);
        body.push_str(
            r#"    let mut state = GeneratedRunState::new_with_taints(slots, slot_taints);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Finished(output)) => {
            println!("finished:{:?}:{:?}:events={:?}", output.value, output.taint, output.journal.len());
            let mut index = 0u16;
            while index < output.journal.len() {
                match output.journal.event(index) {
                    Some(event) => println!("event:{index}:{event:?}"),
                    None => println!("event:{index}:None"),
                }
                index = index.saturating_add(1);
            }
        }
        Ok(GeneratedRunStatus::Suspended(suspended)) => {
            println!("suspended:{:?}:events={:?}", suspended.suspension, suspended.journal.len());
            let mut index = 0u16;
            while index < suspended.journal.len() {
                match suspended.journal.event(index) {
                    Some(event) => println!("event:{index}:{event:?}"),
                    None => println!("event:{index}:None"),
                }
                index = index.saturating_add(1);
            }
        }
        Err(error) => println!("err:{error:?}"),
    }
"#,
        );
        generated_step_stdout(workflow, name, &body)
    }

    fn generated_step_stdout(
        workflow: &CompiledWorkflow,
        name: &str,
        body_source: &str,
    ) -> Result<String, String> {
        let generated = emit_rust_workflow(workflow).map_err(|e| e.to_string())?;
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("vb_codegen_step_{}_{}", std::process::id(), name))
            .tempdir()
            .map_err(|e| e.to_string())?;
        let source_path = temp_dir.path().join("generated_step.rs");
        let binary_path = temp_dir.path().join("generated_step_bin");
        let harness = format!("{generated}\nfn main() {{\n{body_source}\n}}\n");
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
            return Err(format!("generated step failed: {stdout}{stderr}"));
        }

        Ok(stdout)
    }

    fn generated_trace_stdout(
        workflow: &CompiledWorkflow,
        name: &str,
        init_source: &str,
    ) -> Result<String, String> {
        let generated = emit_rust_workflow(workflow).map_err(|e| e.to_string())?;
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("vb_codegen_trace_{}_{}", std::process::id(), name))
            .tempdir()
            .map_err(|e| e.to_string())?;
        let source_path = temp_dir.path().join("generated_trace.rs");
        let binary_path = temp_dir.path().join("generated_trace_bin");
        let harness = generated_trace_harness(&generated, workflow, init_source)?;
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
            return Err(format!("generated trace failed: {stdout}{stderr}"));
        }

        Ok(stdout)
    }

    fn generated_trace_harness(
        generated: &str,
        workflow: &CompiledWorkflow,
        init_source: &str,
    ) -> Result<String, String> {
        Ok(format!(
            "{generated}\nfn main() {{\n    const TRACE_STEP_LIMIT: usize = {};\n    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n{init_source}\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    let mut pc: u16 = {};\n    let mut journal = String::new();\n    let mut retry_attempt_total: u16 = 0;\n    let mut terminal = false;\n    for _step_index in 0..TRACE_STEP_LIMIT {{\n        journal.push_str(\"start:\");\n        journal.push_str(&pc.to_string());\n        journal.push('|');\n        let retry_attempt_before = retry_attempt_for_pc(pc, &slots).unwrap_or(0);\n        let outcome = match pc {{\n{}            _ => Err(DriveError::InvalidProgramCounter),\n        }};\n        match outcome {{\n            Ok(StepOutcome::Continue(next)) => {{\n                retry_attempt_total = retry_attempt_total.saturating_add(retry_attempt_before);\n                journal.push_str(\"continue:\");\n                journal.push_str(&next.to_string());\n                journal.push('|');\n                pc = next;\n            }}\n            Ok(StepOutcome::Finished(value)) => {{\n                journal.push_str(\"finished\");\n                println!(\"result:{{value:?}}\");\n                println!(\"final_pc:{{pc}}\");\n                println!(\"slots:{{slots:?}}\");\n                println!(\"journal:{{journal}}\");\n                println!(\"retry_attempt_total:{{retry_attempt_total}}\");\n                terminal = true;\n                break;\n            }}\n            Err(error) => {{\n                journal.push_str(\"error\");\n                println!(\"error:{{error:?}}\");\n                println!(\"final_pc:{{pc}}\");\n                println!(\"slots:{{slots:?}}\");\n                println!(\"journal:{{journal}}\");\n                println!(\"retry_attempt_total:{{retry_attempt_total}}\");\n                terminal = true;\n                break;\n            }}\n        }}\n    }}\n    if !terminal {{\n        journal.push_str(\"step_limit\");\n        println!(\"error:StepLimitExceeded\");\n        println!(\"final_pc:{{pc}}\");\n        println!(\"slots:{{slots:?}}\");\n        println!(\"journal:{{journal}}\");\n        println!(\"retry_attempt_total:{{retry_attempt_total}}\");\n        std::process::exit(22);\n    }}\n}}\n\nfn retry_attempt_for_pc(pc: u16, slots: &[Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Result<u16, DriveError> {{\n    match pc {{\n{}        _ => Ok(0),\n    }}\n}}\n",
            trace_step_limit(workflow)?,
            workflow.entry().get(),
            generated_trace_arms(workflow),
            generated_retry_trace_arms(workflow)
        ))
    }

    fn trace_step_limit(workflow: &CompiledWorkflow) -> Result<usize, String> {
        usize::from(workflow.node_count())
            .checked_add(1)
            .ok_or_else(|| String::from("trace step limit overflow"))
    }

    fn generated_trace_arms(workflow: &CompiledWorkflow) -> String {
        (0..workflow.node_count()).fold(String::new(), |mut arms, idx| {
            arms.push_str("            ");
            arms.push_str(&idx.to_string());
            arms.push_str(" => step_");
            arms.push_str(&idx.to_string());
            arms.push_str("(&mut slots, &mut slot_taints, &mut list_store, &mut object_store),\n");
            arms
        })
    }

    fn generated_retry_trace_arms(workflow: &CompiledWorkflow) -> String {
        (0..workflow.node_count()).fold(String::new(), |mut arms, idx| {
            if let Some(node) = workflow.node(StepIdx::new(idx))
                && let CompiledNodeKind::RetryCheck { policy_slot, .. } = node.kind
            {
                arms.push_str("        ");
                arms.push_str(&idx.to_string());
                arms.push_str(" => read_retry_state_from_slot(slots, ");
                arms.push_str(&policy_slot.get().to_string());
                arms.push_str(
                    ", CONTRACT_MAX_RETRY_ATTEMPTS).map(|state| state.current_attempt()),\n",
                );
            }
            arms
        })
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

    fn ir_drive_finished_output_with_init(
        workflow: &CompiledWorkflow,
        init: &[(SlotIdx, SlotValue, Taint)],
    ) -> Result<(SlotValue, Taint), String> {
        let mut run = new_run_frame(RunId::new(22), workflow).map_err(|e| e.to_string())?;
        init.iter().try_for_each(|(slot, value, taint)| {
            run.write_slot_with_taint(*slot, *value, *taint)
                .map_err(|e| e.to_string())
        })?;
        let mut store = ValueStore::new();
        let signal = run_until_blocked(workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|e| e.to_string())?;
        match signal {
            EngineSignal::Finished(value, taint) => Ok((value, taint)),
            other => Err(format!("expected finished signal, got {other:?}")),
        }
    }

    fn runtime_drive_error_string_with_init(
        workflow: &CompiledWorkflow,
        init: &[(SlotIdx, SlotValue, Taint)],
    ) -> Result<String, String> {
        let mut run = new_run_frame(RunId::new(23), workflow).map_err(|e| e.to_string())?;
        init.iter().try_for_each(|(slot, value, taint)| {
            run.write_slot_with_taint(*slot, *value, *taint)
                .map_err(|e| e.to_string())
        })?;
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

    fn primitive_retry_check_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_primitive_retry_check_exec"),
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
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(2),
                        exhausted: StepIdx::new(3),
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
                ConstValue::I64(retry_state_raw(1, 2)?),
                ConstValue::I64(99),
                ConstValue::I64(-1),
            ]
            .into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract {
                max_steps: 100,
                max_slots: 10,
                max_constants: 10,
                max_accessors: 0,
                max_expressions: 0,
                max_expr_stack: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                max_step_budget_per_tick: 100,
                max_transitions_per_tick: 100,
                max_blob_bytes: 0,
                max_ipc_payload_bytes: 0,
                max_retry_attempts: 3,
                max_fanout: 1,
                max_collect_items: 0,
                max_queue_depth: 0,
                max_journal_batch_bytes: 0,
                ..ResourceContract::DEFAULT
            },
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn retry_state_raw(attempt: u16, remaining: u16) -> Result<i64, String> {
        let attempt_bits = u64::from(attempt)
            .checked_shl(16)
            .ok_or_else(|| String::from("retry attempt shift overflow"))?;
        let packed = attempt_bits
            .checked_add(u64::from(remaining))
            .ok_or_else(|| String::from("retry state pack overflow"))?;
        i64::try_from(packed).map_err(|e| e.to_string())
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
            max_transitions_per_tick: 500,
            max_blob_bytes: 1024,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 3,
            max_fanout: 8,
            max_collect_items: 100,
            max_queue_depth: 64,
            max_journal_batch_bytes: 512,
            ..ResourceContract::DEFAULT
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
        // Then the output returns a typed action suspension instead of continuing.
        assert!(
            out.contains("ActionPending"),
            "Do node must emit ActionPending suspension, got: {out}"
        );
        assert!(
            out.contains("step: 0"),
            "Do node must preserve pc 0, got: {out}"
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
        emit_action_boundary(
            &mut out,
            StepIdx::new(2),
            ActionId::new(7),
            SlotIdx::new(3),
            Some(StepIdx::new(4)),
        )
        .map_err(|e| e.to_string())?;
        // Then the output reads the input slot and returns ActionSuspend
        assert!(
            out.contains("read_slot(slots, 3)"),
            "action boundary must read input slot 3, got: {out}"
        );
        assert!(
            out.contains("ActionPending { step: 2, action_id: 7, input_slot: 3, resume_pc: 4 }"),
            "action boundary must return typed ActionPending suspension, got: {out}"
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
            max_transitions_per_tick: 500,
            max_blob_bytes: 1024,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 3,
            max_fanout: 8,
            max_collect_items: 100,
            max_queue_depth: 64,
            max_journal_batch_bytes: 512,
            ..ResourceContract::DEFAULT
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
    fn generated_expression_equivalence_trace_matches_interpreter_state_and_replay()
    -> Result<(), String> {
        let workflow = primitive_expression_workflow()?;

        let mut run = new_run_frame(RunId::new(20), &workflow).map_err(|e| e.to_string())?;
        let mut budget = StepBudget::MAX;
        let mut store = ValueStore::new();
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();
        let signal = drive_deterministic_full(
            &workflow,
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
        assert_eq!(signal, RuntimeSignal::Finished(SlotValue::Bool(true)));
        assert_eq!(run.pc(), StepIdx::new(1));
        assert_eq!(
            run.initialized_slots().map_err(|e| e.to_string())?,
            vec![(
                SlotIdx::new(0),
                SlotValue::Bool(true),
                vb_core::Taint::Clean
            )]
        );

        let first_replay = generated_trace_stdout(&workflow, "expr_trace_one", "")?;
        let second_replay = generated_trace_stdout(&workflow, "expr_trace_two", "")?;
        let expected = "result:Bool(true)\nfinal_pc:1\nslots:[Some(Bool(true))]\njournal:start:0|continue:1|start:1|finished\nretry_attempt_total:0\n";
        assert_eq!(first_replay, expected);
        assert_eq!(second_replay, expected);
        Ok(())
    }

    #[test]
    fn generated_choose_type_error_trace_matches_exact_typed_error_and_pc() -> Result<(), String> {
        let workflow = non_boolean_choose_workflow()?;

        assert_boolean_number_type_mismatch(ir_drive_error(&workflow)?)?;
        let generated_stdout = generated_trace_stdout(&workflow, "choose_type_trace", "")?;

        assert_eq!(
            generated_stdout,
            "error:TypeMismatch { expected: \"boolean\", found: \"number\" }\nfinal_pc:0\nslots:[None]\njournal:start:0|error\nretry_attempt_total:0\n"
        );
        Ok(())
    }

    #[test]
    fn generated_retry_check_trace_reports_real_attempt_and_terminal_slot_values()
    -> Result<(), String> {
        let workflow = primitive_retry_check_workflow()?;

        let generated_stdout = generated_trace_stdout(&workflow, "retry_check_trace", "")?;

        assert_eq!(
            generated_stdout,
            "result:I64(99)\nfinal_pc:4\nslots:[Some(I64(65538)), Some(I64(99))]\njournal:start:0|continue:1|start:1|continue:2|start:2|continue:4|start:4|finished\nretry_attempt_total:1\n"
        );
        Ok(())
    }

    #[test]
    fn generated_subset_rejects_unsupported_control_primitives_with_exact_feature()
    -> Result<(), String> {
        let together_error = validate_generated_subset(&together_workflow()?)
            .err()
            .ok_or("TogetherStart unexpectedly accepted")?;
        assert_eq!(
            together_error.to_string(),
            "unsupported generated Rust IR feature: TogetherStart"
        );

        let reduce_error = validate_generated_subset(&reduce_workflow()?)
            .err()
            .ok_or("ReduceStart unexpectedly accepted")?;
        assert_eq!(
            reduce_error.to_string(),
            "unsupported generated Rust IR feature: ReduceStart"
        );

        let repeat_error = validate_generated_subset(&repeat_workflow()?)
            .err()
            .ok_or("RepeatStart unexpectedly accepted")?;
        assert_eq!(
            repeat_error.to_string(),
            "unsupported generated Rust IR feature: RepeatStart"
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
                assert_eq!(id.get(), 1, "runtime must return inserted empty tail");
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
        assert_error_display_contains(&CodegenError::FormatBufferOverflow, "buffer");
        assert_error_display_contains(
            &CodegenError::RustfmtFailed {
                detail: String::from("test"),
            },
            "rustfmt",
        );
        assert_error_display_contains(
            &CodegenError::CompileCheckFailed {
                detail: String::from("test"),
            },
            "compile",
        );
        assert_error_display_contains(
            &CodegenError::SemanticMismatch {
                detail: String::from("test"),
            },
            "semantic",
        );
        assert_error_display_contains(
            &CodegenError::Io(std::io::Error::other("io")),
            "codegen IO error",
        );
        assert_error_display_contains(
            &CodegenError::TrybuildFixture {
                detail: String::from("test"),
            },
            "trybuild",
        );
    }

    fn assert_error_display_contains(error: &CodegenError, keyword: &str) {
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
            .find('}')
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
    fn compare_generated_to_ir_rejects_wrong_action_boundary_count() -> Result<(), String> {
        let workflow = do_action_workflow()?;
        let source = emit_rust_workflow(&workflow)
            .map_err(|e| e.to_string())?
            .replace("Action boundary:", "Action boundary removed:");

        let detail = semantic_mismatch_detail(compare_generated_to_ir(&source, &workflow))?;
        assert_eq!(detail, "action count mismatch: generated has 0, IR has 1");
        Ok(())
    }

    #[test]
    fn compare_generated_to_ir_rejects_missing_terminal_result_pattern() -> Result<(), String> {
        let workflow = minimal_workflow()?;
        let source = emit_rust_workflow(&workflow)
            .map_err(|e| e.to_string())?
            .replace("StepOutcome::Finished", "StepOutcome::Terminated");

        let detail = semantic_mismatch_detail(compare_generated_to_ir(&source, &workflow))?;
        assert_eq!(detail, "generated source is missing terminal result return");
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
    fn contains_expression_codegen_rejects_missing_symbol_store() -> Result<(), String> {
        let workflow = unsupported_contains_expression_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "text helper contains requires runtime symbol store",
            "unsupported generated Rust IR feature: text helper contains requires runtime symbol store",
        )
    }

    #[test]
    fn accessor_traversal_codegen_is_supported() -> Result<(), String> {
        let workflow = unsupported_accessor_traversal_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("object_field_scan"),
            "missing object field traversal: {source}"
        );
        Ok(())
    }

    fn assert_unsupported_ir<T>(
        result: Result<T, CodegenError>,
        expected_feature: &'static str,
        expected_message: &str,
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
            source.contains(
                "stack.push_tainted(read_slot(slots, 0)?, read_taint(slot_taints, 0)?)?;"
            ),
            "empty accessor must preserve root-slot value and taint"
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
        // Then the Do step function contains ActionPending with the correct action id
        assert!(
            source.contains("ActionPending { step: 0, action_id: 5"),
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
            max_transitions_per_tick: 500,
            max_blob_bytes: 65536,
            max_ipc_payload_bytes: 2048,
            max_retry_attempts: 5,
            max_fanout: 16,
            max_collect_items: 200,
            max_queue_depth: 128,
            max_journal_batch_bytes: 1024,
            ..ResourceContract::DEFAULT
        };
        // When emit_resource_contract writes the constants
        let mut out = String::new();
        emit_resource_contract(&mut out, contract).map_err(|e| e.to_string())?;
        // Then all 16 fields are present
        assert_resource_contract_fields(&out)?;
        Ok(())
    }

    // Verify that the drive loop enforces the per-tick step budget.
    #[test]
    fn drive_function_has_no_step_budget_enforcement() -> Result<(), String> {
        // Given a minimal workflow
        let workflow = minimal_workflow()?;
        // When emit_drive_function generates the loop
        let mut out = String::new();
        emit_drive_function(&mut out, &workflow).map_err(|e| e.to_string())?;
        // Then the generated loop decrements a bounded budget and returns the
        // typed exhaustion error rather than spinning forever.
        assert!(
            out.contains("step_budget_remaining: u64 = CONTRACT_MAX_STEP_BUDGET_PER_TICK"),
            "drive function must initialize a bounded step budget, got: {out}"
        );
        assert!(
            out.contains("DriveError::StepBudgetExhausted"),
            "drive function must preserve the typed budget exhaustion error, got: {out}"
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
            "generated source contains forbidden constructs: {violations:?}"
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
            next: Some(StepIdx::new(1)),
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
        let step_count = u16::try_from(
            source
                .lines()
                .filter(|l| l.trim().starts_with("fn step_"))
                .count(),
        )
        .map_err(|e| e.to_string())?;
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
    fn jump_to_self_produces_infinite_loop_without_budget_guard() {
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

    fn single_unsupported_node_workflow(
        name: &'static str,
        kind: CompiledNodeKind,
    ) -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from(name),
            digest: WorkflowDigest::from_bytes([0xC7; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind,
                },
                choose_finish_node(1),
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

    #[test]
    fn generated_subset_rejects_each_collect_reduce_repeat_variant_exactly() -> Result<(), String> {
        let cases = [
            (
                single_unsupported_node_workflow(
                    "test_collect_page_reject",
                    CompiledNodeKind::CollectPage {
                        collector_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
                    },
                )?,
                "CollectPage",
            ),
            (
                single_unsupported_node_workflow(
                    "test_collect_next_reject",
                    CompiledNodeKind::CollectNext {
                        collector_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
                    },
                )?,
                "CollectNext",
            ),
            (
                single_unsupported_node_workflow(
                    "test_collect_finish_reject",
                    CompiledNodeKind::CollectFinish {
                        collector_slot: SlotIdx::new(0),
                    },
                )?,
                "CollectFinish",
            ),
            (
                single_unsupported_node_workflow(
                    "test_reduce_next_reject",
                    CompiledNodeKind::ReduceNext {
                        iterator_slot: SlotIdx::new(0),
                        accumulator: SlotIdx::new(1),
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
                    },
                )?,
                "ReduceNext",
            ),
            (
                single_unsupported_node_workflow(
                    "test_reduce_finish_reject",
                    CompiledNodeKind::ReduceFinish {
                        accumulator: SlotIdx::new(0),
                    },
                )?,
                "ReduceFinish",
            ),
            (
                single_unsupported_node_workflow(
                    "test_repeat_attempt_reject",
                    CompiledNodeKind::RepeatAttempt {
                        attempt_slot: SlotIdx::new(0),
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
                    },
                )?,
                "RepeatAttempt",
            ),
            (
                single_unsupported_node_workflow(
                    "test_repeat_check_reject",
                    CompiledNodeKind::RepeatCheck {
                        attempt_slot: SlotIdx::new(0),
                        done: StepIdx::new(1),
                    },
                )?,
                "RepeatCheck",
            ),
            (
                single_unsupported_node_workflow(
                    "test_repeat_finish_reject",
                    CompiledNodeKind::RepeatFinish {
                        result: SlotIdx::new(0),
                    },
                )?,
                "RepeatFinish",
            ),
        ];

        cases.iter().try_for_each(|(workflow, feature)| {
            let expected = format!("unsupported generated Rust IR feature: {feature}");
            assert_unsupported_ir(validate_generated_subset(workflow), feature, &expected)
        })
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
    fn build_object_generated_step_joins_mixed_taint_to_secret() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_object_taint",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(10));\n    slots[1] = Some(SlotValue::Bool(true));\n    slots[2] = Some(SlotValue::Symbol(2));\n    taints[0] = Taint::Clean;\n    taints[1] = Taint::DerivedFromSecret;\n    taints[2] = Taint::Secret;\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(_) => println!(\"slot={:?};taint={:?}\", slots[3], taints[3]),\n        Err(error) => println!(\"err:{error:?}\"),\n    }",
        )?;
        assert!(
            stdout.contains("slot=Some(Object(0));taint=Secret"),
            "BuildObject must join mixed taint to Secret, got: {stdout}"
        );
        Ok(())
    }

    #[test]
    fn build_object_generated_step_missing_field_reports_slot_null() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_object_missing_field",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(10));\n    slots[2] = Some(SlotValue::Symbol(2));\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Err(DriveError::SlotNull) => println!(\"err:SlotNull\"),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n        Ok(_) => println!(\"unexpected_ok\"),\n    }",
        )?;
        assert_eq!(stdout, "err:SlotNull\n");
        Ok(())
    }

    #[test]
    fn build_object_store_bounds_report_object_store_overflow() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_object_store_bounds",
            "    let mut object_store = ObjectStore::new();\n    let field = ObjectField { key: 0, value: SlotValue::Null, taint: Taint::Clean };\n    match object_store.insert_fields(&[field, field, field, field]) {\n        Err(DriveError::ObjectStoreOverflow) => println!(\"err:ObjectStoreOverflow\"),\n        other => println!(\"unexpected:{other:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "err:ObjectStoreOverflow\n");
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
    fn list_store_contract_wrapper_matches_value_store_contract() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let mut wrapper = String::new();
        let mut direct = String::new();

        emit_list_store_contract(&mut wrapper, &workflow).map_err(|e| e.to_string())?;
        emit_value_store_contract(&mut direct, &workflow).map_err(|e| e.to_string())?;

        assert_eq!(wrapper, direct);
        assert!(
            wrapper.contains("LIST_STORE_RECORD_CAPACITY")
                && wrapper.contains("LIST_STORE_VALUE_CAPACITY"),
            "list-store contract wrapper must emit fixed generated list capacities, got: {wrapper}"
        );
        Ok(())
    }

    #[test]
    fn build_list_generated_step_preserves_clean_taint() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_list_clean_taint",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(10));\n    slots[1] = Some(SlotValue::Bool(true));\n    slots[2] = Some(SlotValue::Symbol(2));\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(_) => println!(\"slot={:?};taint={:?}\", slots[3], taints[3]),\n        Err(error) => println!(\"err:{error:?}\"),\n    }",
        )?;
        assert!(
            stdout.contains("slot=Some(List(0));taint=Clean"),
            "BuildList clean inputs must produce clean output, got: {stdout}"
        );
        Ok(())
    }

    #[test]
    fn build_list_generated_step_joins_secret_taint() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_list_secret_taint",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(10));\n    slots[1] = Some(SlotValue::Bool(true));\n    slots[2] = Some(SlotValue::Symbol(2));\n    taints[1] = Taint::Secret;\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(_) => println!(\"slot={:?};taint={:?}\", slots[3], taints[3]),\n        Err(error) => println!(\"err:{error:?}\"),\n    }",
        )?;
        assert!(
            stdout.contains("slot=Some(List(0));taint=Secret"),
            "BuildList secret input must taint output Secret, got: {stdout}"
        );
        Ok(())
    }

    #[test]
    fn build_list_generated_step_missing_item_reports_slot_null() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_list_missing_item",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(10));\n    slots[2] = Some(SlotValue::Symbol(2));\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Err(DriveError::SlotNull) => println!(\"err:SlotNull\"),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n        Ok(_) => println!(\"unexpected_ok\"),\n    }",
        )?;
        assert_eq!(stdout, "err:SlotNull\n");
        Ok(())
    }

    #[test]
    fn build_list_store_bounds_report_list_store_overflow() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_list_store_bounds",
            "    let mut list_store = ListStore::new();\n    match list_store.insert_items_with_taints(\n        &[SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3), SlotValue::I64(4)],\n        &[Taint::Clean, Taint::Clean, Taint::Clean, Taint::Clean],\n    ) {\n        Err(DriveError::ListStoreOverflow) => println!(\"err:ListStoreOverflow\"),\n        other => println!(\"unexpected:{other:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "err:ListStoreOverflow\n");
        Ok(())
    }

    #[test]
    fn build_list_store_taint_length_mismatch_reports_typed_failure() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_list_taint_mismatch",
            "    let mut list_store = ListStore::new();\n    match list_store.insert_items_with_taints(&[SlotValue::I64(1)], &[]) {\n        Err(DriveError::InvalidCompiledWorkflow { reason }) => {\n            println!(\"err:InvalidCompiledWorkflow:{reason}\");\n        }\n        other => println!(\"unexpected:{other:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "err:InvalidCompiledWorkflow:list value/taint length mismatch\n"
        );
        Ok(())
    }

    #[test]
    fn generated_list_expect_reports_exact_type_mismatch() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "build_list_exact_type_mismatch",
            "    match expect_list_value(SlotValue::I64(7)) {\n        Err(DriveError::TypeMismatch { expected, found }) => {\n            println!(\"err:TypeMismatch:{expected}:{found}\");\n        }\n        other => println!(\"unexpected:{other:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "err:TypeMismatch:list:number\n");
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
            out.contains("read_retry_state_from_slot(slots, 0, CONTRACT_MAX_RETRY_ATTEMPTS)"),
            "RetryCheck must decode typed retry state from policy slot, got: {out}"
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
            out.contains("retry_check_target(_retry_state.current_attempt(), CONTRACT_MAX_RETRY_ATTEMPTS, 1, 2)"),
            "RetryCheck body branch must target step 1, got: {out}"
        );
        assert!(
            out.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
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
            out.contains("_retry_state.current_attempt()"),
            "RetryCheck must read invariant-checked retry attempt, got: {out}"
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
            out.contains("read_retry_state_from_slot"),
            "RetryCheck must route policy through typed decoder, got: {out}"
        );
        assert!(
            out.contains("read_retry_state_from_slot"),
            "RetryCheck must delegate type guard to decoder, got: {out}"
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
    fn starts_with_expression_rejects_missing_symbol_store() -> Result<(), String> {
        let workflow = starts_with_expression_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "text helper starts_with requires runtime symbol store",
            "unsupported generated Rust IR feature: text helper starts_with requires runtime symbol store",
        )
    }

    #[test]
    fn starts_with_expression_emit_fails_closed() -> Result<(), String> {
        let workflow = starts_with_expression_workflow()?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "text helper starts_with requires runtime symbol store",
            "unsupported generated Rust IR feature: text helper starts_with requires runtime symbol store",
        )
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
    fn ends_with_expression_rejects_missing_symbol_store() -> Result<(), String> {
        let workflow = ends_with_expression_workflow()?;
        assert_unsupported_ir(
            validate_generated_subset(&workflow),
            "text helper ends_with requires runtime symbol store",
            "unsupported generated Rust IR feature: text helper ends_with requires runtime symbol store",
        )
    }

    #[test]
    fn ends_with_expression_emit_fails_closed() -> Result<(), String> {
        let workflow = ends_with_expression_workflow()?;
        assert_unsupported_ir(
            emit_rust_workflow(&workflow),
            "text helper ends_with requires runtime symbol store",
            "unsupported generated Rust IR feature: text helper ends_with requires runtime symbol store",
        )
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

    fn has_generated_execution_workflow(search_value: i64) -> Result<CompiledWorkflow, String> {
        let expr = ExprProgram::try_from_ops(Box::new([
            vb_core::ExprOp::LoadSlot(SlotIdx::new(2)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Has,
        ]))
        .map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_has_generated_execution"),
            digest: WorkflowDigest::from_bytes([0xE2; 32]),
            nodes: vec![
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
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(3)),
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
                        result: SlotIdx::new(3),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(search_value)].into_boxed_slice(),
            slot_count: 4,
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
    fn has_expression_emits_list_contains_match() -> Result<(), String> {
        let workflow = has_expression_workflow()?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("SlotValue::List"),
            "Has must match SlotValue::List"
        );
        assert!(
            source.contains("list_contains_item"),
            "Has must scan list contents"
        );
        assert!(
            source.contains("SlotValue::Bool"),
            "Has must produce SlotValue::Bool result"
        );
        Ok(())
    }

    #[test]
    fn has_generated_execution_returns_true_for_present_item() -> Result<(), String> {
        let workflow = has_generated_execution_workflow(7)?;
        let stdout = generated_drive_stdout(
            &workflow,
            "has_present_item",
            "    slots[0] = Some(SlotValue::I64(7));\n    slots[1] = Some(SlotValue::I64(9));",
        )?;
        assert_eq!(stdout, "ok:Bool(true)\n");
        Ok(())
    }

    #[test]
    fn has_generated_execution_returns_false_for_absent_item() -> Result<(), String> {
        let workflow = has_generated_execution_workflow(8)?;
        let stdout = generated_drive_stdout(
            &workflow,
            "has_absent_item",
            "    slots[0] = Some(SlotValue::I64(7));\n    slots[1] = Some(SlotValue::I64(9));",
        )?;
        assert_eq!(stdout, "ok:Bool(false)\n");
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

    #[test]
    fn direct_expr_emit_preserves_fail_closed_helper_branches() -> Result<(), String> {
        let cases = [
            (
                unsupported_contains_expression_workflow()?,
                "text helper contains requires runtime symbol store",
            ),
            (
                starts_with_expression_workflow()?,
                "text helper starts_with requires runtime symbol store",
            ),
            (
                ends_with_expression_workflow()?,
                "text helper ends_with requires runtime symbol store",
            ),
        ];

        cases.iter().try_for_each(|(workflow, reason)| {
            let mut out = String::new();
            emit_expr_function(&mut out, vb_core::ExprIdx::new(0), workflow)
                .map_err(|e| e.to_string())?;
            assert!(
                out.contains("InvalidCompiledWorkflow") && out.contains(reason),
                "direct expression emission must keep fail-closed reason {reason}, got: {out}"
            );
            Ok::<(), String>(())
        })?;

        Ok(())
    }

    fn length_expression_workflow() -> Result<CompiledWorkflow, String> {
        direct_expression_workflow(
            "test_length",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::Length,
            ]),
            Box::new([]),
            1,
        )
    }

    fn empty_expression_workflow() -> Result<CompiledWorkflow, String> {
        direct_expression_workflow(
            "test_empty",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::Empty,
            ]),
            Box::new([]),
            1,
        )
    }

    #[test]
    fn direct_expr_emit_keeps_length_and_empty_typed_runtime_guards() -> Result<(), String> {
        let cases = [
            (
                length_expression_workflow()?,
                "list or object",
                "SlotValue::I64(_len)",
            ),
            (
                empty_expression_workflow()?,
                "list, object, or null",
                "SlotValue::Bool(_is_empty)",
            ),
        ];

        cases
            .iter()
            .try_for_each(|(workflow, expected_type, result)| {
                let mut out = String::new();
                emit_expr_function(&mut out, vb_core::ExprIdx::new(0), workflow)
                    .map_err(|e| e.to_string())?;
                assert!(
                    out.contains(expected_type) && out.contains(result),
                    "direct expression emission must retain typed guard {expected_type}, got: {out}"
                );
                Ok::<(), String>(())
            })?;

        Ok(())
    }

    // ========================================================================
    // Tests for Length/Empty helper support (bug-fix: false positives removed)
    // These tests verify that validate_generated_subset no longer rejects Length
    // and Empty helpers, and that emit_rust_workflow generates correct code.
    // ========================================================================

    /// B-01: validate_generated_subset must accept Length helper.
    /// Before bug-fix: returns UnsupportedIr error.
    /// After bug-fix: returns Ok(()).
    #[test]
    fn validate_generated_subset_accepts_length_helper() -> Result<(), String> {
        let workflow = length_expression_workflow()?;
        // The bug causes Length to be incorrectly flagged as unsupported.
        // After fix, this must succeed.
        validate_generated_subset(&workflow)
            .map_err(|e| format!("Length helper must be supported but got error: {e}"))?;
        Ok(())
    }

    /// B-02: validate_generated_subset must accept Empty helper.
    /// Before bug-fix: returns UnsupportedIr error.
    /// After bug-fix: returns Ok(()).
    #[test]
    fn validate_generated_subset_accepts_empty_helper() -> Result<(), String> {
        let workflow = empty_expression_workflow()?;
        // The bug causes Empty to be incorrectly flagged as unsupported.
        // After fix, this must succeed.
        validate_generated_subset(&workflow)
            .map_err(|e| format!("Empty helper must be supported but got error: {e}"))?;
        Ok(())
    }

    /// B-03: emit_expr_function generates list_item_count for Length on List.
    #[test]
    fn length_list_expression_emits_list_item_count() -> Result<(), String> {
        // Use direct_expression_workflow to create a Length expression
        let workflow = direct_expression_workflow(
            "test_length_list",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::Length,
            ]),
            Box::new([]),
            1,
        )?;
        let mut out = String::new();
        emit_expr_function(&mut out, vb_core::ExprIdx::new(0), &workflow)
            .map_err(|e| e.to_string())?;
        // Length on List must emit list_item_count
        assert!(
            out.contains("list_item_count"),
            "Length on List must emit list_item_count, got: {out}"
        );
        assert!(
            out.contains("SlotValue::List"),
            "Length codegen must check SlotValue::List variant, got: {out}"
        );
        assert!(
            out.contains("SlotValue::I64"),
            "Length result must be SlotValue::I64, got: {out}"
        );
        Ok(())
    }

    /// B-04: emit_expr_function generates object_field_count for Length on Object.
    #[test]
    fn length_object_expression_emits_object_field_count() -> Result<(), String> {
        // Length on Object must emit object_field_count
        let workflow = direct_expression_workflow(
            "test_length_object",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::Length,
            ]),
            Box::new([]),
            1,
        )?;
        let mut out = String::new();
        emit_expr_function(&mut out, vb_core::ExprIdx::new(0), &workflow)
            .map_err(|e| e.to_string())?;
        assert!(
            out.contains("object_field_count"),
            "Length on Object must emit object_field_count, got: {out}"
        );
        assert!(
            out.contains("SlotValue::Object"),
            "Length codegen must check SlotValue::Object variant, got: {out}"
        );
        Ok(())
    }

    /// B-05: emit_expr_function generates list_item_count == 0 for Empty on List.
    #[test]
    fn empty_list_expression_emits_list_item_count_eq_zero() -> Result<(), String> {
        let workflow = direct_expression_workflow(
            "test_empty_list",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::Empty,
            ]),
            Box::new([]),
            1,
        )?;
        let mut out = String::new();
        emit_expr_function(&mut out, vb_core::ExprIdx::new(0), &workflow)
            .map_err(|e| e.to_string())?;
        // Empty on List must emit list_item_count(...)? == 0
        assert!(
            out.contains("list_item_count"),
            "Empty on List must emit list_item_count, got: {out}"
        );
        assert!(
            out.contains("== 0"),
            "Empty on List must check == 0, got: {out}"
        );
        assert!(
            out.contains("SlotValue::Bool"),
            "Empty result must be SlotValue::Bool, got: {out}"
        );
        Ok(())
    }

    /// B-06: emit_expr_function generates object_field_count == 0 for Empty on Object.
    #[test]
    fn empty_object_expression_emits_object_field_count_eq_zero() -> Result<(), String> {
        let workflow = direct_expression_workflow(
            "test_empty_object",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::Empty,
            ]),
            Box::new([]),
            1,
        )?;
        let mut out = String::new();
        emit_expr_function(&mut out, vb_core::ExprIdx::new(0), &workflow)
            .map_err(|e| e.to_string())?;
        // Empty on Object must emit object_field_count(...)? == 0
        assert!(
            out.contains("object_field_count"),
            "Empty on Object must emit object_field_count, got: {out}"
        );
        assert!(
            out.contains("== 0"),
            "Empty on Object must check == 0, got: {out}"
        );
        Ok(())
    }

    /// B-07: emit_expr_function generates matches!(v, Null) for Empty on Null.
    #[test]
    fn empty_null_expression_emits_matches_null() -> Result<(), String> {
        let workflow = direct_expression_workflow(
            "test_empty_null",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::Empty,
            ]),
            Box::new([]),
            1,
        )?;
        let mut out = String::new();
        emit_expr_function(&mut out, vb_core::ExprIdx::new(0), &workflow)
            .map_err(|e| e.to_string())?;
        // Empty on Null must emit matches!(v, Null) => true
        assert!(
            out.contains("SlotValue::Null"),
            "Empty on Null must check SlotValue::Null variant, got: {out}"
        );
        assert!(
            out.contains("true"),
            "Empty on Null must return true, got: {out}"
        );
        assert!(
            out.contains("SlotValue::Bool"),
            "Empty result must be SlotValue::Bool, got: {out}"
        );
        Ok(())
    }

    fn direct_expression_workflow(
        name: &'static str,
        ops: Box<[vb_core::ExprOp]>,
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
                        expr: vb_core::ExprIdx::new(0),
                    },
                },
                choose_finish_node(1),
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

    #[test]
    fn direct_expr_emit_covers_allocating_helper_branches() -> Result<(), String> {
        let append = direct_expression_workflow(
            "test_direct_append_emit",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::Append,
            ]),
            vec![ConstValue::I64(7)].into_boxed_slice(),
            1,
        )?;
        let append_if = direct_expression_workflow(
            "test_direct_append_if_emit",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
                vb_core::ExprOp::AppendIf,
            ]),
            vec![ConstValue::I64(7), ConstValue::Bool(true)].into_boxed_slice(),
            1,
        )?;
        let merge = direct_expression_workflow(
            "test_direct_merge_emit",
            Box::new([
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::LoadSlot(SlotIdx::new(1)),
                vb_core::ExprOp::Merge,
            ]),
            Box::new([]),
            2,
        )?;

        let cases = [
            (append, "append_list_item"),
            (append_if, "clone_list_items"),
            (merge, "merge_object_records"),
        ];
        cases.iter().try_for_each(|(workflow, expected)| {
            let mut out = String::new();
            emit_expr_function(&mut out, vb_core::ExprIdx::new(0), workflow)
                .map_err(|e| e.to_string())?;
            assert!(
                out.contains(expected),
                "direct expression emission must include helper {expected}, got: {out}"
            );
            Ok::<(), String>(())
        })
    }

    #[test]
    fn direct_expr_emit_covers_mul_and_missing_expression_branches() -> Result<(), String> {
        let mul = direct_expression_workflow(
            "test_direct_mul_emit",
            Box::new([
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
                vb_core::ExprOp::Mul,
            ]),
            vec![ConstValue::I64(6), ConstValue::I64(7)].into_boxed_slice(),
            1,
        )?;

        let mut mul_out = String::new();
        emit_expr_function(&mut mul_out, vb_core::ExprIdx::new(0), &mul)
            .map_err(|e| e.to_string())?;
        assert!(
            mul_out.contains("checked_mul") && mul_out.contains("IntegerOverflow"),
            "Mul emission must use checked multiplication, got: {mul_out}"
        );

        let mut missing_expr_out = String::new();
        emit_expr_function(&mut missing_expr_out, vb_core::ExprIdx::new(1), &mul)
            .map_err(|e| e.to_string())?;
        assert!(
            missing_expr_out.contains("Err(DriveError::ExprOutOfBounds { expr: 1 })"),
            "missing expression emission must preserve the exact missing expression index, got: {missing_expr_out}"
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
    fn sum_expression_codegen_is_supported() -> Result<(), String> {
        let workflow = sum_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("sum_list_items"),
            "missing sum helper: {source}"
        );
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
    fn unique_expression_codegen_is_supported() -> Result<(), String> {
        let workflow = unique_expression_workflow()?;
        validate_generated_subset(&workflow).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        assert!(
            source.contains("unique_list_items"),
            "missing unique helper: {source}"
        );
        Ok(())
    }

    // --- Cross-cutting: runtime-store-free helper ops pass validation and emit ---

    #[test]
    fn runtime_store_free_helper_ops_together_pass_validation() -> Result<(), String> {
        // Build one workflow with helper ops that do not need symbol strings.
        let ops_has = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
            vb_core::ExprOp::Has,
        ];
        let ops_exists = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Exists,
        ];
        let ops_sum = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Sum,
        ];
        let ops_unique = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Unique,
        ];
        let ops_count = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Count,
        ];

        let expr_has =
            ExprProgram::try_from_ops(ops_has.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_exists =
            ExprProgram::try_from_ops(ops_exists.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_sum =
            ExprProgram::try_from_ops(ops_sum.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_unique =
            ExprProgram::try_from_ops(ops_unique.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_count =
            ExprProgram::try_from_ops(ops_count.into_boxed_slice()).map_err(|e| e.to_string())?;
        let expr_has_again = ExprProgram::try_from_ops(
            vec![
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::LoadConst(ConstIdx::new(1)),
                vb_core::ExprOp::Has,
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let expr_exists_again = ExprProgram::try_from_ops(
            vec![
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::Exists,
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let expr_count_again = ExprProgram::try_from_ops(
            vec![
                vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
                vb_core::ExprOp::Count,
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;

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
                expr_has,
                expr_exists,
                expr_sum,
                expr_unique,
                expr_count,
                expr_has_again,
                expr_exists_again,
                expr_count_again,
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
            "must generate eval_expr_0 (Has)"
        );
        assert!(
            source.contains("fn eval_expr_1"),
            "must generate eval_expr_1 (Exists)"
        );
        assert!(
            source.contains("fn eval_expr_2"),
            "must generate eval_expr_2 (Sum)"
        );
        assert!(
            source.contains("fn eval_expr_3"),
            "must generate eval_expr_3 (Unique)"
        );
        assert!(
            source.contains("fn eval_expr_4"),
            "must generate eval_expr_4 (Count)"
        );
        assert!(
            source.contains("fn eval_expr_5"),
            "must generate eval_expr_5 (Has repeated)"
        );
        assert!(
            source.contains("fn eval_expr_6"),
            "must generate eval_expr_6 (Exists repeated)"
        );
        assert!(
            source.contains("fn eval_expr_7"),
            "must generate eval_expr_7 (Count repeated)"
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
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(5),
                    input: SlotIdx::new(0),
                },
            },
            finish_node(1),
        ];
        let wf = make_step_workflow(nodes, 2);
        let code = emit_node_in_wf(StepIdx::new(0), &wf);
        assert!(
            code.contains("ActionPending"),
            "Do should emit ActionPending suspension: {code}"
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
            code.contains("WaitUntil { step: 0, deadline_slot: 0, resume_pc: 1 }"),
            "should suspend with wait-until metadata: {code}"
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
            code.contains(
                "WaitEvent { step: 0, event_slot: 0, timeout_slot: Some(1), resume_pc: 1 }"
            ),
            "should suspend with wait-event metadata: {code}"
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
    fn step_do_without_next_emits_missing_next_after_input_read() -> Result<(), String> {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(7),
                input: SlotIdx::new(1),
            },
        }];
        let wf = make_step_workflow(nodes, 2);
        let mut out = String::new();
        let node = wf.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        emit_step_function(&mut out, node, &wf).map_err(|e| e.to_string())?;
        assert!(
            out.contains("read_slot(slots, 1)"),
            "Do must still validate input slot before suspension: {out}"
        );
        assert!(
            out.contains("MissingNextStep"),
            "Do without next must emit MissingNextStep: {out}"
        );
        Ok(())
    }

    #[test]
    fn step_wait_event_with_timeout_without_next_emits_missing_next() -> Result<(), String> {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: Some(SlotIdx::new(1)),
            },
        }];
        let wf = make_step_workflow(nodes, 2);
        let mut out = String::new();
        let node = wf.node(StepIdx::new(0)).ok_or("node 0 missing")?;
        emit_step_function(&mut out, node, &wf).map_err(|e| e.to_string())?;
        assert!(out.contains("_event"), "should read event: {out}");
        assert!(out.contains("_timeout"), "should read timeout: {out}");
        assert!(
            out.contains("MissingNextStep"),
            "WaitEvent with timeout and no next must emit MissingNextStep: {out}"
        );
        Ok(())
    }

    #[test]
    fn step_ask_with_next_emits_resume_metadata() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(1),
                timeout_slot: Some(SlotIdx::new(2)),
            },
        };
        let code = emit_first_node(node, 5);
        assert!(
            code.contains(
                "AskPending { step: 0, prompt_slot: 1, timeout_slot: Some(2), resume_pc: 1 }"
            ),
            "Ask must preserve prompt, timeout, and resume metadata: {code}"
        );
    }

    #[test]
    fn field_accessor_generated_step_joins_root_and_field_taint() -> Result<(), String> {
        let expr = ExprProgram::try_from_ops(
            vec![vb_core::ExprOp::LoadAccessor(vb_core::AccessorIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let wf = make_step_workflow_with_symbols(
            vec![
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
            ],
            2,
            1,
            Box::new([]),
            vec![expr].into_boxed_slice(),
            vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: vec![PathSegment::Field(vb_core::SymbolId::new(0))].into_boxed_slice(),
            }]
            .into_boxed_slice(),
        );
        let stdout = generated_step_stdout(
            &wf,
            "field_accessor_taint_join",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    let field = ObjectField { key: 0, value: SlotValue::I64(7), taint: Taint::Secret };\n    let object = object_store.insert_fields(&[field]).map_err(|error| format!(\"setup:{error:?}\"));\n    match object {\n        Ok(handle) => {\n            slots[0] = Some(SlotValue::Object(handle));\n            taints[0] = Taint::DerivedFromSecret;\n            match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n                Ok(_) => println!(\"slot={:?};taint={:?}\", slots[1], taints[1]),\n                Err(error) => println!(\"err:{error:?}\"),\n            }\n        }\n        Err(error) => println!(\"{error}\"),\n    }",
        )?;
        assert_eq!(stdout, "slot=Some(I64(7));taint=Secret\n");
        Ok(())
    }

    #[test]
    fn list_accessor_generated_step_joins_root_and_item_taint() -> Result<(), String> {
        let expr = ExprProgram::try_from_ops(
            vec![vb_core::ExprOp::LoadAccessor(vb_core::AccessorIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let wf = make_step_workflow_with_symbols(
            vec![
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
            ],
            2,
            0,
            Box::new([]),
            vec![expr].into_boxed_slice(),
            vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: vec![PathSegment::Index(0)].into_boxed_slice(),
            }]
            .into_boxed_slice(),
        );
        let stdout = generated_step_stdout(
            &wf,
            "list_accessor_taint_join",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    let list = list_store.insert_items_with_taints(&[SlotValue::I64(9)], &[Taint::Secret]).map_err(|error| format!(\"setup:{error:?}\"));\n    match list {\n        Ok(handle) => {\n            slots[0] = Some(SlotValue::List(handle));\n            taints[0] = Taint::DerivedFromSecret;\n            match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n                Ok(_) => println!(\"slot={:?};taint={:?}\", slots[1], taints[1]),\n                Err(error) => println!(\"err:{error:?}\"),\n            }\n        }\n        Err(error) => println!(\"{error}\"),\n    }",
        )?;
        assert_eq!(stdout, "slot=Some(I64(9));taint=Secret\n");
        Ok(())
    }

    #[test]
    fn append_helper_reports_overflow_when_capacity_is_full() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "append_helper_overflow",
            "    let mut list_store = ListStore::new();\n    let list = list_store.insert_items_with_taints(\n        &[SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],\n        &[Taint::Clean, Taint::Clean, Taint::Clean],\n    );\n    match list {\n        Ok(handle) => match append_list_item(&mut list_store, handle, SlotValue::I64(4), Taint::Clean) {\n            Err(DriveError::ListStoreOverflow) => println!(\"err:ListStoreOverflow\"),\n            other => println!(\"unexpected:{other:?}\"),\n        },\n        Err(error) => println!(\"setup:{error:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "err:ListStoreOverflow\n");
        Ok(())
    }

    #[test]
    fn unique_helper_uses_checked_prefix_slice_and_preserves_first_taint() -> Result<(), String> {
        let workflow = build_list_multi_item_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "unique_helper_checked_slice",
            "    let mut list_store = ListStore::new();\n    list_store.records[0] = Some(ListRecord { start: 0, len: 3 });\n    list_store.values[0] = SlotValue::I64(1);\n    list_store.values[1] = SlotValue::I64(1);\n    list_store.values[2] = SlotValue::I64(2);\n    list_store.taints[0] = Taint::Secret;\n    list_store.taints[1] = Taint::Clean;\n    list_store.taints[2] = Taint::DerivedFromSecret;\n    list_store.record_len = 1;\n    list_store.value_len = 0;\n    match unique_list_items(&mut list_store, 0) {\n        Ok(unique) => match (list_store.value_at(unique, 0), list_store.value_at(unique, 1), list_item_count(&list_store, unique)) {\n            (Ok((first, first_taint)), Ok((second, second_taint)), Ok(count)) => {\n                println!(\"count={count};first={first:?}:{first_taint:?};second={second:?}:{second_taint:?}\");\n            }\n            other => println!(\"unexpected:{other:?}\"),\n        },\n        Err(error) => println!(\"err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "count=2;first=I64(1):Secret;second=I64(2):DerivedFromSecret\n"
        );
        Ok(())
    }

    #[test]
    fn merge_helper_reports_overflow_for_disjoint_full_objects() -> Result<(), String> {
        let workflow = build_object_multi_field_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "merge_helper_overflow",
            "    let mut fields = [None; OBJECT_STORE_FIELD_CAPACITY];\n    let existing = ObjectField { key: 0, value: SlotValue::I64(1), taint: Taint::Clean };\n    for index in 0..OBJECT_STORE_FIELD_CAPACITY {\n        match fields.get_mut(index) {\n            Some(slot) => *slot = Some(existing),\n            None => println!(\"setup:index\"),\n        }\n    }\n    let new_field = ObjectField { key: 99, value: SlotValue::I64(9), taint: Taint::Clean };\n    match upsert_object_field(&mut fields, 3, new_field) {\n        Err(DriveError::ObjectStoreOverflow) => println!(\"err:ObjectStoreOverflow\"),\n        other => println!(\"unexpected:{other:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "err:ObjectStoreOverflow\n");
        Ok(())
    }

    #[test]
    fn collect_nodes_are_rejected_by_generated_subset_validation() -> Result<(), String> {
        let error = validate_generated_subset(&collect_workflow()?)
            .err()
            .ok_or("CollectStart unexpectedly accepted")?;
        assert!(
            matches!(error, CodegenError::UnsupportedIr { feature } if feature == "CollectStart"),
            "CollectStart must return exact UnsupportedIr feature, got: {error}"
        );
        assert_eq!(
            error.to_string(),
            "unsupported generated Rust IR feature: CollectStart"
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
        assert!(code.contains('3'), "should contain slot index 3: {code}");
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
        assert!(
            code.contains("read_retry_state_from_slot"),
            "should read typed retry policy slot: {code}"
        );
        assert!(
            code.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
            "should check retry limit: {code}"
        );
        assert!(
            code.contains("retry_check_target"),
            "should continue to body: {code}"
        );
        assert!(
            code.contains("1, 2"),
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
            code.contains("_retry_state.current_attempt()"),
            "should extract retry attempt: {code}"
        );
        assert!(
            code.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
            "should define limit: {code}"
        );
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
            line.contains('3'),
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
            max_transitions_per_tick: 1000,
            max_input_bytes: 512,
            max_output_bytes: 256,
            max_blob_bytes: 4096,
            max_ipc_payload_bytes: 128,
            max_retry_attempts: 1,
            max_fanout: 2,
            max_collect_items: 50,
            max_queue_depth: 10,
            max_journal_batch_bytes: 2048,
            ..ResourceContract::DEFAULT
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
            max_transitions_per_tick: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            max_blob_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            ..ResourceContract::DEFAULT
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
    /// ActionPending suspension with the exact pc, action_id, input_slot, and resume pc.
    #[test]
    fn emit_action_boundary_correct_ids() -> Result<(), String> {
        let mut out = String::new();
        let action = ActionId::new(42);
        let input = SlotIdx::new(7);
        emit_action_boundary(
            &mut out,
            StepIdx::new(9),
            action,
            input,
            Some(StepIdx::new(10)),
        )
        .map_err(|e| e.to_string())?;

        assert!(
            out.contains("Action boundary: action_id=42, input_slot=7"),
            "should emit comment with action_id=42 and input_slot=7, got: {out}"
        );
        assert!(
            out.contains("read_slot(slots, 7)"),
            "should read input slot 7, got: {out}"
        );
        assert!(
            out.contains("ActionPending"),
            "should emit ActionPending suspension"
        );
        assert!(out.contains("step: 9"), "should embed step 9 in suspension");
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
        emit_action_boundary(
            &mut out,
            StepIdx::new(0),
            ActionId::new(0),
            SlotIdx::new(0),
            Some(StepIdx::new(1)),
        )
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
        emit_action_boundary(
            &mut out,
            StepIdx::new(65533),
            ActionId::new(65535),
            SlotIdx::new(65534),
            Some(StepIdx::new(65535)),
        )
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
                        entry: StepIdx::new(2),
                        join: StepIdx::new(3),
                        accumulator: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(3)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherJoin {
                        branch_count: 1,
                        accumulator: SlotIdx::new(0),
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
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
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

        // Must contain typed suspension to step_2 after the wait resolves.
        assert!(
            source.contains("WaitUntil { step: 1, deadline_slot: 0, resume_pc: 2 }"),
            "WaitUntil must suspend with resume pc 2"
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
            source.contains("read_retry_state_from_slot(slots, 0, CONTRACT_MAX_RETRY_ATTEMPTS)"),
            "RetryCheck must decode policy from slot 0"
        );

        // RetryCheck must compare retry count to CONTRACT_MAX_RETRY_ATTEMPTS
        assert!(
            source.contains("CONTRACT_MAX_RETRY_ATTEMPTS"),
            "RetryCheck must reference CONTRACT_MAX_RETRY_ATTEMPTS"
        );

        // RetryCheck must have branch targets for retry body and exhausted path
        assert!(
            source.contains("retry_check_target(_retry_state.current_attempt(), CONTRACT_MAX_RETRY_ATTEMPTS, 1, 4)"),
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

    // =========================================================================
    // POST-005..POST-010 executable generated-mode regression tests
    // =========================================================================

    fn post_build_object_copy_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("post_build_object_copy"),
            digest: WorkflowDigest::from_bytes([0xA5; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(2)),
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
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(3)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
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
            symbols_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn post_build_list_copy_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("post_build_list_copy"),
            digest: WorkflowDigest::from_bytes([0xA6; 32]),
            nodes: vec![
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
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(3)),
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(2),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
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

    fn post_eval_add_workflow() -> Result<CompiledWorkflow, String> {
        let expr = ExprProgram::try_from_ops(
            vec![
                vb_core::ExprOp::LoadSlot(SlotIdx::new(0)),
                vb_core::ExprOp::LoadSlot(SlotIdx::new(1)),
                vb_core::ExprOp::Add,
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("post_eval_add"),
            digest: WorkflowDigest::from_bytes([0xA7; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(2)),
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
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
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

    fn post_deep_accessor_workflow() -> Result<CompiledWorkflow, String> {
        let expr = ExprProgram::try_from_ops(
            vec![vb_core::ExprOp::LoadAccessor(vb_core::AccessorIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("post_deep_accessor"),
            digest: WorkflowDigest::from_bytes([0xAB; 32]),
            nodes: vec![
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
            expressions: vec![expr].into_boxed_slice(),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path: vec![PathSegment::Field(vb_core::SymbolId::new(0)); 17].into_boxed_slice(),
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn post_ask_resume_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("post_ask_resume"),
            digest: WorkflowDigest::from_bytes([0xA8; 32]),
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
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::AskResume {
                        answer: SlotIdx::new(2),
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

    fn post_retry_check_attempt_workflow(attempt: u16) -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("post_retry_check_attempt"),
            digest: WorkflowDigest::from_bytes([0xA9; 32]),
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
                    kind: CompiledNodeKind::RetryCheck {
                        policy_slot: SlotIdx::new(0),
                        body: StepIdx::new(2),
                        exhausted: StepIdx::new(3),
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
                ConstValue::I64(retry_state_raw(attempt, 0)?),
                ConstValue::I64(99),
                ConstValue::I64(-1),
            ]
            .into_boxed_slice(),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract {
                max_retry_attempts: 3,
                max_step_budget_per_tick: 100,
                ..ResourceContract::DEFAULT
            },
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn post_budget_exhaustion_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("post_budget_exhaustion"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
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
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
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
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract {
                max_step_budget_per_tick: 3,
                ..ResourceContract::DEFAULT
            },
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn post_005_build_object_field_slot_roundtrip_preserves_value() -> Result<(), String> {
        let workflow = post_build_object_copy_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_object_roundtrip",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(17));\n    slots[1] = Some(SlotValue::Bool(true));\n    taints[0] = Taint::Secret;\n    taints[1] = Taint::Clean;\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(StepOutcome::Continue(1)) => {}\n        Ok(StepOutcome::Continue(next)) => { println!(\"unexpected_step0_continue:{next}\"); return; }\n        Ok(StepOutcome::Finished(value)) => { println!(\"unexpected_step0_finished:{value:?}\"); return; }\n        Err(error) => { println!(\"unexpected_step0_err:{error:?}\"); return; }\n    }\n    match step_1(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(StepOutcome::Continue(2)) => println!(\"object={:?};copy={:?};taints={:?}:{:?}\", slots[2], slots[3], taints[2], taints[3]),\n        Ok(StepOutcome::Continue(next)) => println!(\"unexpected_step1_continue:{next}\"),\n        Ok(StepOutcome::Finished(value)) => println!(\"unexpected_step1_finished:{value:?}\"),\n        Err(error) => println!(\"unexpected_step1_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "object=Some(Object(0));copy=Some(Object(0));taints=Secret:Secret\n"
        );
        Ok(())
    }

    #[test]
    fn post_005_batch_list_slot_copy_preserves_list_handle() -> Result<(), String> {
        let workflow = post_build_list_copy_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_list_copy",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(7));\n    slots[1] = Some(SlotValue::I64(8));\n    taints[1] = Taint::DerivedFromSecret;\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(StepOutcome::Continue(1)) => {}\n        Ok(StepOutcome::Continue(next)) => { println!(\"unexpected_step0_continue:{next}\"); return; }\n        Ok(StepOutcome::Finished(value)) => { println!(\"unexpected_step0_finished:{value:?}\"); return; }\n        Err(error) => { println!(\"unexpected_step0_err:{error:?}\"); return; }\n    }\n    match step_1(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(StepOutcome::Continue(2)) => println!(\"list={:?};copy={:?};taints={:?}:{:?}\", slots[2], slots[3], taints[2], taints[3]),\n        Ok(StepOutcome::Continue(next)) => println!(\"unexpected_step1_continue:{next}\"),\n        Ok(StepOutcome::Finished(value)) => println!(\"unexpected_step1_finished:{value:?}\"),\n        Err(error) => println!(\"unexpected_step1_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "list=Some(List(0));copy=Some(List(0));taints=DerivedFromSecret:DerivedFromSecret\n"
        );
        Ok(())
    }

    #[test]
    fn post_006_eval_expr_binary_op_preserves_operand_taints() -> Result<(), String> {
        let workflow = post_eval_add_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_eval_add_taint",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(40));\n    slots[1] = Some(SlotValue::I64(2));\n    taints[0] = Taint::Secret;\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Ok(StepOutcome::Continue(1)) => println!(\"slot={:?};taint={:?}\", slots[2], taints[2]),\n        Ok(StepOutcome::Continue(next)) => println!(\"unexpected_continue:{next}\"),\n        Ok(StepOutcome::Finished(value)) => println!(\"unexpected_finished:{value:?}\"),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "slot=Some(I64(42));taint=Secret\n");
        Ok(())
    }

    #[test]
    fn post_006_action_result_taint_attaches_to_output_slot() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_action_completion_taint",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(99));\n    let mut state = GeneratedRunState::new(slots);\n    match state.run_until_blocked() {\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"suspended:{:?}:events={:?}\", suspended.suspension, suspended.journal.len()),\n        Ok(GeneratedRunStatus::Finished(output)) => { println!(\"unexpected_finished:{:?}:{:?}\", output.value, output.taint); return; }\n        Err(error) => { println!(\"unexpected_run_err:{error:?}\"); return; }\n    }\n    match state.complete_action(0, 10, 1, SlotValue::I64(123), Taint::DerivedFromSecret) {\n        Ok(GeneratedRunStatus::Finished(output)) => {\n            println!(\"finished:{:?}:{:?}:events={:?}\", output.value, output.taint, output.journal.len());\n            println!(\"event0={:?}\", output.journal.event(0));\n            println!(\"event1={:?}\", output.journal.event(1));\n            println!(\"event2={:?}\", output.journal.event(2));\n            println!(\"event3={:?}\", output.journal.event(3));\n        }\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"unexpected_suspended:{:?}\", suspended.suspension),\n        Err(error) => println!(\"unexpected_complete_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "suspended:ActionPending { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 }:events=1\nfinished:I64(123):DerivedFromSecret:events=4\nevent0=Some(ActionScheduled { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 })\nevent1=Some(SlotWritten { slot: 1, value: Some(I64(123)), taint: DerivedFromSecret })\nevent2=Some(ActionCompleted { step: 0, action_id: 10, output_slot: 1, value: I64(123), taint: DerivedFromSecret })\nevent3=Some(RunFinished { step: 1, value: I64(123), taint: DerivedFromSecret })\n"
        );
        Ok(())
    }

    #[test]
    fn post_007_slot_out_of_bounds_preserves_slot_index() -> Result<(), String> {
        let workflow = post_eval_add_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_slot_oob",
            "    let slots = [None; WORKFLOW_SLOT_COUNT];\n    match read_slot(&slots, 99) {\n        Err(DriveError::SlotOutOfBounds { slot: 99 }) => println!(\"err:SlotOutOfBounds:99\"),\n        other => println!(\"unexpected:{other:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "err:SlotOutOfBounds:99\n");
        Ok(())
    }

    #[test]
    fn post_007_missing_output_slot_preserves_step_index() -> Result<(), String> {
        let workflow = make_step_workflow(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 4,
                        body: StepIdx::new(1),
                        done: StepIdx::new(1),
                    },
                },
                finish_node(1),
            ],
            2,
        );
        let stdout = generated_step_stdout(
            &workflow,
            "post_missing_output",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    let mut list_store = ListStore::new();\n    let mut object_store = ObjectStore::new();\n    match step_0(&mut slots, &mut taints, &mut list_store, &mut object_store) {\n        Err(DriveError::MissingOutputSlot { step: 0 }) => println!(\"err:MissingOutputSlot:0\"),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n        Ok(StepOutcome::Continue(next)) => println!(\"unexpected_continue:{next}\"),\n        Ok(StepOutcome::Finished(value)) => println!(\"unexpected_finished:{value:?}\"),\n    }",
        )?;
        assert_eq!(stdout, "err:MissingOutputSlot:0\n");
        Ok(())
    }

    #[test]
    fn post_007_accessor_path_too_deep_preserves_depth_and_max() -> Result<(), String> {
        let error = post_deep_accessor_workflow()
            .err()
            .ok_or("deep accessor workflow unexpectedly validated")?;
        assert_eq!(error, "accessor path depth 17 exceeds maximum 16");
        Ok(())
    }

    #[test]
    fn post_007_expr_out_of_bounds_preserves_expr_index() -> Result<(), String> {
        let workflow = post_eval_add_workflow()?;
        let mut out = String::new();
        emit_expr_function(&mut out, vb_core::ExprIdx::new(7), &workflow)
            .map_err(|e| e.to_string())?;
        assert!(
            out.contains("Err(DriveError::ExprOutOfBounds { expr: 7 })"),
            "missing expression code must preserve the exact expression index, got: {out}"
        );
        Ok(())
    }

    #[test]
    fn post_007_step_budget_exhausted_error_preserved() -> Result<(), String> {
        let workflow = post_budget_exhaustion_workflow()?;
        let stdout = generated_drive_stdout(&workflow, "post_budget_exhausted", "")?;
        assert_eq!(stdout, "err:StepBudgetExhausted\n");
        Ok(())
    }

    #[test]
    fn post_007_taint_violation_error_preserved() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_tainted_do_rejected",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(1));\n    taints[0] = Taint::Secret;\n    let mut state = GeneratedRunState::new_with_taints(slots, taints);\n    match state.run_until_blocked() {\n        Err(DriveError::TaintViolation { step: 0 }) => println!(\"err:TaintViolation:0\"),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"unexpected_suspended:{:?}\", suspended.suspension),\n        Ok(GeneratedRunStatus::Finished(output)) => println!(\"unexpected_finished:{:?}:{:?}\", output.value, output.taint),\n    }",
        )?;
        assert_eq!(stdout, "err:TaintViolation:0\n");
        Ok(())
    }

    #[test]
    fn post_008_ask_resume_populates_answer_slot_from_ticket() -> Result<(), String> {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_ask_answer_api",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(11));\n    slots[1] = Some(SlotValue::I64(30));\n    let mut state = GeneratedRunState::new(slots);\n    match state.run_until_blocked() {\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"suspended:{:?}:events={:?}\", suspended.suspension, suspended.journal.len()),\n        Ok(GeneratedRunStatus::Finished(output)) => { println!(\"unexpected_finished:{:?}:{:?}\", output.value, output.taint); return; }\n        Err(error) => { println!(\"unexpected_run_err:{error:?}\"); return; }\n    }\n    match state.answer_ask(0, 1, SlotValue::I64(55), Taint::DerivedFromSecret) {\n        Ok(GeneratedRunStatus::Finished(output)) => {\n            println!(\"finished:{:?}:{:?}:events={:?}\", output.value, output.taint, output.journal.len());\n            println!(\"event0={:?}\", output.journal.event(0));\n            println!(\"event1={:?}\", output.journal.event(1));\n            println!(\"event2={:?}\", output.journal.event(2));\n        }\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"unexpected_suspended:{:?}\", suspended.suspension),\n        Err(error) => println!(\"unexpected_answer_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "suspended:AskPending { step: 0, prompt_slot: 0, timeout_slot: Some(1), resume_pc: 1 }:events=0\nfinished:I64(55):DerivedFromSecret:events=3\nevent0=Some(SlotWritten { slot: 2, value: Some(I64(55)), taint: DerivedFromSecret })\nevent1=Some(AskAnswered { ask_step: 0, resume_step: 1, answer_slot: 2, value: I64(55), taint: DerivedFromSecret })\nevent2=Some(RunFinished { step: 2, value: I64(55), taint: DerivedFromSecret })\n"
        );
        Ok(())
    }

    #[test]
    fn post_008_ask_ticket_preserves_prompt_and_timeout() -> Result<(), String> {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_drive_stdout(
            &workflow,
            "post_ask_ticket",
            "    slots[0] = Some(SlotValue::I64(11));\n    slots[1] = Some(SlotValue::I64(30));",
        )?;
        assert_eq!(
            stdout,
            "err:AskSuspend { step: 0, prompt_slot: 0, timeout_slot: Some(1), resume_pc: 1 }\n"
        );
        Ok(())
    }

    #[test]
    fn post_009_retry_check_routes_to_exhausted_when_attempt_eq_max() -> Result<(), String> {
        let workflow = post_retry_check_attempt_workflow(3)?;
        let generated_stdout = generated_trace_stdout(&workflow, "post_retry_eq_max", "")?;
        assert_eq!(
            generated_stdout,
            "result:I64(-1)\nfinal_pc:4\nslots:[Some(I64(196608)), Some(I64(-1))]\njournal:start:0|continue:1|start:1|continue:3|start:3|continue:4|start:4|finished\nretry_attempt_total:3\n"
        );
        Ok(())
    }

    #[test]
    fn post_009_retry_check_routes_to_exhausted_when_attempt_gt_max() -> Result<(), String> {
        let workflow = post_retry_check_attempt_workflow(4)?;
        let stdout = generated_trace_stdout(&workflow, "post_retry_gt_max", "")?;
        assert_eq!(
            stdout,
            "error:InvalidRetryState\nfinal_pc:1\nslots:[Some(I64(262144)), None]\njournal:start:0|continue:1|start:1|error\nretry_attempt_total:0\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_slot_written_journal_event_emitted() -> Result<(), String> {
        let workflow = post_eval_add_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_slot_written_journal_event",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(5));\n    slots[1] = Some(SlotValue::I64(6));\n    match drive_with_journal(slots) {\n        Ok(GeneratedRunStatus::Finished(output)) => {\n            println!(\"finished:{:?}:{:?}:events={:?}\", output.value, output.taint, output.journal.len());\n            println!(\"event0={:?}\", output.journal.event(0));\n            println!(\"event1={:?}\", output.journal.event(1));\n            println!(\"event2={:?}\", output.journal.event(2));\n        }\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"unexpected_suspended:{:?}\", suspended.suspension),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "finished:I64(11):Clean:events=2\nevent0=Some(SlotWritten { slot: 2, value: Some(I64(11)), taint: Clean })\nevent1=Some(RunFinished { step: 1, value: I64(11), taint: Clean })\nevent2=None\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_action_scheduled_journal_event_emitted() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_action_scheduled_journal_event",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(99));\n    match drive_with_journal(slots) {\n        Ok(GeneratedRunStatus::Suspended(suspended)) => {\n            println!(\"suspended:{:?}:events={:?}\", suspended.suspension, suspended.journal.len());\n            println!(\"event0={:?}\", suspended.journal.event(0));\n            println!(\"event1={:?}\", suspended.journal.event(1));\n        }\n        Ok(GeneratedRunStatus::Finished(output)) => println!(\"unexpected_finished:{:?}:{:?}\", output.value, output.taint),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "suspended:ActionPending { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 }:events=1\nevent0=Some(ActionScheduled { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 })\nevent1=None\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_action_completed_journal_event_emitted() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_action_completed_journal_event",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(99));\n    let mut state = GeneratedRunState::new(slots);\n    match state.run_until_blocked() {\n        Ok(GeneratedRunStatus::Suspended(_)) => {}\n        Ok(GeneratedRunStatus::Finished(output)) => { println!(\"unexpected_finished:{:?}:{:?}\", output.value, output.taint); return; }\n        Err(error) => { println!(\"unexpected_run_err:{error:?}\"); return; }\n    }\n    match state.complete_action(0, 10, 1, SlotValue::Bool(true), Taint::Secret) {\n        Ok(GeneratedRunStatus::Finished(output)) => {\n            println!(\"finished:{:?}:{:?}:events={:?}\", output.value, output.taint, output.journal.len());\n            println!(\"event1={:?}\", output.journal.event(1));\n            println!(\"event2={:?}\", output.journal.event(2));\n            println!(\"event3={:?}\", output.journal.event(3));\n        }\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"unexpected_suspended:{:?}\", suspended.suspension),\n        Err(error) => println!(\"unexpected_complete_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "finished:Bool(true):Secret:events=4\nevent1=Some(SlotWritten { slot: 1, value: Some(Bool(true)), taint: Secret })\nevent2=Some(ActionCompleted { step: 0, action_id: 10, output_slot: 1, value: Bool(true), taint: Secret })\nevent3=Some(RunFinished { step: 1, value: Bool(true), taint: Secret })\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_run_finished_journal_event_emitted() -> Result<(), String> {
        let workflow = post_eval_add_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_run_finished_journal_event",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    let mut taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(20));\n    slots[1] = Some(SlotValue::I64(22));\n    taints[0] = Taint::DerivedFromSecret;\n    let mut state = GeneratedRunState::new_with_taints(slots, taints);\n    match state.run_until_blocked() {\n        Ok(GeneratedRunStatus::Finished(output)) => {\n            println!(\"finished:{:?}:{:?}:events={:?}\", output.value, output.taint, output.journal.len());\n            println!(\"event0={:?}\", output.journal.event(0));\n            println!(\"event1={:?}\", output.journal.event(1));\n        }\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"unexpected_suspended:{:?}\", suspended.suspension),\n        Err(error) => println!(\"unexpected_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "finished:I64(42):DerivedFromSecret:events=2\nevent0=Some(SlotWritten { slot: 2, value: Some(I64(42)), taint: DerivedFromSecret })\nevent1=Some(RunFinished { step: 1, value: I64(42), taint: DerivedFromSecret })\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_ask_answered_journal_event_emitted() -> Result<(), String> {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_ask_answered_journal_event",
            "    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    slots[0] = Some(SlotValue::I64(11));\n    slots[1] = Some(SlotValue::I64(30));\n    let mut state = GeneratedRunState::new(slots);\n    match state.run_until_blocked() {\n        Ok(GeneratedRunStatus::Suspended(_)) => {}\n        Ok(GeneratedRunStatus::Finished(output)) => { println!(\"unexpected_finished:{:?}:{:?}\", output.value, output.taint); return; }\n        Err(error) => { println!(\"unexpected_run_err:{error:?}\"); return; }\n    }\n    match state.answer_ask(0, 1, SlotValue::Bool(false), Taint::Secret) {\n        Ok(GeneratedRunStatus::Finished(output)) => {\n            println!(\"finished:{:?}:{:?}:events={:?}\", output.value, output.taint, output.journal.len());\n            println!(\"event0={:?}\", output.journal.event(0));\n            println!(\"event1={:?}\", output.journal.event(1));\n            println!(\"event2={:?}\", output.journal.event(2));\n        }\n        Ok(GeneratedRunStatus::Suspended(suspended)) => println!(\"unexpected_suspended:{:?}\", suspended.suspension),\n        Err(error) => println!(\"unexpected_answer_err:{error:?}\"),\n    }",
        )?;
        assert_eq!(
            stdout,
            "finished:Bool(false):Secret:events=3\nevent0=Some(SlotWritten { slot: 2, value: Some(Bool(false)), taint: Secret })\nevent1=Some(AskAnswered { ask_step: 0, resume_step: 1, answer_slot: 2, value: Bool(false), taint: Secret })\nevent2=Some(RunFinished { step: 2, value: Bool(false), taint: Secret })\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_invalid_action_resume_reports_step() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_invalid_action_resume",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(99));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("pending:{:?}:events={:?}", suspended.suspension, suspended.journal.len()),
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    let before_events = state.journal.len();
    let before_slot = state.slots[1];
    match state.complete_action(0, 11, 1, SlotValue::I64(1), Taint::Clean) {
        Err(DriveError::InvalidResume { step: 0 }) => println!("err:InvalidResume:0:events={:?}:slot={:?}", state.journal.len(), state.slots[1]),
        Err(error) => println!("unexpected_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }
    println!("unchanged={:?}:{:?}", state.journal.len() == before_events, state.slots[1] == before_slot);"#,
        )?;
        assert_eq!(
            stdout,
            "pending:ActionPending { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 }:events=1
err:InvalidResume:0:events=1:slot=None
unchanged=true:true
"
        );
        Ok(())
    }

    #[test]
    fn post_010_invalid_ask_resume_reports_ask_step() -> Result<(), String> {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_invalid_ask_resume",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(11));
    slots[1] = Some(SlotValue::I64(30));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("pending:{:?}:events={:?}", suspended.suspension, suspended.journal.len()),
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    let before_events = state.journal.len();
    let before_slot = state.slots[2];
    match state.answer_ask(9, 1, SlotValue::I64(1), Taint::Clean) {
        Err(DriveError::InvalidResume { step: 9 }) => println!("err:InvalidResume:9:events={:?}:slot={:?}", state.journal.len(), state.slots[2]),
        Err(error) => println!("unexpected_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }
    println!("unchanged={:?}:{:?}", state.journal.len() == before_events, state.slots[2] == before_slot);"#,
        )?;
        assert_eq!(
            stdout,
            "pending:AskPending { step: 0, prompt_slot: 0, timeout_slot: Some(1), resume_pc: 1 }:events=0
err:InvalidResume:9:events=0:slot=None
unchanged=true:true
"
        );
        Ok(())
    }

    #[test]
    fn post_010_fresh_action_resume_without_pending_state_does_not_mutate() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_fresh_action_resume",
            r#"    let slots = [None; WORKFLOW_SLOT_COUNT];
    let mut state = GeneratedRunState::new(slots);
    match state.complete_action(0, 10, 1, SlotValue::I64(1), Taint::Clean) {
        Err(DriveError::InvalidResume { step: 0 }) => println!("err:InvalidResume:0:events={:?}:slot={:?}", state.journal.len(), state.slots[1]),
        Err(error) => println!("unexpected_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }"#,
        )?;
        assert_eq!(stdout, "err:InvalidResume:0:events=0:slot=None\n");
        Ok(())
    }

    #[test]
    fn post_010_fresh_ask_resume_without_pending_state_does_not_mutate() -> Result<(), String> {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_fresh_ask_resume",
            r#"    let slots = [None; WORKFLOW_SLOT_COUNT];
    let mut state = GeneratedRunState::new(slots);
    match state.answer_ask(0, 1, SlotValue::I64(1), Taint::Clean) {
        Err(DriveError::InvalidResume { step: 0 }) => println!("err:InvalidResume:0:events={:?}:slot={:?}", state.journal.len(), state.slots[2]),
        Err(error) => println!("unexpected_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }"#,
        )?;
        assert_eq!(stdout, "err:InvalidResume:0:events=0:slot=None\n");
        Ok(())
    }

    #[test]
    fn post_010_wrong_action_output_slot_does_not_mutate_pending_action() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_wrong_action_output_slot",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(99));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("pending:{:?}:events={:?}", suspended.suspension, suspended.journal.len()),
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    let before_events = state.journal.len();
    let before_slot = state.slots[1];
    match state.complete_action(0, 10, 0, SlotValue::I64(1), Taint::Clean) {
        Err(DriveError::InvalidResume { step: 0 }) => println!("err:InvalidResume:0:events={:?}:slot={:?}", state.journal.len(), state.slots[1]),
        Err(error) => println!("unexpected_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }
    println!("unchanged={:?}:{:?}", state.journal.len() == before_events, state.slots[1] == before_slot);"#,
        )?;
        assert_eq!(
            stdout,
            "pending:ActionPending { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 }:events=1\nerr:InvalidResume:0:events=1:slot=None\nunchanged=true:true\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_wrong_ask_resume_step_does_not_mutate_pending_ask() -> Result<(), String> {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_wrong_ask_resume_step",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(11));
    slots[1] = Some(SlotValue::I64(30));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("pending:{:?}:events={:?}", suspended.suspension, suspended.journal.len()),
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    let before_events = state.journal.len();
    let before_slot = state.slots[2];
    match state.answer_ask(0, 2, SlotValue::I64(1), Taint::Clean) {
        Err(DriveError::InvalidResume { step: 0 }) => println!("err:InvalidResume:0:events={:?}:slot={:?}", state.journal.len(), state.slots[2]),
        Err(error) => println!("unexpected_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }
    println!("unchanged={:?}:{:?}", state.journal.len() == before_events, state.slots[2] == before_slot);"#,
        )?;
        assert_eq!(
            stdout,
            "pending:AskPending { step: 0, prompt_slot: 0, timeout_slot: Some(1), resume_pc: 1 }:events=0\nerr:InvalidResume:0:events=0:slot=None\nunchanged=true:true\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_duplicate_action_completion_returns_invalid_resume_without_mutation()
    -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_duplicate_action_completion",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(99));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(_)) => {}
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    match state.complete_action(0, 10, 1, SlotValue::I64(1), Taint::Clean) {
        Ok(GeneratedRunStatus::Finished(output)) => println!("first:{:?}:events={:?}:slot={:?}", output.value, output.journal.len(), state.slots[1]),
        Ok(GeneratedRunStatus::Suspended(suspended)) => { println!("unexpected_first_suspended:{:?}", suspended.suspension); return; }
        Err(error) => { println!("unexpected_first_err:{error:?}"); return; }
    }
    match state.complete_action(0, 10, 1, SlotValue::I64(2), Taint::Secret) {
        Err(DriveError::InvalidResume { step: 0 }) => println!("duplicate_err:InvalidResume:0:events={:?}:slot={:?}", state.journal.len(), state.slots[1]),
        Err(error) => println!("unexpected_duplicate_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_duplicate_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_duplicate_finished:{:?}:{:?}", output.value, output.taint),
    }
    let mut action_completed_count = 0u16;
    let mut index = 0u16;
    while index < state.journal.len() {
        match state.journal.event(index) {
            Some(JournalEvent::ActionCompleted { .. }) => action_completed_count = action_completed_count.saturating_add(1),
            _ => {}
        }
        index = index.saturating_add(1);
    }
    println!("action_completed_count={action_completed_count}");"#,
        )?;
        assert_eq!(
            stdout,
            "first:I64(1):events=4:slot=Some(I64(1))
duplicate_err:InvalidResume:0:events=4:slot=Some(I64(1))
action_completed_count=1
"
        );
        Ok(())
    }

    #[test]
    fn post_010_duplicate_ask_answer_returns_invalid_resume_without_mutation() -> Result<(), String>
    {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_duplicate_ask_answer",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(11));
    slots[1] = Some(SlotValue::I64(30));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(_)) => {}
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    match state.answer_ask(0, 1, SlotValue::I64(55), Taint::DerivedFromSecret) {
        Ok(GeneratedRunStatus::Finished(output)) => println!("first:{:?}:{:?}:events={:?}:slot={:?}", output.value, output.taint, output.journal.len(), state.slots[2]),
        Ok(GeneratedRunStatus::Suspended(suspended)) => { println!("unexpected_first_suspended:{:?}", suspended.suspension); return; }
        Err(error) => { println!("unexpected_first_err:{error:?}"); return; }
    }
    match state.answer_ask(0, 1, SlotValue::I64(99), Taint::Secret) {
        Err(DriveError::InvalidResume { step: 0 }) => println!("duplicate_err:InvalidResume:0:events={:?}:slot={:?}", state.journal.len(), state.slots[2]),
        Err(error) => println!("unexpected_duplicate_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_duplicate_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_duplicate_finished:{:?}:{:?}", output.value, output.taint),
    }
    let mut ask_answered_count = 0u16;
    let mut index = 0u16;
    while index < state.journal.len() {
        match state.journal.event(index) {
            Some(JournalEvent::AskAnswered { .. }) => ask_answered_count = ask_answered_count.saturating_add(1),
            _ => {}
        }
        index = index.saturating_add(1);
    }
    println!("ask_answered_count={ask_answered_count}");"#,
        )?;
        assert_eq!(
            stdout,
            "first:I64(55):DerivedFromSecret:events=3:slot=Some(I64(55))\nduplicate_err:InvalidResume:0:events=3:slot=Some(I64(55))\nask_answered_count=1\n"
        );
        Ok(())
    }

    #[test]
    fn post_010_journal_overflow_reports_typed_error() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_journal_overflow",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(99));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("start_events={:?}", suspended.journal.len()),
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    let capacity = match u16::try_from(GENERATED_JOURNAL_CAPACITY) {
        Ok(value) => value,
        Err(_) => { println!("capacity_conversion_failed"); return; }
    };
    while state.journal.len() < capacity {
        match state.journal.push(JournalEvent::RunFinished { step: 99, value: SlotValue::Null, taint: Taint::Clean }) {
            Ok(()) => {}
            Err(error) => { println!("unexpected_fill_err:{error:?}"); return; }
        }
    }
    println!("filled_events={:?}:slot={:?}", state.journal.len(), state.slots[1]);
    match state.complete_action(0, 10, 1, SlotValue::I64(4), Taint::Secret) {
        Err(DriveError::JournalOverflow) => println!("err:JournalOverflow:events={:?}:slot={:?}:last={:?}", state.journal.len(), state.slots[1], state.journal.event(capacity.saturating_sub(1))),
        Err(error) => println!("unexpected_complete_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }"#,
        )?;
        assert_eq!(
            stdout,
            "start_events=1
filled_events=12:slot=None
err:JournalOverflow:events=12:slot=None:last=Some(RunFinished { step: 99, value: Null, taint: Clean })
"
        );
        Ok(())
    }

    #[test]
    fn post_010_ask_answer_journal_overflow_reports_typed_error_before_mutation()
    -> Result<(), String> {
        let workflow = post_ask_resume_workflow()?;
        let stdout = generated_step_stdout(
            &workflow,
            "post_ask_journal_overflow",
            r#"    let mut slots = [None; WORKFLOW_SLOT_COUNT];
    slots[0] = Some(SlotValue::I64(11));
    slots[1] = Some(SlotValue::I64(30));
    let mut state = GeneratedRunState::new(slots);
    match state.run_until_blocked() {
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("start_events={:?}:pending={:?}", suspended.journal.len(), suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => { println!("unexpected_initial_finished:{:?}:{:?}", output.value, output.taint); return; }
        Err(error) => { println!("unexpected_initial_err:{error:?}"); return; }
    }
    let capacity = match u16::try_from(GENERATED_JOURNAL_CAPACITY) {
        Ok(value) => value,
        Err(_) => { println!("capacity_conversion_failed"); return; }
    };
    while state.journal.len() < capacity {
        match state.journal.push(JournalEvent::RunFinished { step: 99, value: SlotValue::Null, taint: Taint::Clean }) {
            Ok(()) => {}
            Err(error) => { println!("unexpected_fill_err:{error:?}"); return; }
        }
    }
    println!("filled_events={:?}:slot={:?}", state.journal.len(), state.slots[2]);
    match state.answer_ask(0, 1, SlotValue::I64(55), Taint::Secret) {
        Err(DriveError::JournalOverflow) => println!("err:JournalOverflow:events={:?}:slot={:?}:last={:?}", state.journal.len(), state.slots[2], state.journal.event(capacity.saturating_sub(1))),
        Err(error) => println!("unexpected_answer_err:{error:?}"),
        Ok(GeneratedRunStatus::Suspended(suspended)) => println!("unexpected_suspended:{:?}", suspended.suspension),
        Ok(GeneratedRunStatus::Finished(output)) => println!("unexpected_finished:{:?}:{:?}", output.value, output.taint),
    }"#,
        )?;
        assert_eq!(
            stdout,
            "start_events=0:pending=AskPending { step: 0, prompt_slot: 0, timeout_slot: Some(1), resume_pc: 1 }\nfilled_events=18:slot=None\nerr:JournalOverflow:events=18:slot=None:last=Some(RunFinished { step: 99, value: Null, taint: Clean })\n"
        );
        Ok(())
    }

    #[test]
    fn post_011_generated_finished_value_taint_and_journal_match_ir_for_expression()
    -> Result<(), String> {
        let workflow = post_eval_add_workflow()?;
        let init = [
            (SlotIdx::new(0), SlotValue::I64(40), Taint::Secret),
            (SlotIdx::new(1), SlotValue::I64(2), Taint::Clean),
        ];
        let (ir_value, ir_taint) = ir_drive_finished_output_with_init(&workflow, &init)?;
        let stdout = generated_state_run_stdout(
            &workflow,
            "post_parity_expr_finished_output",
            r#"    slots[0] = Some(SlotValue::I64(40));
    slots[1] = Some(SlotValue::I64(2));
    slot_taints[0] = Taint::Secret;
    slot_taints[1] = Taint::Clean;
"#,
        )?;
        assert_eq!(
            stdout,
            format!(
                "finished:{ir_value:?}:{ir_taint:?}:events=2\nevent:0:SlotWritten {{ slot: 2, value: Some({ir_value:?}), taint: {ir_taint:?} }}\nevent:1:RunFinished {{ step: 1, value: {ir_value:?}, taint: {ir_taint:?} }}\n"
            )
        );
        Ok(())
    }

    #[test]
    fn post_011_generated_finished_value_taint_and_journal_match_ir_for_constant_expression()
    -> Result<(), String> {
        let workflow = primitive_expression_workflow()?;
        let (ir_value, ir_taint) = ir_drive_finished_output_with_init(&workflow, &[])?;
        let stdout = generated_state_run_stdout(
            &workflow,
            "post_parity_primitive_expression_finished_output",
            "",
        )?;
        assert_eq!(
            stdout,
            format!(
                "finished:{ir_value:?}:{ir_taint:?}:events=2\nevent:0:SlotWritten {{ slot: 0, value: Some({ir_value:?}), taint: {ir_taint:?} }}\nevent:1:RunFinished {{ step: 1, value: {ir_value:?}, taint: {ir_taint:?} }}\n"
            )
        );
        Ok(())
    }

    #[test]
    fn post_011_generated_suspension_matches_ir_for_action_boundary() -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let ir_signal = ir_action_suspend_signal(&workflow, SlotIdx::new(0))?;
        assert_eq!(ir_signal, EngineSignal::AwaitingAction);
        let stdout = generated_state_run_stdout(
            &workflow,
            "post_parity_action_suspension",
            r#"    slots[0] = Some(SlotValue::I64(99));
"#,
        )?;
        assert_eq!(
            stdout,
            "suspended:ActionPending { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 }:events=1\nevent:0:ActionScheduled { step: 0, action_id: 10, input_slot: 0, resume_pc: 1 }\n"
        );
        Ok(())
    }

    #[test]
    fn post_011_generated_no_contract_do_taint_violation_matches_runtime_error()
    -> Result<(), String> {
        let workflow = action_suspend_workflow(ActionId::new(10), SlotIdx::new(0))?;
        let runtime_error = runtime_drive_error_string_with_init(
            &workflow,
            &[(SlotIdx::new(0), SlotValue::I64(99), Taint::Secret)],
        )?;
        assert!(
            runtime_error.contains("taint violation") || runtime_error.contains("TaintViolation")
        );
        let stdout = generated_state_run_stdout(
            &workflow,
            "post_parity_no_contract_do_taint_violation",
            r#"    slots[0] = Some(SlotValue::I64(99));
    slot_taints[0] = Taint::Secret;
"#,
        )?;
        assert_eq!(stdout, "err:TaintViolation { step: 0 }\n");
        Ok(())
    }
}
