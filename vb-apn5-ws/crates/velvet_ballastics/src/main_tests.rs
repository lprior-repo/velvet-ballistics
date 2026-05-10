//! Tests for velvet_ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]

use super::{
    ActionRegistryMode, Command, DurabilityMode, INPUT_MAPPING_DECODE_FAILED_MESSAGE,
    INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE, INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
    InputMappingError, ParseError, RunStatus, StepTarget, StorageWorkflowResolver,
    action_contract_detail, action_idempotency_name, action_table_rows, build_step_frame,
    decode_step_inputs, execute_step_isolated, map_runtime_inputs, node_kind_name, parse_args,
    redacted_slot_value, registered_cli_actions, run_compiled_workflow, setup_exit_code,
    signal_name, suggested_ai_commands, write_step_inputs,
};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_storage::{CompiledIrRecord, EventSeq, JournalEvent};

#[test]
fn parse_ai_context_accepts_run_id_db_and_json() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "ai-context",
        "42",
        "--db",
        "journal-db",
        "--json",
    ]));

    assert!(matches!(parsed, Ok(Command::AiContext { .. })));
    if let Ok(Command::AiContext { run_id, db, output }) = parsed {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("journal-db"));
        assert_eq!(output, super::OutputFormat::Json);
    }
}

#[test]
fn parse_ai_context_requires_db() {
    let parsed = parse_args(&args(&["velvet-ballastics", "ai-context", "42"]));

    assert!(matches!(parsed, Err(ParseError::MissingArgument("--db"))));
}

#[test]
fn ai_context_redacts_secret_snapshot_slot_value() {
    let encoded = postcard::to_allocvec(&vb_core::SlotValue::I64(99));
    assert!(encoded.is_ok(), "slot value should encode: {encoded:?}");
    let Ok(encoded) = encoded else {
        return;
    };
    let snapshot = vb_storage::RunSnapshot {
        run: vb_core::RunId::new(1),
        seq: EventSeq::new(1),
        workflow: WorkflowDigest::from_bytes([7; 32]),
        slots: Vec::new(),
        taint: vec![2],
    };

    let value = redacted_slot_value(SlotIdx::ZERO, Some(&encoded), Some(&snapshot));

    assert_eq!(value, serde_json::Value::String("[REDACTED]".to_string()));
}

#[test]
fn ai_context_failed_run_suggests_incident_command() {
    let commands = suggested_ai_commands("7", std::path::Path::new("db-path"), RunStatus::Failed);

    assert!(
        commands
            .iter()
            .any(|command| command.contains("incident 7 --db db-path --json"))
    );
}

fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}

fn finish_workflow() -> Option<CompiledWorkflow> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("finish"),
        digest: WorkflowDigest::from_bytes([9; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn parse_run_accepts_db_for_journaled_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "journaled",
        "--db",
        "journal-db",
    ]));

    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run {
        workflow,
        input_bin,
        durability,
        db,
        ..
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(input_bin, PathBuf::from("input.bin"));
        assert_eq!(durability, DurabilityMode::Journaled);
        assert_eq!(db, Some(PathBuf::from("journal-db")));
    }
}

#[test]
fn parse_run_compiled_requires_db_for_strict_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run-compiled",
        "workflow.vbir",
        "--input-bin",
        "input.bin",
        "--durability",
        "strict",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_run_none_mode_keeps_db_optional() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));

    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run { durability, db, .. }) = parsed {
        assert_eq!(durability, DurabilityMode::None);
        assert_eq!(db, None);
    }
}

#[test]
fn input_mapping_failure_message_uses_stable_code() {
    assert_eq!(
        INPUT_MAPPING_DECODE_FAILED_MESSAGE,
        "INPUT_MAPPING_FAILED: input-bin decode failed"
    );
    assert_eq!(
        INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
        "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count"
    );
    assert_eq!(
        INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
        "INPUT_MAPPING_FAILED: input slot index out of range"
    );
}

#[test]
fn input_mapping_errors_render_exact_variant_messages() {
    assert_eq!(
        InputMappingError::DecodeFailed.to_string(),
        INPUT_MAPPING_DECODE_FAILED_MESSAGE
    );
    assert_eq!(
        InputMappingError::SlotCountExceeded.to_string(),
        INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE
    );
    assert_eq!(
        InputMappingError::SlotIndexOutOfRange.to_string(),
        INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE
    );
}

#[test]
fn parse_action_list_defaults_to_registered_text_output() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "list"]));

    assert!(
        matches!(
            parsed,
            Ok(Command::ActionList {
                output: super::OutputFormat::Text,
                registry: ActionRegistryMode::Registered,
            })
        ),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_action_list_accepts_empty_registry_json() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
        "empty",
        "--json",
    ]));

    assert!(
        matches!(
            parsed,
            Ok(Command::ActionList {
                output: super::OutputFormat::Json,
                registry: ActionRegistryMode::Empty,
            })
        ),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_action_inspect_accepts_action_id_and_json() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "inspect",
        "2",
        "--json",
    ]));

    assert!(
        matches!(
            parsed,
            Ok(Command::ActionInspect {
                action_id: 2,
                output: super::OutputFormat::Json,
                registry: ActionRegistryMode::Registered,
            })
        ),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_action_inspect_rejects_invalid_action_id() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "inspect",
        "not-a-number",
    ]));

    assert_eq!(
        parsed,
        Err(ParseError::InvalidActionId("not-a-number".into()))
    );
}

#[test]
fn parse_action_list_rejects_invalid_registry() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
        "bogus",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::UnknownActionRegistry(ref registry)) if registry == "bogus"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_action_list_rejects_missing_registry_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
    ]));

    assert_eq!(parsed, Err(ParseError::MissingActionRegistryValue));
    assert_eq!(
        ParseError::MissingActionRegistryValue.to_string(),
        "missing action-args value for --registry (expected: registered, empty, uninitialized)"
    );
}

#[test]
fn parse_action_list_rejects_registry_value_consuming_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
        "--json",
    ]));

    assert_eq!(parsed, Err(ParseError::MissingActionRegistryValue));
}

#[test]
fn parse_action_list_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "list", "--bogus"]));

    assert_eq!(
        parsed,
        Err(ParseError::UnknownActionListFlag("--bogus".into()))
    );
}

#[test]
fn parse_action_list_rejects_trailing_argument() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "list", "junk"]));

    assert_eq!(
        parsed,
        Err(ParseError::UnexpectedActionListArgument("junk".into()))
    );
}

#[test]
fn registered_cli_actions_returns_three_sorted_contracts_without_mutation() {
    let registry = match registered_cli_actions() {
        Ok(registry) => registry,
        Err(error) => {
            assert!(false, "registered CLI actions should build: {error}");
            return;
        }
    };
    let before_len = registry.len();
    let first_listing: Vec<u16> = registry
        .registered_contracts()
        .iter()
        .map(|contract| contract.id.get())
        .collect();
    let second_listing: Vec<u16> = registry
        .registered_contracts()
        .iter()
        .map(|contract| contract.id.get())
        .collect();

    assert_eq!(first_listing, vec![1, 2, 3]);
    assert_eq!(second_listing, first_listing);
    assert_eq!(registry.len(), before_len);
}

#[test]
fn registered_cli_action_table_rows_are_exact() {
    let registry = match registered_cli_actions() {
        Ok(registry) => registry,
        Err(error) => {
            assert!(false, "registered CLI actions should build: {error}");
            return;
        }
    };
    let rows = action_table_rows(&registry);

    assert_eq!(rows.len(), 3);
    let [first, second, third] = rows.as_slice() else {
        assert!(false, "expected exactly three action table rows: {rows:?}");
        return;
    };
    assert_eq!(first.id, 1);
    assert_eq!(first.idempotency, "deterministic_pure");
    assert_eq!(first.retry_safety, "safe");
    assert_eq!(first.side_effect, "none");
    assert_eq!(first.input_slot_count, 1);
    assert_eq!(first.output_slot_count, 1);
    assert_eq!(first.timeout_ms, 1_000);
    assert_eq!(second.id, 2);
    assert_eq!(second.idempotency, "idempotent_external");
    assert_eq!(second.retry_safety, "key_required");
    assert_eq!(second.side_effect, "writes");
    assert_eq!(second.input_slot_count, 2);
    assert_eq!(second.output_slot_count, 1);
    assert_eq!(second.timeout_ms, 5_000);
    assert_eq!(third.id, 3);
    assert_eq!(third.idempotency, "at_least_once_external");
    assert_eq!(third.retry_safety, "unsafe");
    assert_eq!(third.side_effect, "sends");
    assert_eq!(third.input_slot_count, 1);
    assert_eq!(third.output_slot_count, 0);
    assert_eq!(third.timeout_ms, 10_000);
}

#[test]
fn registered_cli_action_inspect_detail_contains_contract_and_rules() -> Result<(), String> {
    let registry = registered_cli_actions()
        .map_err(|error| format!("registered CLI actions should build: {error}"))?;
    let contract = registry
        .resolve_compile_time(vb_core::ActionId::new(2))
        .map_err(|error| format!("action 2 should resolve: {error}"))?;
    let detail = action_contract_detail(contract);

    assert_eq!(detail.id, 2);
    assert_eq!(detail.idempotency, "idempotent_external");
    assert_eq!(detail.retry_safety, "key_required");
    assert_eq!(detail.side_effect, "writes");
    assert_eq!(detail.input_slot_count, 2);
    assert_eq!(detail.output_slot_count, 1);
    assert_eq!(detail.max_input_bytes, 65_536);
    assert_eq!(detail.max_output_bytes, 65_536);
    assert_eq!(detail.timeout_ms, 5_000);
    assert!(detail.failure_codes.contains(&"permission_denied"));
    assert_eq!(
        detail.idempotency_rule,
        "external retries require a stable idempotency key"
    );
    Ok(())
}

#[test]
fn action_contract_names_are_stable_for_json_and_table_output() {
    assert_eq!(
        action_idempotency_name(vb_core::action::Idempotency::DeterministicPure),
        "deterministic_pure"
    );
}

#[test]
fn map_runtime_inputs_decodes_slot_values() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {
        let values: Box<[vb_core::SlotValue]> = Box::from([vb_core::SlotValue::Bool(true)]);
        let payload = postcard::to_allocvec(&values);
        assert_eq!(payload.as_ref().map(|_| ()), Ok(()));
        let Ok(payload) = payload else {
            return;
        };
        let mapped = map_runtime_inputs(&compiled, &payload);
        assert_eq!(
            mapped,
            Ok(Box::from([(
                vb_core::SlotIdx::ZERO,
                vb_core::SlotValue::Bool(true)
            )]))
        );
    }
}

#[test]
fn map_runtime_inputs_rejects_malformed_input_bin() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {
        let mapped = map_runtime_inputs(&compiled, b"not-postcard");
        assert_eq!(mapped, Err(InputMappingError::DecodeFailed));
    }
}

#[test]
fn map_runtime_inputs_rejects_excess_slots_with_exact_variant() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {
        let values: Box<[vb_core::SlotValue]> = Box::from([
            vb_core::SlotValue::Bool(true),
            vb_core::SlotValue::Bool(false),
        ]);
        let payload = postcard::to_allocvec(&values);
        assert_eq!(payload.as_ref().map(|_| ()), Ok(()));
        let Ok(payload) = payload else {
            return;
        };
        let mapped = map_runtime_inputs(&compiled, &payload);
        assert!(
            matches!(mapped, Err(InputMappingError::SlotCountExceeded)),
            "excess slots should return SlotCountExceeded: {mapped:?}"
        );
    }
}

#[test]
fn journaled_run_writes_storage_events() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "test directory should be available: {dir:?}");

    if let (Some(compiled), Ok(dir)) = (compiled, dir) {
        let code = run_compiled_workflow(
            &compiled,
            Box::from([]),
            DurabilityMode::Journaled,
            Some(dir.path()),
        );
        assert_eq!(code, std::process::ExitCode::SUCCESS);

        let journal = vb_storage::FjallJournal::open(dir.path(), None);
        assert!(journal.is_ok(), "journal should reopen");
        if let Ok(journal) = journal {
            let run = vb_core::RunId::new(1);
            let events = journal.events_for_run(run);
            assert!(events.is_ok(), "events should be readable: {events:?}");

            if let Ok(events) = events {
                assert!(events.contains(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: WorkflowDigest::from_bytes([9; 32]),
                }));
                assert!(events.iter().any(|e| matches!(
                    e,
                    JournalEvent::RunFinished {
                        run: r,
                        result: SlotIdx::ZERO,
                        ..
                    } if *r == run
                )));
            }
        }
    }
}

#[test]
fn ipc_storage_resolver_loads_compiled_ir_from_journal() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "test directory should be available: {dir:?}");

    if let (Some(compiled), Ok(dir)) = (compiled, dir) {
        let journal = vb_storage::FjallJournal::open(dir.path(), None);
        assert!(journal.is_ok(), "journal should open");
        let Ok(journal) = journal else {
            return;
        };
        let parts = compiled.to_parts();
        let ir = postcard::to_allocvec(&parts);
        assert!(ir.is_ok(), "workflow parts should encode: {ir:?}");
        let Ok(ir) = ir else {
            return;
        };
        let record = CompiledIrRecord {
            digest: compiled.digest(),
            ir,
        };
        assert!(
            journal.put_compiled_ir(&record).is_ok(),
            "put_compiled_ir must succeed"
        );
        let mut resolver = StorageWorkflowResolver {
            journal: Arc::new(journal),
        };

        let resolved =
            vb_ipc::server::WorkflowResolver::resolve_workflow(&mut resolver, compiled.digest());

        assert!(resolved.is_ok(), "resolver should load compiled IR");
        let Ok(resolved) = resolved else {
            return;
        };
        assert_eq!(resolved.digest(), compiled.digest());
    }
}

#[test]
fn ipc_storage_resolver_returns_not_found_for_missing_digest() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "test directory should be available: {dir:?}");
    if let Ok(dir) = dir {
        let journal = vb_storage::FjallJournal::open(dir.path(), None);
        assert!(journal.is_ok(), "journal should open");
        let Ok(journal) = journal else {
            return;
        };
        let mut resolver = StorageWorkflowResolver {
            journal: Arc::new(journal),
        };

        let result = vb_ipc::server::WorkflowResolver::resolve_workflow(
            &mut resolver,
            WorkflowDigest::from_bytes([5; 32]),
        );

        assert!(
            matches!(
                result,
                Err(vb_ipc::server::WorkflowResolutionError::NotFound)
            ),
            "missing digest should return NotFound"
        );
    }
}

#[test]
fn parse_run_without_step_flags_produces_none_step() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run { step, .. }) = parsed {
        assert!(step.is_none());
    }
}

#[test]
fn parse_run_step_requires_step_input() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--step",
        "0",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--step-input"))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn step_target_holds_step_id_and_path() {
    let target = StepTarget {
        step_id: 5,
        step_input: PathBuf::from("data.bin"),
    };
    assert_eq!(target.step_id, 5);
    assert_eq!(target.step_input, PathBuf::from("data.bin"));
}

#[test]
fn node_kind_name_returns_correct_labels() {
    assert_eq!(node_kind_name(&CompiledNodeKind::Nop), "Nop");
    assert_eq!(
        node_kind_name(&CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }),
        "SetConst"
    );
    assert_eq!(
        node_kind_name(&CompiledNodeKind::Finish {
            result: SlotIdx::ZERO
        }),
        "Finish"
    );
}

#[test]
fn signal_name_returns_correct_labels() {
    assert_eq!(signal_name(&vb_core::EngineSignal::Continue), "Continue");
    assert_eq!(
        signal_name(&vb_core::EngineSignal::Finished(
            vb_core::SlotValue::Bool(true),
            vb_core::Taint::Clean
        )),
        "Finished"
    );
}

#[test]
fn decode_step_inputs_empty_data_returns_empty() {
    let result = decode_step_inputs(b"");
    assert_eq!(result, Ok(Box::from([])));
}

#[test]
fn decode_step_inputs_invalid_data_returns_error() {
    let result = decode_step_inputs(b"garbage");
    assert_eq!(result, Err(setup_exit_code()));
}

#[test]
fn write_step_inputs_populates_frame_slots() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {
        let frame = build_step_frame(&compiled, StepIdx::ZERO);
        assert!(frame.is_ok(), "frame should build: {frame:?}");
        let Ok(mut frame) = frame else {
            return;
        };
        let inputs: Box<[vb_core::SlotValue]> = Box::from([vb_core::SlotValue::I64(42)]);
        write_step_inputs(&mut frame, &inputs);
        assert_eq!(
            frame.read_slot(SlotIdx::ZERO),
            Ok(&vb_core::SlotValue::I64(42))
        );
    }
}

#[test]
fn execute_step_isolated_set_const_step_succeeds() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {
        let node = compiled.node(StepIdx::ZERO);
        assert!(node.is_some(), "step 0 must exist");
        let Some(node) = node else {
            return;
        };
        let inputs: Box<[vb_core::SlotValue]> = Box::from([]);
        let code = execute_step_isolated(&compiled, StepIdx::ZERO, node, &inputs);
        assert_eq!(code, std::process::ExitCode::SUCCESS);
    }
}

#[test]
fn build_step_frame_out_of_range_returns_error() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {
        let result = build_step_frame(&compiled, StepIdx::new(99));
        assert_eq!(result, Err(setup_exit_code()));
    }
}

#[test]
fn parse_error_exact_variant_coverage() {
    assert_eq!(
        parse_args(&args(&[
            "velvet-ballastics",
            "compile",
            "workflow.yaml",
            "--emit",
            "binary",
            "--out",
            "workflow.out",
        ])),
        Err(ParseError::UnknownEmitTarget("binary".into()))
    );
    assert_eq!(
        parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "eventual",
        ])),
        Err(ParseError::UnknownDurability("eventual".into()))
    );
    assert_eq!(
        parse_args(&args(&["velvet-ballastics", "action", "show"])),
        Err(ParseError::UnknownActionCommand("show".into()))
    );
    assert_eq!(
        parse_args(&args(&[
            "velvet-ballastics",
            "answer",
            "run-1",
            "--step",
            "not-a-step",
            "--value-file",
            "value.bin",
            "--db",
            "journal-db",
        ])),
        Err(ParseError::InvalidStep("not-a-step".into()))
    );
}

#[test]
fn input_mapping_error_exact_variant_coverage() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {
        assert_eq!(
            map_runtime_inputs(&compiled, b"not-postcard"),
            Err(InputMappingError::DecodeFailed)
        );

        let too_many_values: Box<[vb_core::SlotValue]> = Box::from([
            vb_core::SlotValue::Bool(true),
            vb_core::SlotValue::Bool(false),
        ]);
        let encoded = postcard::to_allocvec(&too_many_values);
        if let Ok(encoded) = encoded {
            assert_eq!(
                map_runtime_inputs(&compiled, &encoded),
                Err(InputMappingError::SlotCountExceeded)
            );
        } else {
            assert!(false, "test payload should encode: {encoded:?}");
        }
    }

    assert_eq!(
        InputMappingError::SlotIndexOutOfRange.to_string(),
        INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE
    );
    assert_eq!(
        InputMappingError::DecodeFailed.to_string(),
        INPUT_MAPPING_DECODE_FAILED_MESSAGE
    );
    assert_eq!(
        InputMappingError::SlotCountExceeded.to_string(),
        INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE
    );
}
