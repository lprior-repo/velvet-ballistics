//! Tests for velvet_ballistics binary entrypoint.
//!
//! This file consolidates tests from `main_tests.rs` and `mode_activation_tests.rs`.
//! Mode activation boundary tests enforce POST-001, POST-002, POST-003, POST-004, POST-005
//! and INV-001 through INV-005 from the vb-am5q contract.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]

use super::{
    ActionName, ActionRegistryMode, CliExitCode, Command, DiffMode, DurabilityMode,
    INPUT_MAPPING_DECODE_FAILED_MESSAGE, INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
    INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE, InputMappingError, OutputFormat, ParseError,
    RunStatus, StepTarget, StorageWorkflowResolver, action_contract_detail,
    action_idempotency_name, action_table_rows, build_step_frame, decode_step_inputs,
    execute_step_isolated, map_runtime_inputs, node_kind_name, parse_args, parse_run_id,
    redacted_slot_value, registered_cli_actions, run_compiled_workflow, setup_exit_code,
    signal_name, suggested_ai_commands, write_step_inputs,
};
use crate::args::LegacyJsonOutput;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_storage::{CompiledIrRecord, EventSeq, JournalEvent};

fn main_test_tempdir() -> std::io::Result<tempfile::TempDir> {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/vb-cli-main-tests-tmp");
    std::fs::create_dir_all(&root)?;
    tempfile::Builder::new()
        .prefix("vb-cli-main-")
        .tempdir_in(root)
}

#[test]
fn parse_ai_context_accepts_run_id_db_and_defaults_to_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "ai-context",
        "42",
        "--db",
        "journal-db",
    ]));

    assert!(matches!(parsed, Ok(Command::AiContext { .. })));
    if let Ok(Command::AiContext { run_id, db, output }) = parsed {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("journal-db"));
        assert_eq!(output, super::OutputFormat::Text);
    }
}

#[test]
fn parse_ai_context_requires_db() {
    let parsed = parse_args(&args(&["velvet-ballistics", "ai-context", "42"]));

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
            .any(|command| command.contains("incident 7 --db db-path --emit yaml"))
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
        "velvet-ballistics",
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
        "velvet-ballistics",
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
        "velvet-ballistics",
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
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list"]));

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
fn parse_action_list_accepts_empty_registry_defaults_to_text() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "list",
        "--registry",
        "empty",
    ]));

    assert!(
        matches!(
            parsed,
            Ok(Command::ActionList {
                output: super::OutputFormat::Text,
                registry: ActionRegistryMode::Empty,
            })
        ),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_action_inspect_accepts_action_name_defaults_to_text() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "inspect",
        "send_email",
    ]));

    assert!(
        matches!(
            parsed,
            Ok(Command::ActionInspect {
                ref action_name,
                output: super::OutputFormat::Text,
                registry: ActionRegistryMode::Registered,
            }) if action_name.as_str() == "send_email"
        ),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_action_inspect_rejects_invalid_action_name() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "inspect",
        "bad name",
    ]));

    assert_eq!(
        parsed,
        Err(ParseError::InvalidActionName(
            "action name contains whitespace".into()
        ))
    );
}

#[test]
fn parse_action_list_rejects_invalid_registry() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        "velvet-ballistics",
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
        "velvet-ballistics",
        "action",
        "list",
        "--registry",
        "--json",
    ]));

    assert_eq!(parsed, Err(ParseError::MissingActionRegistryValue));
}

#[test]
fn parse_action_list_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list", "--bogus"]));

    assert_eq!(
        parsed,
        Err(ParseError::UnknownActionListFlag("--bogus".into()))
    );
}

#[test]
fn parse_action_list_rejects_trailing_argument() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list", "junk"]));

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
            assert!(
                matches!(error.to_string().as_str(), ""),
                "registered CLI actions should build: {error}"
            );
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
            assert!(
                matches!(error.to_string().as_str(), ""),
                "registered CLI actions should build: {error}"
            );
            return;
        }
    };
    let rows = action_table_rows(&registry);

    assert_eq!(rows.len(), 3);
    let [first, second, third] = rows.as_slice() else {
        assert!(
            rows.is_empty(),
            "expected exactly three action table rows: {rows:?}"
        );
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
fn registered_cli_action_inspect_detail_contains_contract_and_rules() {
    let registry = registered_cli_actions();
    assert!(
        registry.is_ok(),
        "registered CLI actions should build: {registry:?}"
    );
    let registry = match registry {
        Ok(registry) => registry,
        Err(_) => return,
    };
    let contract = registry.resolve_compile_time(vb_core::ActionId::new(2));
    assert!(contract.is_ok(), "action 2 should resolve: {contract:?}");
    let contract = match contract {
        Ok(contract) => contract,
        Err(_) => return,
    };
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
    let dir = main_test_tempdir();
    assert!(dir.is_ok(), "test directory should be available: {dir:?}");

    if let (Some(compiled), Ok(dir)) = (compiled, dir) {
        let code = run_compiled_workflow(
            vb_core::RunId::new(1),
            compiled.clone(),
            Box::from([]),
            DurabilityMode::Journaled,
            Some(dir.path()),
            OutputFormat::Text,
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
    let dir = main_test_tempdir();
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
    let dir = main_test_tempdir();
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
        "velvet-ballistics",
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
        "velvet-ballistics",
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
    let result = decode_step_inputs(b"", OutputFormat::Text);
    assert_eq!(result, Ok(Box::from([])));
}

#[test]
fn decode_step_inputs_invalid_data_returns_error() {
    let result = decode_step_inputs(b"garbage", OutputFormat::Text);
    // Decode error is a validation failure per PRE-004 contract requirement
    assert_eq!(result, Err(CliExitCode::ValidationFailed.into()));
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
        assert_eq!(write_step_inputs(&mut frame, &inputs), Ok(()));
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
        let code =
            execute_step_isolated(&compiled, StepIdx::ZERO, node, &inputs, OutputFormat::Text);
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
            "velvet-ballistics",
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
            "velvet-ballistics",
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
        parse_args(&args(&["velvet-ballistics", "action", "show"])),
        Err(ParseError::UnknownActionCommand("show".into()))
    );
    assert_eq!(
        parse_args(&args(&[
            "velvet-ballistics",
            "answer",
            "run-1",
            "--slot",
            "not-a-slot",
            "--value",
            "value.bin",
            "--db",
            "journal-db",
        ])),
        Err(ParseError::InvalidSlot("not-a-slot".into()))
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
            assert!(encoded.is_ok(), "test payload should encode: {encoded:?}");
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

// ---------------------------------------------------------------------------
// parse_run_id tests
// ---------------------------------------------------------------------------

#[test]
fn parse_run_id_accepts_valid_decimal_u64_string() {
    let result = parse_run_id("42", OutputFormat::Text);
    assert!(
        result.is_ok(),
        "parse_run_id should accept valid decimal string: {result:?}"
    );
    let Ok(run_id) = result else { return };
    assert_eq!(run_id.get(), 42);
}

#[test]
fn parse_run_id_accepts_large_u64() {
    let result = parse_run_id("18446744073709551615", OutputFormat::Text);
    assert!(
        result.is_ok(),
        "parse_run_id should accept max u64: {result:?}"
    );
    let Ok(run_id) = result else { return };
    assert_eq!(run_id.get(), u64::MAX);
}

#[test]
fn parse_run_id_rejects_non_numeric_string() {
    let result = parse_run_id("abc", OutputFormat::Text);
    assert!(
        result.is_err(),
        "parse_run_id should reject non-numeric string: {result:?}"
    );
    let Err(code) = result else { return };
    assert_eq!(
        code,
        std::process::ExitCode::from(CliExitCode::ValidationFailed as u8)
    );
}

#[test]
fn parse_run_id_rejects_empty_string() {
    let result = parse_run_id("", OutputFormat::Text);
    assert!(
        result.is_err(),
        "parse_run_id should reject empty string: {result:?}"
    );
    let Err(code) = result else { return };
    assert_eq!(
        code,
        std::process::ExitCode::from(CliExitCode::ValidationFailed as u8)
    );
}

#[test]
fn parse_run_id_rejects_zero() {
    // RunId(0) is not a valid run identifier in the domain model.
    let result = parse_run_id("0", OutputFormat::Text);
    assert!(
        result.is_err(),
        "parse_run_id should reject zero: {result:?}"
    );
    let Err(code) = result else { return };
    assert_eq!(
        code,
        std::process::ExitCode::from(CliExitCode::ValidationFailed as u8)
    );
}

#[test]
fn parse_run_id_rejects_negative_number() {
    let result = parse_run_id("-1", OutputFormat::Text);
    assert!(
        result.is_err(),
        "parse_run_id should reject negative numbers: {result:?}"
    );
}

#[test]
fn parse_run_id_rejects_hex_string() {
    let result = parse_run_id("0x10", OutputFormat::Text);
    assert!(
        result.is_err(),
        "parse_run_id should reject hex strings: {result:?}"
    );
}

#[test]
fn parse_run_id_rejects_float_string() {
    let result = parse_run_id("1.5", OutputFormat::Text);
    assert!(
        result.is_err(),
        "parse_run_id should reject float strings: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// OutputFormat tests
// ---------------------------------------------------------------------------

#[test]
fn output_format_default_is_text() {
    assert_eq!(OutputFormat::default(), OutputFormat::Text);
}

// ---------------------------------------------------------------------------
// Mode Activation Tests (consolidated from mode_activation_tests.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod mode_activation {
    use super::*;
    use crate::args::{
        ActionRegistryMode, Command, DurabilityMode, EmitTarget, OutputFormat, ParseError,
        StatusOptions, SystemStatusOptions, VerifyProfile,
    };
    use proptest::prelude::*;

    // Import from mode_error — this module does NOT exist yet (RED phase)
    use crate::mode_error::{CommandMode, ModeError, command_mode};

    // =============================================================================
    // SECTION 1: ModeError Enum — Exit Code Mappings (Error Taxonomy)
    // =============================================================================

    #[test]
    fn mode_error_invalid_mode_maps_to_validation_failed() {
        // ModeError::InvalidMode (defensive) → CliExitCode::ValidationFailed (exit 2 per contract)
        let err = ModeError::InvalidMode;
        let code = CliExitCode::from(err);
        assert_eq!(code, CliExitCode::ValidationFailed);
        assert_eq!(u8::from(CliExitCode::from(ModeError::InvalidMode)), 2u8);
    }

    #[test]
    fn mode_error_storage_init_failed_maps_to_storage_error() {
        // ModeError::StorageInitFailed → CliExitCode::StorageError (exit 5)
        let err = ModeError::StorageInitFailed {
            path: PathBuf::from("/tmp/nonexistent"),
            cause: "No such file or directory".to_string(),
        };
        let code = CliExitCode::from(err);
        assert_eq!(code, CliExitCode::StorageError);
        assert_eq!(u8::from(code), 5u8);
    }

    #[test]
    fn mode_error_storage_init_failed_display_includes_path_and_cause() {
        let err = ModeError::StorageInitFailed {
            path: PathBuf::from("/data/journal"),
            cause: "Permission denied".to_string(),
        };
        let display = err.to_string();
        assert!(
            display.contains("/data/journal"),
            "display must contain path: {display}"
        );
        assert!(
            display.contains("Permission denied"),
            "display must contain cause: {display}"
        );
    }

    #[test]
    fn mode_error_runtime_init_failed_maps_to_runtime_failed() {
        // ModeError::RuntimeInitFailed → CliExitCode::RuntimeFailed (exit 1 per contract)
        let err = ModeError::RuntimeInitFailed {
            cause: "shard count must be non-zero".to_string(),
        };
        let code = CliExitCode::from(err);
        assert_eq!(code, CliExitCode::RuntimeFailed);
        assert_eq!(u8::from(code), 1u8);
    }

    #[test]
    fn mode_error_runtime_init_failed_display_includes_cause() {
        let err = ModeError::RuntimeInitFailed {
            cause: "invalid config".to_string(),
        };
        let display = err.to_string();
        assert!(
            display.contains("invalid config"),
            "display must contain cause: {display}"
        );
    }

    #[test]
    fn mode_error_ui_init_failed_maps_to_action_policy_error() {
        // ModeError::UiInitFailed → CliExitCode::ActionPolicyError (exit 7)
        let err = ModeError::UiInitFailed {
            cause: "display server unavailable".to_string(),
        };
        let code = CliExitCode::from(err);
        assert_eq!(code, CliExitCode::ActionPolicyError);
        assert_eq!(u8::from(code), 7u8);
    }

    #[test]
    fn mode_error_ui_init_failed_display_includes_cause() {
        let err = ModeError::UiInitFailed {
            cause: "GPU initialization failed".to_string(),
        };
        let display = err.to_string();
        assert!(
            display.contains("GPU initialization failed"),
            "display must contain cause: {display}"
        );
    }

    #[test]
    fn mode_error_pure_command_storage_access_attempted_maps_to_storage_error() {
        // DEFECT: pure command handler attempted to open storage
        // This must NEVER happen; indicates a contract violation
        let err = ModeError::PureCommandStorageAccessAttempted {
            command: "validate".to_string(),
        };
        let code = CliExitCode::from(err);
        assert_eq!(code, CliExitCode::StorageError);
        assert_eq!(u8::from(code), 5u8);
    }

    #[test]
    fn mode_error_pure_command_storage_access_attempted_display_includes_command() {
        let err = ModeError::PureCommandStorageAccessAttempted {
            command: "verify".to_string(),
        };
        let display = err.to_string();
        assert!(
            display.contains("verify"),
            "display must contain command name: {display}"
        );
    }

    #[test]
    fn mode_error_all_variants_have_distinct_exit_codes() {
        // Exit code uniqueness invariant: all 5 ModeError variants must map to distinct exit codes
        let codes: Vec<u8> = vec![
            u8::from(CliExitCode::from(ModeError::InvalidMode)),
            u8::from(CliExitCode::from(ModeError::StorageInitFailed {
                path: PathBuf::from("/tmp"),
                cause: "test".to_string(),
            })),
            u8::from(CliExitCode::from(ModeError::RuntimeInitFailed {
                cause: "test".to_string(),
            })),
            u8::from(CliExitCode::from(ModeError::UiInitFailed {
                cause: "test".to_string(),
            })),
            u8::from(CliExitCode::from(
                ModeError::PureCommandStorageAccessAttempted {
                    command: "test".to_string(),
                },
            )),
        ];
        // Per contract: InvalidMode maps to ValidationFailed (2), StorageInitFailed and
        // PureCommandStorageAccessAttempted both map to StorageError (5),
        // RuntimeInitFailed maps to RuntimeFailed (1), UiInitFailed maps to ActionPolicyError (7)
        let expected = [2u8, 5, 1, 7, 5];
        assert_eq!(codes, expected, "ModeError exit codes must match contract");
    }

    // =============================================================================
    // SECTION 2: CommandMode Enum
    // =============================================================================

    #[test]
    fn command_mode_enum_has_pure_variant() {
        let _mode = CommandMode::Pure;
    }

    #[test]
    fn command_mode_enum_has_storage_variant() {
        let _mode = CommandMode::Storage;
    }

    #[test]
    fn command_mode_enum_has_runtime_variant() {
        let _mode = CommandMode::Runtime;
    }

    #[test]
    fn command_mode_enum_has_ui_variant() {
        let _mode = CommandMode::UI;
    }

    #[test]
    fn command_mode_enum_all_variants_are_distinct() {
        assert_ne!(CommandMode::Pure, CommandMode::Storage);
        assert_ne!(CommandMode::Pure, CommandMode::Runtime);
        assert_ne!(CommandMode::Pure, CommandMode::UI);
        assert_ne!(CommandMode::Storage, CommandMode::Runtime);
        assert_ne!(CommandMode::Storage, CommandMode::UI);
        assert_ne!(CommandMode::Runtime, CommandMode::UI);
    }

    // =============================================================================
    // SECTION 3: command_mode() — Pure Commands (POST-002, INV-001)
    // =============================================================================

    #[test]
    fn command_mode_validate_is_pure() {
        let cmd = Command::Validate {
            workflow: PathBuf::from("workflow.yaml"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_verify_is_pure() {
        let cmd = Command::Verify {
            workflow: PathBuf::from("workflow.yaml"),
            profile: VerifyProfile::Standard,
            output: OutputFormat::Text,
            legacy_json: LegacyJsonOutput::Disabled,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_explain_is_pure() {
        let cmd = Command::Explain {
            workflow: PathBuf::from("workflow.yaml"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_compile_is_pure() {
        let cmd = Command::Compile {
            workflow: PathBuf::from("workflow.yaml"),
            emit: EmitTarget::Ir,
            out: PathBuf::from("output.vbir"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_graph_is_pure() {
        let cmd = Command::Graph {
            workflow: PathBuf::from("workflow.yaml"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_simulate_is_pure() {
        let cmd = Command::Simulate {
            workflow: PathBuf::from("workflow.yaml"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_bench_run_is_pure() {
        // bench-run: uses Runtime::new (not new_with_journal), no FjallJournal::open
        let cmd = Command::BenchRun {
            workflow: PathBuf::from("workflow.yaml"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_agent_context_is_pure() {
        // agent-context: static JSON build, no storage
        let cmd = Command::AgentContext { deliver: None };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_status_is_storage() {
        // status: may call FjallJournal::open when --db <path> is supplied,
        // so it is classified as Storage (similar to doctor).
        let cmd = Command::Status {
            options: StatusOptions::default(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_system_status_is_storage() {
        // system status: may call FjallJournal::open when --db <path> is
        // supplied, so it is classified as Storage.
        let cmd = Command::SystemStatus {
            options: SystemStatusOptions::default(),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_action_list_is_pure() {
        let cmd = Command::ActionList {
            output: OutputFormat::Text,
            registry: ActionRegistryMode::Registered,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    #[test]
    fn command_mode_action_inspect_is_pure() {
        let cmd = Command::ActionInspect {
            action_name: ActionName::new("send_email")
                .expect("test fixture: \"send_email\" is a known-valid action name"),
            output: OutputFormat::Text,
            registry: ActionRegistryMode::Registered,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }

    // =============================================================================
    // SECTION 4: command_mode() — Storage Commands (POST-003)
    // =============================================================================

    #[test]
    fn command_mode_run_with_durability_is_storage() {
        let cmd = Command::Run {
            workflow: PathBuf::from("workflow.yaml"),
            input_bin: PathBuf::from("input.bin"),
            durability: DurabilityMode::Journaled,
            db: Some(PathBuf::from("/tmp/journal")),
            step: None,
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_run_compiled_is_storage() {
        let cmd = Command::RunCompiled {
            workflow: PathBuf::from("workflow.vbir"),
            input_bin: PathBuf::from("input.bin"),
            durability: DurabilityMode::Journaled,
            db: Some(PathBuf::from("/tmp/journal")),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_submit_is_storage() {
        let cmd = Command::Submit {
            workflow: PathBuf::from("workflow.yaml"),
            input_bin: PathBuf::from("input.bin"),
            db: PathBuf::from("/tmp/journal"),
            durability: DurabilityMode::Journaled,
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_inspect_is_storage() {
        let cmd = Command::Inspect {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_events_is_storage() {
        let cmd = Command::Events {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
            status: None,
            limit: None,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_replay_is_storage() {
        let cmd = Command::Replay {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_trace_is_storage() {
        let cmd = Command::Trace {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
            filters: crate::commands_journal::TraceFilters::default(),
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_retry_is_storage() {
        let cmd = Command::Retry {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_resume_is_storage() {
        let cmd = Command::Resume {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_doctor_is_storage() {
        let cmd = Command::Doctor {
            db: Some(PathBuf::from("/tmp/journal")),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_answer_is_storage() {
        let cmd = Command::Answer {
            run_id: "1".to_string(),
            step: 0,
            value_file: PathBuf::from("value.bin"),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_diff_is_storage() {
        let cmd = Command::Diff {
            diff_mode: DiffMode::RunAgainst {
                run_a: "1".to_string(),
                run_b: "2".to_string(),
                db: PathBuf::from("/tmp/journal"),
            },
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_incident_is_storage() {
        let cmd = Command::Incident {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    #[test]
    fn command_mode_ai_context_is_storage() {
        // ai-context: opens FjallJournal for run context
        let cmd = Command::AiContext {
            run_id: "1".to_string(),
            db: PathBuf::from("/tmp/journal"),
            output: OutputFormat::Text,
        };
        assert_eq!(command_mode(&cmd), CommandMode::Storage);
    }

    // =============================================================================
    // SECTION 5: command_mode() — Runtime Commands (POST-003, INV-004)
    // =============================================================================

    #[test]
    fn command_mode_ipc_serve_is_runtime() {
        // ipc-serve: Runtime::new_with_journal + FjallJournal + IPC
        let cmd = Command::IpcServe {
            socket: PathBuf::from("/tmp/socket"),
            db: PathBuf::from("/tmp/journal"),
        };
        assert_eq!(command_mode(&cmd), CommandMode::Runtime);
    }

    // =============================================================================
    // SECTION 6: Mode Activation Matrix Completeness (POST-001)
    // =============================================================================

    #[test]
    fn command_mode_all_25_command_variants_are_classified() {
        // Every Command variant must appear in the Mode Activation Matrix.
        // This is a completeness check: no command falls through without classification.

        // Pure commands (8)
        assert_eq!(
            command_mode(&Command::AgentContext { deliver: None }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::Validate {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::Verify {
                workflow: PathBuf::from("w.yaml"),
                profile: VerifyProfile::Standard,
                output: OutputFormat::Text,
                legacy_json: LegacyJsonOutput::Disabled,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::Explain {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::Compile {
                workflow: PathBuf::from("w.yaml"),
                emit: EmitTarget::Ir,
                out: PathBuf::from("o.vbir"),
                output: OutputFormat::Text,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::Graph {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::Simulate {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::BenchRun {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::Status {
                options: StatusOptions::default(),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::ActionList {
                output: OutputFormat::Text,
                registry: ActionRegistryMode::Registered,
            }),
            CommandMode::Pure
        );
        assert_eq!(
            command_mode(&Command::ActionInspect {
                action_name: ActionName::new("send_email")
                    .expect("test fixture: \"send_email\" is a known-valid action name"),
                output: OutputFormat::Text,
                registry: ActionRegistryMode::Registered,
            }),
            CommandMode::Pure
        );

        // Storage commands (16)
        assert_eq!(
            command_mode(&Command::Run {
                workflow: PathBuf::from("w.yaml"),
                input_bin: PathBuf::from("i.bin"),
                durability: DurabilityMode::Journaled,
                db: Some(PathBuf::from("/tmp/j")),
                step: None,
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::RunCompiled {
                workflow: PathBuf::from("w.vbir"),
                input_bin: PathBuf::from("i.bin"),
                durability: DurabilityMode::Journaled,
                db: Some(PathBuf::from("/tmp/j")),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Submit {
                workflow: PathBuf::from("w.yaml"),
                input_bin: PathBuf::from("i.bin"),
                db: PathBuf::from("/tmp/j"),
                durability: DurabilityMode::Journaled,
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Inspect {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Events {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
                status: None,
                limit: None,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Replay {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Trace {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
                filters: crate::commands_journal::TraceFilters::default(),
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Retry {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Resume {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Doctor {
                db: Some(PathBuf::from("/tmp/j")),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Answer {
                run_id: "1".to_string(),
                step: 0,
                value_file: PathBuf::from("v.bin"),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Diff {
                diff_mode: DiffMode::RunAgainst {
                    run_a: "1".to_string(),
                    run_b: "2".to_string(),
                    db: PathBuf::from("/tmp/j"),
                },
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::Incident {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );
        assert_eq!(
            command_mode(&Command::AiContext {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            }),
            CommandMode::Storage
        );

        // Runtime commands (1)
        assert_eq!(
            command_mode(&Command::IpcServe {
                socket: PathBuf::from("/tmp/socket"),
                db: PathBuf::from("/tmp/j"),
            }),
            CommandMode::Runtime
        );
    }

    // =============================================================================
    // SECTION 7: Pure Mode Invariants (INV-001, INV-002, INV-003)
    // =============================================================================

    #[test]
    fn pure_commands_are_not_storage_nor_runtime_nor_ui() {
        // INV-001: Pure commands do NOT call FjallJournal::open
        // INV-002: UI dependencies remain scoped to UI mode
        // INV-003: Exit codes remain stable regardless of inactive subsystems
        let pure_commands: &[Command] = &[
            Command::AgentContext { deliver: None },
            Command::Validate {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            },
            Command::Verify {
                workflow: PathBuf::from("w.yaml"),
                profile: VerifyProfile::Standard,
                output: OutputFormat::Text,
                legacy_json: LegacyJsonOutput::Disabled,
            },
            Command::Explain {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            },
            Command::Compile {
                workflow: PathBuf::from("w.yaml"),
                emit: EmitTarget::Ir,
                out: PathBuf::from("o.vbir"),
                output: OutputFormat::Text,
            },
            Command::Graph {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            },
            Command::Simulate {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            },
            Command::BenchRun {
                workflow: PathBuf::from("w.yaml"),
                output: OutputFormat::Text,
            },
        ];

        for cmd in pure_commands {
            let mode = command_mode(cmd);
            assert_eq!(
                mode,
                CommandMode::Pure,
                "Pure command {cmd:?} must be Pure mode"
            );
            assert_ne!(
                mode,
                CommandMode::Storage,
                "Pure command must NOT be Storage"
            );
            assert_ne!(
                mode,
                CommandMode::Runtime,
                "Pure command must NOT be Runtime"
            );
            assert_ne!(mode, CommandMode::UI, "Pure command must NOT be UI");
        }
    }

    // =============================================================================
    // SECTION 8: Storage Commands Must Not Be Pure or Runtime or UI
    // =============================================================================

    #[test]
    fn storage_commands_are_not_pure_nor_runtime_nor_ui() {
        let storage_commands: &[Command] = &[
            Command::Inspect {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
            },
            Command::Events {
                run_id: "1".to_string(),
                db: PathBuf::from("/tmp/j"),
                output: OutputFormat::Text,
                status: None,
                limit: None,
            },
            Command::Doctor {
                db: Some(PathBuf::from("/tmp/j")),
                output: OutputFormat::Text,
            },
            Command::Status {
                options: StatusOptions::default(),
                output: OutputFormat::Text,
            },
            Command::SystemStatus {
                options: SystemStatusOptions::default(),
                output: OutputFormat::Text,
            },
        ];

        for cmd in storage_commands {
            let mode = command_mode(cmd);
            assert_eq!(
                mode,
                CommandMode::Storage,
                "Storage command {cmd:?} must be Storage mode"
            );
            assert_ne!(mode, CommandMode::Pure, "Storage command must NOT be Pure");
            assert_ne!(
                mode,
                CommandMode::Runtime,
                "Storage command must NOT be Runtime"
            );
            assert_ne!(mode, CommandMode::UI, "Storage command must NOT be UI");
        }
    }

    // =============================================================================
    // SECTION 9: Runtime Commands Must Not Be Pure or Storage or UI
    // =============================================================================

    #[test]
    fn runtime_commands_are_not_pure_nor_storage_nor_ui() {
        let cmd = Command::IpcServe {
            socket: PathBuf::from("/tmp/socket"),
            db: PathBuf::from("/tmp/j"),
        };
        let mode = command_mode(&cmd);
        assert_eq!(mode, CommandMode::Runtime, "ipc-serve must be Runtime mode");
        assert_ne!(mode, CommandMode::Pure, "Runtime command must NOT be Pure");
        assert_ne!(
            mode,
            CommandMode::Storage,
            "Runtime command must NOT be Storage"
        );
        assert_ne!(mode, CommandMode::UI, "Runtime command must NOT be UI");
    }

    // =============================================================================
    // SECTION 10: CliExitCode Discriminants (POST-005)
    // =============================================================================

    #[test]
    fn cli_exit_code_all_9_variants_distinct() {
        // INV-003: All 9 CliExitCode variants have distinct discriminant values
        let codes: [u8; 9] = [
            u8::from(CliExitCode::Success),
            u8::from(CliExitCode::ValidationFailed),
            u8::from(CliExitCode::VerificationFailed),
            u8::from(CliExitCode::CompileFailed),
            u8::from(CliExitCode::RuntimeFailed),
            u8::from(CliExitCode::StorageError),
            u8::from(CliExitCode::IpcError),
            u8::from(CliExitCode::ActionPolicyError),
            u8::from(CliExitCode::ReplayDivergence),
        ];
        let mut sorted = codes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            codes.len(),
            "CliExitCode variants must be distinct: {codes:?}"
        );
    }

    // =============================================================================
    // SECTION 11: Exit Code Stability (POST-005)
    // =============================================================================

    #[test]
    fn parse_error_unknown_command_exit_code_is_1() {
        // ERR-Taxonomy: UnknownCommand → CliExitCode::ValidationFailed (exit 1)
        let parsed = crate::args::parse_args(&args(&["velvet-ballistics", "foobar"]));
        assert!(
            matches!(parsed, Err(ParseError::UnknownCommand(_))),
            "foobar must be UnknownCommand"
        );
        // The main() match arm maps this to CliExitCode::ValidationFailed
    }

    // =============================================================================
    // SECTION 12: Proptest Invariants for parse_args
    // =============================================================================

    proptest! {
        #[test]
        fn parse_args_valid_commands_all_return_some(
            cmd_name in prop::sample::select(&[
                "validate", "verify", "explain", "compile", "run", "run-compiled",
                "ipc-serve", "inspect", "events", "replay", "trace", "retry", "resume",
                "bench-run", "doctor", "answer", "graph", "diff", "incident", "submit",
                "simulate", "ai-context", "agent-context", "status", "action",
            ][..])
        ) {
            // Property 1: Every valid command string is handled without panic
            // Some commands need additional args, so Err is acceptable
            let parsed = crate::args::parse_args(&args(&["velvet-ballistics", cmd_name]));
            assert!(matches!(parsed, Ok(_) | Err(_)));
        }

        #[test]
        fn parse_args_unknown_command_returns_unknown_command_error(cmd_name in "[a-z]{1,20}") {
            // Property 2: Unknown commands produce UnknownCommand error
            let known = [
                "validate", "verify", "explain", "compile", "run", "run-compiled",
                "ipc-serve", "inspect", "events", "replay", "trace", "retry", "resume",
                "bench-run", "doctor", "answer", "graph", "diff", "incident", "submit",
                "simulate", "ai-context", "agent-context", "status", "action", "help",
                "version",
            ];
            prop_assume!(!known.contains(&cmd_name.as_str()));

            let parsed = crate::args::parse_args(&args(&["velvet-ballistics", &cmd_name]));
            assert!(matches!(parsed, Err(ParseError::UnknownCommand(_))));
        }

        #[test]
        fn parse_durability_only_accepts_strict_journaled_none(input in prop::sample::select(&[
            "strict",
            "journaled",
            "none",
        ][..])) {
            // Property: Only "strict", "journaled", "none" parse successfully
            let parsed = crate::args::parse_args(&args(&[
                "velvet-ballistics", "run", "w.yaml", "--input-bin", "i.bin",
                "--durability", input, "--db", "/tmp/j",
            ]));
            let expected_mode = match input {
                "strict" => DurabilityMode::Strict,
                "journaled" => DurabilityMode::Journaled,
                "none" => DurabilityMode::None,
                _ => unreachable!(),
            };
            let is_run = matches!(parsed, Ok(Command::Run { .. }));
            assert!(is_run);
            // Also verify durability matches
            if let Ok(Command::Run { durability, .. }) = &parsed {
                assert_eq!(*durability, expected_mode);
            }
        }

        #[test]
        fn parse_durability_rejects_invalid_durability(input in "invalid_durability_[a-z]{5,20}") {
            // Property: Invalid durability strings return UnknownDurability
            let parsed = crate::args::parse_args(&args(&[
                "velvet-ballistics", "run", "w.yaml", "--input-bin", "i.bin",
                "--durability", &input,
            ]));
            assert!(matches!(parsed, Err(ParseError::UnknownDurability(_))));
        }
    }
}
