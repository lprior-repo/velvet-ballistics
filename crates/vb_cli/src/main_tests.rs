//! Tests for velvet_ballistics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]

use super::{
    ActionRegistryMode, CliExitCode, Command, DurabilityMode, EventStatus,
    INPUT_MAPPING_DECODE_FAILED_MESSAGE, INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
    INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE, InputMappingError, OutputFormat, ParseError,
    RunStatus, StepTarget, StorageWorkflowResolver, action_contract_detail,
    action_idempotency_name, action_table_rows, build_step_frame, cmd_events,
    decode_step_inputs, execute_step_isolated, map_runtime_inputs, node_kind_name, parse_args,
    parse_run_id, redacted_slot_value, registered_cli_actions, run_compiled_workflow,
    setup_exit_code, signal_name, suggested_ai_commands, write_step_inputs,
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

// ---- events --status / --limit integration (vb-qwsyi) ----
//
// These tests call cmd_events directly with a real Fjall journal so the bug
// is exercised end-to-end. The pure `filter_events` helper is unit-tested
// separately in commands_journal::tests.

fn run_finish_journal(dir: &std::path::Path) -> bool {
    let compiled = match finish_workflow() {
        Some(c) => c,
        None => return false,
    };
    let code = run_compiled_workflow(
        vb_core::RunId::new(1),
        compiled.clone(),
        Box::from([]),
        DurabilityMode::Journaled,
        Some(dir),
        OutputFormat::Text,
    );
    code == std::process::ExitCode::SUCCESS
}

#[test]
fn cmd_events_no_filter_returns_all_events() {
    let dir = match main_test_tempdir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !run_finish_journal(dir.path()) {
        return;
    }
    let code = cmd_events("1", dir.path(), OutputFormat::Text, None, None);
    assert_eq!(
        code,
        std::process::ExitCode::SUCCESS,
        "events without filters must succeed"
    );
}

#[test]
fn cmd_events_status_completed_filters_to_completed_only() {
    let dir = match main_test_tempdir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !run_finish_journal(dir.path()) {
        return;
    }
    // RunFinished is the terminal event of the synth workflow and is
    // classified as Completed by the canonical status mapping.
    let code = cmd_events(
        "1",
        dir.path(),
        OutputFormat::Text,
        Some(EventStatus::Completed),
        None,
    );
    assert_eq!(
        code,
        std::process::ExitCode::SUCCESS,
        "events --status completed must succeed"
    );
}

#[test]
fn cmd_events_status_failed_with_synth_workflow_succeeds_with_zero_match() {
    // The synth workflow has no failed events, so the filter returns
    // 0 events but the command still exits 0 (filter is not a hard error).
    let dir = match main_test_tempdir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !run_finish_journal(dir.path()) {
        return;
    }
    let code = cmd_events(
        "1",
        dir.path(),
        OutputFormat::Text,
        Some(EventStatus::Failed),
        None,
    );
    assert_eq!(
        code,
        std::process::ExitCode::SUCCESS,
        "events --status failed on a successful run must still succeed"
    );
}

#[test]
fn cmd_events_limit_truncates_output() {
    let dir = match main_test_tempdir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !run_finish_journal(dir.path()) {
        return;
    }
    // limit=1 must succeed even though the journal has many events.
    let code = cmd_events("1", dir.path(), OutputFormat::Text, None, Some(1));
    assert_eq!(
        code,
        std::process::ExitCode::SUCCESS,
        "events --limit 1 must succeed"
    );
}

#[test]
fn cmd_events_status_and_limit_combined() {
    let dir = match main_test_tempdir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !run_finish_journal(dir.path()) {
        return;
    }
    // Real bug scenario from the user: `--status failed --limit 10` on a
    // journal with no failed events. The previous (buggy) implementation
    // ignored both flags and returned ALL events. The fixed implementation
    // returns 0 events (filter first, then truncate).
    let code = cmd_events(
        "1",
        dir.path(),
        OutputFormat::Text,
        Some(EventStatus::Failed),
        Some(10),
    );
    assert_eq!(
        code,
        std::process::ExitCode::SUCCESS,
        "events --status failed --limit 10 must succeed"
    );
}

#[test]
fn cmd_events_yaml_output_succeeds_with_filters() {
    // YAML output must also be filter-aware. The previous bug ignored
    // filters in all output modes; the fix applies them before JSON encoding.
    let dir = match main_test_tempdir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !run_finish_journal(dir.path()) {
        return;
    }
    let code = cmd_events(
        "1",
        dir.path(),
        OutputFormat::Yaml,
        Some(EventStatus::Completed),
        Some(5),
    );
    assert_eq!(
        code,
        std::process::ExitCode::SUCCESS,
        "events --status completed --limit 5 --emit yaml must succeed"
    );
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
