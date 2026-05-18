#![forbid(unsafe_code)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::panic_in_result_fn)]

use vb_compile::{
    CompileError, CompileErrors, SlotCompiler, SourceMark, WaitKind, YamlCompiler, YamlLimits,
    build_accessor_table, build_constant_pool, build_slot_layout, check_idempotency_gates,
    compile_source, compile_to_generated_rust, compile_workflow, compute_compiled_digest,
    emit_compiled_artifact, is_compile_idempotency_gate_accepted, lower_ask, lower_choose,
    lower_collect, lower_do, lower_finish, lower_for_each, lower_reduce, lower_repeat, lower_set,
    lower_steps_to_ir, lower_together, lower_wait, validate_ir,
};
use vb_core::{
    ActionContract, ActionId, CompiledNode, CompiledNodeKind, ConstIdx, ConstValue, Idempotency,
    ResourceContract, RetrySafety, SideEffect, SlotBranch, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowParts,
};

const SOURCE_LINE_LIMIT: usize = 300;
const VB_COMPILE_LIB_RS: &str = include_str!("../../vb_compile/src/lib.rs");
const VB_COMPILE_CORE_RS: &str = include_str!("../../vb_compile/src/mod_compile_core.rs");
const VB_COMPILE_ERRORS_RS: &str = include_str!("../../vb_compile/src/mod_compile_errors.rs");
const VB_COMPILE_VALIDATION_RS: &str =
    include_str!("../../vb_compile/src/mod_compile_validation.rs");
const VB_COMPILE_LOWERING_RS: &str = include_str!("../../vb_compile/src/mod_compile_lowering.rs");
const MINIMAL_WORKFLOW: &[u8] = br#"version: velvet-ballastics/v1
name: minimal_finish
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
const EXPECTED_MINIMAL_ARTIFACT_BYTES: &[u8] = &[
    14, 109, 105, 110, 105, 109, 97, 108, 95, 102, 105, 110, 105, 115, 104, 238, 210, 249, 161,
    106, 116, 203, 148, 212, 114, 40, 2, 8, 180, 166, 205, 9, 41, 144, 183, 223, 95, 103, 39, 180,
    12, 249, 123, 26, 152, 113, 128, 1, 0, 0, 0, 0, 0, 33, 0, 0, 0, 0, 1, 0, 0, 144, 78, 128, 8,
    255, 255, 3, 128, 64, 128, 32, 64, 144, 78, 144, 78, 128, 128, 64, 128, 128, 16, 128, 128, 128,
    8, 128, 128, 64, 3, 64, 128, 8, 128, 8, 128, 128, 64, 0, 1, 4, 100, 111, 110, 101,
];
const EXPECTED_MINIMAL_ARTIFACT_DIGEST: WorkflowDigest = WorkflowDigest::from_bytes([
    220, 25, 198, 234, 250, 40, 166, 180, 136, 254, 213, 18, 240, 132, 236, 127, 218, 196, 88, 53,
    177, 22, 161, 97, 69, 138, 131, 28, 50, 42, 237, 174,
]);
const EXPECTED_MINIMAL_GENERATED_DIGEST: WorkflowDigest = WorkflowDigest::from_bytes([
    63, 64, 128, 60, 49, 67, 227, 251, 100, 242, 87, 255, 194, 142, 170, 33, 138, 122, 104, 168,
    72, 30, 170, 234, 117, 111, 72, 178, 103, 206, 33, 147,
]);

#[test]
fn crate_root_api_compiles_when_vb_compile_is_split() -> Result<(), String> {
    let limits = YamlLimits::default();
    let compiler = YamlCompiler::new(limits);
    let workflow = compiler
        .compile(MINIMAL_WORKFLOW)
        .map_err(|e| e.to_string())?;
    let parts = workflow.to_parts();
    let digest = WorkflowDigest::from_bytes([7; 32]);
    let mut builder = SlotCompiler::new();

    let set_node = lower_set(StepIdx::new(0), SlotIdx::new(0), ConstIdx::new(0), None);
    let do_node = lower_do(
        StepIdx::new(1),
        ActionId::new(1),
        SlotIdx::new(0),
        Some(SlotIdx::new(1)),
        None,
        &mut builder,
    );
    let choose_node = lower_choose(
        StepIdx::new(2),
        vec![SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(3),
        }],
        Some(StepIdx::new(4)),
        &mut builder,
    )
    .map_err(|e| e.to_string())?;
    let for_each_nodes = lower_for_each(
        StepIdx::new(3),
        SlotIdx::new(0),
        SlotIdx::new(1),
        1,
        StepIdx::new(4),
        StepIdx::new(5),
        &mut builder,
    )
    .map_err(|e| e.to_string())?;
    let together_nodes = lower_together(
        StepIdx::new(6),
        vec![StepIdx::new(7)],
        StepIdx::new(8),
        &mut builder,
    )
    .map_err(|e| e.to_string())?;
    let collect_nodes = lower_collect(
        StepIdx::new(9),
        SlotIdx::new(0),
        1,
        1,
        StepIdx::new(10),
        StepIdx::new(11),
        &mut builder,
    )
    .map_err(|e| e.to_string())?;
    let reduce_nodes = lower_reduce(
        StepIdx::new(12),
        SlotIdx::new(0),
        SlotIdx::new(1),
        ConstIdx::new(0),
        StepIdx::new(13),
        StepIdx::new(14),
        &mut builder,
    )
    .map_err(|e| e.to_string())?;
    let repeat_nodes = lower_repeat(
        StepIdx::new(15),
        2,
        StepIdx::new(16),
        StepIdx::new(17),
        &mut builder,
    )
    .map_err(|e| e.to_string())?;
    let wait_node = lower_wait(
        StepIdx::new(18),
        WaitKind::Until {
            deadline: SlotIdx::new(2),
        },
        &mut builder,
    );
    let ask_nodes = lower_ask(
        StepIdx::new(19),
        SlotIdx::new(2),
        SlotIdx::new(3),
        None,
        &mut builder,
    )
    .map_err(|e| e.to_string())?;
    let finish_node = lower_finish(StepIdx::new(21), SlotIdx::new(0), &mut builder);

    assert_eq!(limits, YamlLimits::default());
    assert_eq!(build_slot_layout(&parts), parts.slot_count);
    assert_eq!(build_accessor_table(&parts), parts.accessors.as_ref());
    assert_eq!(build_constant_pool(&parts), parts.constants.as_ref());
    assert_eq!(
        compute_compiled_digest(b"abc"),
        WorkflowDigest::from_bytes(blake3::hash(b"abc").into())
    );
    assert_eq!(
        set_node.kind,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }
    );
    assert_eq!(
        do_node.kind,
        CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0)
        }
    );
    assert_eq!(
        choose_node,
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(3),
                }]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(4)),
            },
        }
    );
    assert_eq!(
        for_each_nodes,
        vec![
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 1,
                    body: StepIdx::new(4),
                    done: StepIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(1),
                    body: StepIdx::new(4),
                    done: StepIdx::new(5),
                },
            },
        ]
    );
    assert_eq!(
        together_nodes,
        vec![
            CompiledNode {
                id: StepIdx::new(6),
                output: Some(SlotIdx::new(2)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(7)].into_boxed_slice(),
                    join: StepIdx::new(8),
                },
            },
            CompiledNode {
                id: StepIdx::new(8),
                output: Some(SlotIdx::new(2)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherJoin {
                    branch_count: 1,
                    accumulator: SlotIdx::new(2),
                },
            },
        ]
    );
    assert_eq!(
        collect_nodes,
        vec![
            CompiledNode {
                id: StepIdx::new(9),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 1,
                    page_size: 1,
                    body: StepIdx::new(10),
                    done: StepIdx::new(11),
                },
            },
            CompiledNode {
                id: StepIdx::new(10),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectPage {
                    collector_slot: SlotIdx::new(0),
                    body: StepIdx::new(10),
                    done: StepIdx::new(11),
                },
            },
            CompiledNode {
                id: StepIdx::new(11),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectFinish {
                    collector_slot: SlotIdx::new(0),
                },
            },
        ]
    );
    assert_eq!(
        reduce_nodes,
        vec![
            CompiledNode {
                id: StepIdx::new(12),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ReduceStart {
                    input: SlotIdx::new(0),
                    accumulator: SlotIdx::new(1),
                    initial: ConstIdx::new(0),
                    body: StepIdx::new(13),
                    done: StepIdx::new(14),
                },
            },
            CompiledNode {
                id: StepIdx::new(13),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ReduceNext {
                    iterator_slot: SlotIdx::new(1),
                    accumulator: SlotIdx::new(1),
                    body: StepIdx::new(13),
                    done: StepIdx::new(14),
                },
            },
            CompiledNode {
                id: StepIdx::new(14),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ReduceFinish {
                    accumulator: SlotIdx::new(1),
                },
            },
        ]
    );
    assert_eq!(
        repeat_nodes,
        vec![
            CompiledNode {
                id: StepIdx::new(15),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatStart {
                    max_attempts: 2,
                    body: StepIdx::new(16),
                    done: StepIdx::new(17),
                },
            },
            CompiledNode {
                id: StepIdx::new(16),
                output: Some(SlotIdx::new(16)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatAttempt {
                    attempt_slot: SlotIdx::new(16),
                    body: StepIdx::new(16),
                    done: StepIdx::new(17),
                },
            },
            CompiledNode {
                id: StepIdx::new(17),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatFinish {
                    result: SlotIdx::new(16),
                },
            },
        ]
    );
    assert_eq!(
        wait_node.kind,
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(2)
        }
    );
    assert_eq!(
        ask_nodes,
        vec![
            CompiledNode {
                id: StepIdx::new(19),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(2),
                    timeout_slot: None,
                },
            },
            CompiledNode {
                id: StepIdx::new(20),
                output: Some(SlotIdx::new(3)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(3),
                },
            },
        ]
    );
    assert_eq!(
        finish_node.kind,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }
    );

    let lowered = lower_steps_to_ir(
        vec![set_node],
        Vec::new(),
        Vec::new(),
        vec![ConstValue::I64(1)],
        1,
        0,
        "lowered",
        digest,
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(lowered.name(), "lowered");

    let validated = validate_ir(parts).map_err(|e| e.to_string())?;
    assert_eq!(validated.name(), "minimal_finish");
    assert_eq!(
        compile_source(
            &vb_yaml::parse_workflow_source(
                std::str::from_utf8(MINIMAL_WORKFLOW).map_err(|e| e.to_string())?
            )
            .map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?
        .name(),
        "minimal_finish"
    );
    let artifact = emit_compiled_artifact(&workflow).map_err(|e| e.to_string())?;
    assert_eq!(artifact.as_ref(), EXPECTED_MINIMAL_ARTIFACT_BYTES);
    assert_eq!(
        compute_compiled_digest(&artifact),
        EXPECTED_MINIMAL_ARTIFACT_DIGEST
    );
    assert_eq!(artifact.starts_with(b"\x0eminimal_finish"), true);
    assert_eq!(artifact.ends_with(b"\x04done"), true);
    let generated = compile_to_generated_rust(&workflow).map_err(|e| e.to_string())?;
    assert_eq!(
        compute_compiled_digest(generated.as_bytes()),
        EXPECTED_MINIMAL_GENERATED_DIGEST
    );
    assert_eq!(generated_shape_checks(&generated), Vec::<&str>::new());
    let _mark = SourceMark {
        index: 0,
        end_index: 0,
        line: 0,
        column: 0,
        available: false,
    };
    Ok(())
}

#[test]
fn compile_outputs_match_baseline_when_workflow_is_accepted() -> Result<(), String> {
    let workflow = compile_workflow(MINIMAL_WORKFLOW).map_err(|e| e.to_string())?;
    let parts = workflow.to_parts();
    let node = parts
        .nodes
        .first()
        .ok_or_else(|| "missing node 0".to_string())?;
    let artifact = emit_compiled_artifact(&workflow).map_err(|e| e.to_string())?;
    let artifact_digest = compute_compiled_digest(&artifact);
    let generated = compile_to_generated_rust(&workflow).map_err(|e| e.to_string())?;

    assert_eq!(workflow.name(), "minimal_finish");
    assert_eq!(workflow.node_count(), 1);
    assert_eq!(workflow.slot_count(), 1);
    assert_eq!(workflow.symbols_count(), 0);
    assert_eq!(workflow.entry(), StepIdx::new(0));
    assert_eq!(workflow.step_name(StepIdx::new(0)), Some("done"));
    assert_eq!(node.id, StepIdx::new(0));
    assert_eq!(node.output, None);
    assert_eq!(node.next, None);
    assert_eq!(
        node.kind,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }
    );
    assert_eq!(parts.constants.as_ref(), &[]);
    assert_eq!(artifact.as_ref(), EXPECTED_MINIMAL_ARTIFACT_BYTES);
    assert_eq!(artifact_digest, EXPECTED_MINIMAL_ARTIFACT_DIGEST);
    assert_eq!(
        compute_compiled_digest(&artifact),
        EXPECTED_MINIMAL_ARTIFACT_DIGEST
    );
    assert_eq!(
        compute_compiled_digest(generated.as_bytes()),
        EXPECTED_MINIMAL_GENERATED_DIGEST
    );
    assert_eq!(generated_shape_checks(&generated), Vec::<&str>::new());
    Ok(())
}

#[test]
fn compile_errors_match_baseline_when_workflow_is_rejected() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: duplicate_outputs
when:
  manual: {}
steps:
  - id: first
    set:
      output: saved
      value: "1"
  - id: second
    set:
      output: saved
      value: "2"
  - id: done
    finish:
      result: saved
"#;

    let errors = match compile_workflow(source) {
        Ok(workflow) => {
            return Err(format!(
                "compile unexpectedly succeeded: {}",
                workflow.name()
            ));
        }
        Err(CompileErrors(errors)) => errors,
    };
    let first = errors
        .first()
        .ok_or_else(|| "missing first compile error".to_string())?;

    assert_eq!(errors.len(), 1);
    assert_eq!(first.diagnostic_code(), "DUPLICATE_ID");
    assert_eq!(first.to_string(), "duplicate set output name: saved");
    assert_eq!(
        matches!(first, CompileError::DuplicateOutputName { name } if name.as_ref() == "saved"),
        true
    );
    Ok(())
}

#[test]
fn idempotency_gate_matches_validation_contract_for_bounded_cases() -> Result<(), String> {
    let side_effects = [
        SideEffect::None,
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ];
    let retry_safety = [
        RetrySafety::Safe,
        RetrySafety::KeyRequired,
        RetrySafety::Unsafe,
    ];
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];
    let mut checked = 0usize;

    for side_effect in side_effects {
        for retry in retry_safety {
            for idempotency in idempotencies {
                let contract = action_contract(side_effect, retry, idempotency);
                let expected = side_effect == SideEffect::None
                    || ((retry == RetrySafety::Safe || retry == RetrySafety::KeyRequired)
                        && idempotency == Idempotency::IdempotentExternal);
                assert_eq!(is_compile_idempotency_gate_accepted(&contract), expected);
                assert_eq!(
                    vb_validate::idempotency_contract::is_statically_idempotent_contract(&contract)
                        .is_ok(),
                    expected
                );
                assert_eq!(check_idempotency_gates(&[contract]).is_ok(), expected);
                checked = checked
                    .checked_add(1)
                    .ok_or_else(|| "case counter overflow".to_string())?;
            }
        }
    }

    assert_eq!(checked, 45);
    Ok(())
}

#[test]
fn private_compile_modules_are_not_public_when_split_is_complete() {
    let forbidden_public_paths = [
        "pub mod compile",
        "pub mod lower",
        "pub mod validation",
        "pub mod mod_compile_core",
        "pub mod mod_compile_errors",
        "pub mod mod_compile_validation",
        "pub mod mod_compile_lowering",
        "pub use compile",
        "pub use lower",
        "pub use validation",
        "pub use mod_compile_",
    ];
    let leaks: Vec<&str> = forbidden_public_paths
        .into_iter()
        .filter(|needle| VB_COMPILE_LIB_RS.contains(needle))
        .collect();

    assert_eq!(leaks, Vec::<&str>::new());
}

#[test]
fn lib_rs_declares_only_facade_and_private_split_modules_when_refactor_completes() {
    let required_private_modules = [
        "mod mod_compile_core;",
        "mod mod_compile_errors;",
        "mod mod_compile_validation;",
        "mod mod_compile_lowering;",
    ];
    let missing: Vec<&str> = required_private_modules
        .into_iter()
        .filter(|module_decl| !VB_COMPILE_LIB_RS.contains(module_decl))
        .collect();
    let lib_rs_lines = VB_COMPILE_LIB_RS.lines().count();
    let module_sources = [
        ("mod_compile_core", VB_COMPILE_CORE_RS),
        ("mod_compile_errors", VB_COMPILE_ERRORS_RS),
        ("mod_compile_validation", VB_COMPILE_VALIDATION_RS),
        ("mod_compile_lowering", VB_COMPILE_LOWERING_RS),
    ];
    let doc_only_modules: Vec<&str> = module_sources
        .into_iter()
        .filter_map(|(name, source)| {
            let owns_submodules = source.lines().any(|line| line.starts_with("mod "));
            (source.lines().count() < 50 && !owns_submodules).then_some(name)
        })
        .collect();
    let include_bodies: Vec<&str> = module_sources
        .into_iter()
        .filter_map(|(name, source)| source.contains("include!(").then_some(name))
        .collect();
    let hidden_impl = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vb_compile/src/compile_core_impl.rs");

    assert_eq!(missing, Vec::<&str>::new());
    assert_eq!(lib_rs_lines < SOURCE_LINE_LIMIT, true);
    assert_eq!(doc_only_modules, Vec::<&str>::new());
    assert_eq!(include_bodies, Vec::<&str>::new());
    assert_eq!(hidden_impl.exists(), false);
}

#[test]
fn mod_compile_dependency_edges_remain_acyclic_and_diagnostic_leaf() -> Result<(), String> {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vb_compile/src");
    let forbidden_edges = [
        (
            "mod_compile_errors",
            "crate::mod_compile_validation",
            "mod_compile_errors must remain a leaf diagnostic module",
        ),
        (
            "mod_compile_validation",
            "crate::mod_compile_core",
            "mod_compile_validation must not depend on the compile facade",
        ),
    ];
    let mut violations = Vec::new();

    for (module_dir, forbidden, reason) in forbidden_edges {
        let module_root = src_dir.join(module_dir);
        let module_file = src_dir.join(format!("{module_dir}.rs"));
        let mut sources = rust_sources_under(&module_root)?;
        if module_file.exists() {
            let source =
                std::fs::read_to_string(&module_file).map_err(|error| error.to_string())?;
            sources.push((module_file, source));
        }
        for (path, source) in sources {
            if source.contains(forbidden) {
                violations.push(format!("{} contains {forbidden}: {reason}", path.display()));
            }
        }
    }

    violations.sort();
    assert_eq!(violations, Vec::<String>::new());
    Ok(())
}

fn rust_sources_under(root: &std::path::Path) -> Result<Vec<(std::path::PathBuf, String)>, String> {
    let mut sources = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            let entries = std::fs::read_dir(&path).map_err(|error| error.to_string())?;
            for entry_result in entries {
                let entry = entry_result.map_err(|error| error.to_string())?;
                pending.push(entry.path());
            }
            continue;
        }
        if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs") {
            let source = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
            sources.push((path, source));
        }
    }
    Ok(sources)
}

#[test]
fn vb_compile_production_sources_remain_under_agreed_line_limit() -> Result<(), String> {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vb_compile/src");
    let mut source_line_counts = Vec::new();
    let mut pending_dirs = vec![src_dir];

    while let Some(directory) = pending_dirs.pop() {
        let entries = std::fs::read_dir(&directory).map_err(|error| error.to_string())?;
        for entry_result in entries {
            let entry = entry_result.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                let directory_name = path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .ok_or_else(|| "source directory name is not UTF-8".to_string())?;
                if directory_name.starts_with("mod_compile_") {
                    pending_dirs.push(path);
                }
                continue;
            }
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
            let relative_path = path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .map_err(|error| error.to_string())?
                .display()
                .to_string();
            source_line_counts.push((relative_path, source.lines().count()));
        }
    }

    source_line_counts.sort_by(|left, right| left.0.cmp(&right.0));
    let local_oversized_sources: Vec<(String, usize)> = source_line_counts
        .iter()
        .filter_map(|(path, lines)| {
            let local_source = path.ends_with("/lib.rs") || path.contains("/mod_compile_");
            (*lines >= SOURCE_LINE_LIMIT && local_source).then_some((path.clone(), *lines))
        })
        .collect();

    assert_eq!(
        local_oversized_sources,
        Vec::<(String, usize)>::new(),
        "bead-local crates/vb_compile split sources must stay below {SOURCE_LINE_LIMIT} lines; pre-existing unrelated top-level debt is DEFERRED_GLOBAL; observed counts: {source_line_counts:?}"
    );
    Ok(())
}

fn generated_shape_checks(generated: &str) -> Vec<&'static str> {
    let required_snippets = [
        (
            "crate forbids unsafe and denies must-use",
            "#![forbid(unsafe_code)]\n#![deny(unused_must_use)]",
        ),
        (
            "single-slot single-node constants",
            "const WORKFLOW_SLOT_COUNT: usize = 1;\nconst WORKFLOW_NODE_COUNT: u16 = 1;\nconst WORKFLOW_NODE_COUNT_USIZE: usize = 1;",
        ),
        (
            "empty constant pool",
            "const CONSTANTS: [SlotValue; 0] = [\n];",
        ),
        (
            "main drive signature",
            "pub fn drive(mut slots: [Option<SlotValue>; 1]) -> Result<SlotValue, DriveError>",
        ),
        (
            "drive dispatches pc zero to step zero",
            "0 => step_0(&mut slots, &mut slot_taints, &mut list_store, &mut object_store, &mut collect_states)?,",
        ),
        (
            "finish result reads slot zero",
            "fn finish_result_slot(step: u16) -> Result<u16, DriveError> {\n    match step {\n        0 => Ok(0),",
        ),
        (
            "step zero reads slot zero and finishes",
            "fn step_0(slots: &mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT], _slot_taints: &mut [Taint; WORKFLOW_SLOT_COUNT], _list_store: &mut ListStore, _object_store: &mut ObjectStore, _collect_states: &mut CollectStateStore) -> Result<StepOutcome, DriveError> {\n    let value = read_slot(slots, 0)?;\n    Ok(StepOutcome::Finished(value))\n}",
        ),
        (
            "unknown action remains rejected",
            "pub fn dispatch_action(action_id: u16) -> Result<(), DriveError> {\n    match action_id {\n        _ => Err(DriveError::UnknownAction),",
        ),
    ];

    required_snippets
        .into_iter()
        .filter_map(|(label, snippet)| (!generated.contains(snippet)).then_some(label))
        .collect()
}

fn action_contract(
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
) -> ActionContract {
    ActionContract {
        id: ActionId::new(1),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency,
        side_effect,
        retry_safety,
        required_capabilities: Box::new([]),
    }
}

#[allow(dead_code)]
fn minimal_parts(node: CompiledNode) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("minimal"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![node].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}
