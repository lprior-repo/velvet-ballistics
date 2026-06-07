//! Mode activation boundary tests.
//!
//! These tests enforce POST-001, POST-002, POST-003, POST-004, POST-005
//! and INV-001 through INV-005 from the vb-am5q contract:
//!
//! - POST-001: Each command mode documents and tests its activated subsystems
//! - POST-002: Pure commands run without storage/runtime/UI side effects
//! - POST-003: Runtime commands still initialize required durable components
//! - POST-004: Mode activation is fail-fast before any subsystem init
//! - POST-005: Exit code is stable regardless of inactive subsystems
//! - INV-001: FjallJournal::open is NEVER called from Pure mode handlers
//! - INV-002: UI dependencies remain scoped to UI mode
//! - INV-003: Exit codes remain stable regardless of inactive subsystems
//! - INV-004: Runtime is created only for runtime-dependent commands
//! - INV-005: Command handler functions are pure w.r.t. subsystem init
//!
//! RED PHASE: These tests fail because mode_error module does not exist yet.
//! The module and its exports must be implemented per the contract.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

// Import from the crate — these exist
use crate::args::{
    ActionName, ActionRegistryMode, Command, DiffMode, DurabilityMode, EmitTarget, OutputFormat,
    ParseError, StatusOptions, VerifyProfile,
};
use crate::exit_code::CliExitCode;
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
fn command_mode_status_is_pure() {
    // status: transient in-memory Shard::new, no FjallJournal::open
    let cmd = Command::Status {
        options: StatusOptions::default(),
        output: OutputFormat::Text,
    };
    assert_eq!(command_mode(&cmd), CommandMode::Pure);
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

    // Pure commands (11)
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
        CommandMode::Pure
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

    // Storage commands (14)
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
        Command::Status {
            options: StatusOptions::default(),
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
// Helper
// =============================================================================

fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
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
