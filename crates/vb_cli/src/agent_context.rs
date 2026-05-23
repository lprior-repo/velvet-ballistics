#![forbid(unsafe_code)]

use serde_json::{Map, Value};

/// Build the machine-readable CLI surface for AI agents.
pub(crate) fn build(version: &str) -> Value {
    serde_json::json!({
        "schema_version": "1",
        "kind": "AgentContext",
        "cli": "velvet-ballastics",
        "package": "velvet-ballastics",
        "binary_aliases": ["velvet-ballastics"],
        "version": version,
        "language_version": "velvet-ballastics/v1",
        "agent_contract": {
            "non_interactive_by_default": true,
            "prompt_bypass_flag": "--force",
            "structured_output_flag": "--emit",
            "structured_output_values": ["text", "yaml", "postcard"],
            "legacy_structured_output_flags": ["--json", "--jsonl"],
            "streaming_output_flag": "--jsonl",
            "streaming_output_flag_status": "legacy",
            "stdout": "data only",
            "stderr": "diagnostics only",
            "ansi_when_non_tty": false,
            "bounded_output_required": true,
            "destructive_operations_require_explicit_flag": true,
            "mutation_responses_return_identifiers": true
        },
        "vocabulary_policy": {
            "canonical_output_flag": "--emit",
            "canonical_output_values": ["text", "yaml", "postcard"],
            "legacy_output_flags": ["--json", "--jsonl"],
            "canonical_destructive_bypass_flag": "--force",
            "canonical_resource_verbs": ["get", "list", "create", "update", "delete"],
            "banned_verbs": ["info", "ls"],
            "banned_flags": ["--format=json", "--output=json", "--skip-confirmations"]
        },
        "active_gates": active_gates(),
        "known_blockers": known_blockers(),
        "exit_codes": exit_codes(),
        "enums": enums(),
        "commands": commands(),
        "examples": examples(),
        "planned_agent_primitives": planned_agent_primitives()
    })
}

fn active_gates() -> Value {
    serde_json::json!({
        "validation": {"required": true, "gate": "vb_validate"},
        "verification": {"required": true, "gate": "vb_verify"},
        "compilation": {"required": true, "gate": "vb_compile"},
        "admission": {"required": true, "gate": "vb_storage::admission"},
        "durability": {"required": false, "gate": "vb_storage::FjallJournal"}
    })
}

fn known_blockers() -> Value {
    serde_json::json!({
        "policy": [
            {"category": "validation_failed", "exit_code": 1},
            {"category": "verification_failed", "exit_code": 2},
            {"category": "compile_failed", "exit_code": 3},
            {"category": "runtime_failed", "exit_code": 4},
            {"category": "storage_error", "exit_code": 5},
            {"category": "ipc_error", "exit_code": 6},
            {"category": "action_policy_error", "exit_code": 7},
            {"category": "replay_divergence", "exit_code": 8}
        ],
        "resource": [
            {"category": "slot_count_exceeded"},
            {"category": "input_index_out_of_range"},
            {"category": "journal_capacity"}
        ],
        "capability": [
            {"category": "unregistered_action"},
            {"category": "missing_capability"},
            {"category": "capability_mismatch"}
        ]
    })
}

fn exit_codes() -> Value {
    serde_json::json!({
        "0": "success",
        "1": "runtime failed",
        "2": "validation failed",
        "3": "compile failed",
        "4": "verification failed",
        "5": "storage error",
        "6": "ipc error",
        "7": "action policy error",
        "8": "replay divergence"
    })
}

fn enums() -> Value {
    serde_json::json!({
        "output_emit": ["text", "yaml", "postcard"],
        "compile_artifact_emit": ["ir", "yaml", "postcard"],
        "durability": ["strict", "journaled", "none"],
        "verify_profile": ["quick", "standard", "full"]
    })
}

fn commands() -> Value {
    Value::Object(Map::from_iter([
        command(
            "agent-context",
            serde_json::json!({
                "summary": "Emit this versioned machine-readable CLI schema.",
                "outputs": ["json"],
                "flags": deliver_flags()
            }),
        ),
        command(
            "validate",
            serde_json::json!({
                "summary": "Validate a workflow definition.",
                "positionals": ["workflow.yaml"],
                "flags": emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "verify",
            serde_json::json!({
                "summary": "Verify a workflow with a bounded profile.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"},
                    "--emit": output_emit_spec()
                },
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "explain",
            serde_json::json!({
                "summary": "Explain validation errors in detail.",
                "positionals": ["workflow.yaml"],
                "flags": emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "compile",
            serde_json::json!({
                "summary": "Compile a workflow to an artifact.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--emit": {"type": "enum", "values": ["ir", "yaml", "postcard"], "required": true},
                    "--out": {"type": "path", "required": true}
                },
                "legacy_flags": legacy_json_flags(),
                "output_note": "compile --emit selects artifact format; it does not select operator output"
            }),
        ),
        command(
            "run",
            serde_json::json!({
                "summary": "Compile and execute a workflow.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--input-bin": {"type": "path", "required": true},
                    "--durability": {"type": "enum", "values": ["strict", "journaled", "none"], "required": true},
                    "--db": "path",
                    "--step": "u16",
                    "--step-input": "path",
                    "--emit": output_emit_spec()
                },
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "run-compiled",
            serde_json::json!({
                "summary": "Execute a compiled workflow artifact.",
                "positionals": ["workflow.vbir"],
                "flags": {
                    "--input-bin": {"type": "path", "required": true},
                    "--durability": {"type": "enum", "values": ["strict", "journaled", "none"], "required": true},
                    "--db": "path",
                    "--emit": output_emit_spec()
                },
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "ai-context",
            serde_json::json!({
                "summary": "Emit compact AI context packet for a durable run.",
                "positionals": ["run_id"],
                "flags": db_emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "ipc-serve",
            serde_json::json!({
                "summary": "Start the bounded local binary IPC server.",
                "flags": {"--socket": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}}
            }),
        ),
        command("inspect", run_id_db_command("Inspect a durable run.")),
        command(
            "events",
            run_id_db_command_with_extra_flags(
                "List durable events for a run.",
                serde_json::json!({"--status": "status", "--limit": "i64"}),
            ),
        ),
        command(
            "replay",
            run_id_db_command("Replay a run from the journal."),
        ),
        command(
            "trace",
            run_id_db_command_with_extra_flags(
                "Show step-by-step execution trace.",
                serde_json::json!({
                    "--step": "u16",
                    "--action": "u16",
                    "--status": "status",
                    "--since-seq": "u64",
                    "--until-seq": "u64",
                    "--limit": "usize"
                }),
            ),
        ),
        command(
            "retry",
            run_id_db_command("Retry a failed run from its last successful step."),
        ),
        command("resume", run_id_db_command("Resume a suspended run.")),
        command(
            "bench-run",
            serde_json::json!({
                "summary": "Benchmark a workflow fixture.",
                "positionals": ["workflow.yaml"],
                "flags": emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "doctor",
            serde_json::json!({
                "summary": "Run diagnostic checks.",
                "flags": optional_db_emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "answer",
            serde_json::json!({
                "summary": "Answer a suspended ask step.",
                "positionals": ["run_id"],
                "flags": {"--step": {"type": "u16", "required": true}, "--value-file": {"type": "path", "required": true}, "--db": {"type": "path", "required": true}, "--emit": output_emit_spec()},
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "graph",
            serde_json::json!({
                "summary": "Output the control-flow graph.",
                "positionals": ["workflow.yaml"],
                "flags": emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "diff",
            serde_json::json!({
                "summary": "Compare two durable runs.",
                "positionals": ["run_a", "run_b"],
                "flags": db_emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "incident",
            run_id_db_command("Produce a black-box failure report."),
        ),
        command(
            "submit",
            serde_json::json!({
                "summary": "Submit a workflow run to durable storage.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--input-bin": {"type": "path", "required": true},
                    "--db": {"type": "path", "required": true},
                    "--durability": {"type": "enum", "values": ["strict", "journaled", "none"], "required": true},
                    "--emit": output_emit_spec()
                },
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "simulate",
            serde_json::json!({
                "summary": "Dry-run a workflow without executing actions.",
                "positionals": ["workflow.yaml"],
                "flags": emit_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "cancel",
            serde_json::json!({
                "summary": "Cancel a durable run.",
                "positionals": ["run_id"],
                "flags": {"--db": {"type": "path", "required": true}, "--reason": "text", "--emit": output_emit_spec()},
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "status",
            serde_json::json!({
                "summary": "Report runtime shard status.",
                "flags": {"--active-runs": "usize", "--queue-depth": "usize", "--trace-dropped": "u64", "--emit": {"type": "enum", "values": ["text", "yaml"], "default": "text"}},
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "system status",
            serde_json::json!({
                "summary": "Report bounded system health.",
                "flags": {"--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"}, "--server": {"type": "enum", "values": ["none"], "default": "none"}, "--emit": {"type": "enum", "values": ["text", "yaml"], "default": "text"}},
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "action list",
            serde_json::json!({
                "summary": "List registered action contracts.",
                "flags": action_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
        command(
            "action inspect",
            serde_json::json!({
                "summary": "Show one registered action contract.",
                "positionals": ["action_id"],
                "flags": action_flags(),
                "legacy_flags": legacy_json_flags()
            }),
        ),
    ]))
}

fn command(name: &str, value: Value) -> (String, Value) {
    (name.to_owned(), value)
}

fn output_emit_spec() -> Value {
    serde_json::json!({"type": "enum", "values": ["text", "yaml", "postcard"], "default": "text"})
}

fn emit_flags() -> Value {
    serde_json::json!({"--emit": output_emit_spec()})
}

fn legacy_json_flags() -> Value {
    serde_json::json!({"--json": "legacy bool", "--jsonl": "legacy bool"})
}

fn db_emit_flags() -> Value {
    serde_json::json!({"--db": {"type": "path", "required": true}, "--emit": output_emit_spec()})
}

fn optional_db_emit_flags() -> Value {
    serde_json::json!({"--db": {"type": "path", "required": false}, "--emit": output_emit_spec()})
}

fn action_flags() -> Value {
    serde_json::json!({
        "--emit": output_emit_spec(),
        "--registry": {"type": "enum", "values": ["registered", "empty", "uninitialized"], "default": "registered"}
    })
}

fn deliver_flags() -> Value {
    serde_json::json!({
        "--deliver": {
            "type": "target",
            "required": false,
            "supported": ["stdout", "file:<absolute-path>"],
            "structured_refusal": ["webhook:<url>", "unknown schemes"]
        }
    })
}

fn run_id_db_command(summary: &str) -> Value {
    serde_json::json!({
        "summary": summary,
        "positionals": ["run_id"],
        "flags": db_emit_flags(),
        "legacy_flags": legacy_json_flags()
    })
}

fn run_id_db_command_with_extra_flags(summary: &str, extra_flags: Value) -> Value {
    let mut flags = match db_emit_flags() {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    if let Value::Object(extra) = extra_flags {
        flags.extend(extra);
    }
    serde_json::json!({
        "summary": summary,
        "positionals": ["run_id"],
        "flags": Value::Object(flags),
        "legacy_flags": legacy_json_flags()
    })
}

fn examples() -> Value {
    serde_json::json!([
        {"args": ["agent-context"], "expect_exit": 0},
        {"args": ["help"], "expect_exit": 0},
        {"args": ["version"], "expect_exit": 0},
        {"args": ["status", "--emit", "yaml"], "expect_exit": 0},
        {"args": ["system", "status", "--emit", "yaml"], "expect_exit": 0},
        {"args": ["action", "list", "--emit", "yaml"], "expect_exit": 0}
    ])
}

fn planned_agent_primitives() -> Value {
    serde_json::json!({
        "async_wait_flag": "--wait",
        "jobs_commands": ["jobs list", "jobs get", "jobs prune"],
        "profile_commands": ["profile save", "profile list", "profile show", "profile delete"],
        "delivery_flag": "--deliver",
        "feedback_command": "feedback"
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::build;
    use crate::args::parse_args;

    #[test]
    fn context_has_versioned_schema_and_emit_flag() {
        let context = build("0.1.0");

        assert_eq!(
            context
                .get("schema_version")
                .and_then(serde_json::Value::as_str),
            Some("1")
        );
        assert_eq!(
            context
                .get("agent_contract")
                .and_then(|contract| contract.get("structured_output_flag"))
                .and_then(serde_json::Value::as_str),
            Some("--emit")
        );
    }

    #[test]
    fn context_advertises_only_canonical_binary() {
        let context = build("0.1.0");
        let aliases = context
            .get("binary_aliases")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();

        assert_eq!(
            aliases,
            vec![serde_json::Value::String("velvet-ballastics".to_string())]
        );
    }

    #[test]
    fn context_exposes_parser_surface_commands() {
        let context = build("0.1.0");
        let commands = context
            .get("commands")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();

        for name in [
            "agent-context",
            "ai-context",
            "status",
            "system status",
            "action list",
            "action inspect",
            "cancel",
            "validate",
            "verify",
            "compile",
            "run",
            "run-compiled",
        ] {
            assert!(commands.contains_key(name), "agent context missing {name}");
        }
    }

    #[test]
    fn context_marks_json_flags_legacy_not_canonical() {
        let context = build("0.1.0");
        assert_eq!(
            context
                .get("vocabulary_policy")
                .and_then(|policy| policy.get("canonical_output_flag"))
                .and_then(serde_json::Value::as_str),
            Some("--emit")
        );
        assert_eq!(
            context
                .get("vocabulary_policy")
                .and_then(|policy| policy.get("legacy_output_flags"))
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn advertised_commands_and_emit_values_parse() {
        let context = build("0.1.0");
        let commands = context
            .get("commands")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();

        for name in commands.keys() {
            let parsed = parse_args(&minimal_args(name, None));
            assert!(
                parsed.is_ok(),
                "advertised command must parse: {name}: {parsed:?}"
            );

            let emit_values = commands
                .get(name)
                .and_then(|command| command.get("flags"))
                .and_then(|flags| flags.get("--emit"))
                .and_then(|emit| emit.get("values"))
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            for value in emit_values {
                let Some(raw_emit) = value.as_str() else {
                    continue;
                };
                let parsed = parse_args(&minimal_args(name, Some(raw_emit)));
                assert!(
                    parsed.is_ok(),
                    "advertised emit value must parse: {name} --emit {raw_emit}: {parsed:?}"
                );
            }
        }
    }

    fn minimal_args(command: &str, emit_override: Option<&str>) -> Vec<OsString> {
        let mut args = vec![OsString::from("velvet-ballastics")];
        args.extend(command.split_whitespace().map(OsString::from));
        match command {
            "validate" | "verify" | "explain" | "bench-run" | "graph" | "simulate" => {
                args.push(OsString::from("workflow.yaml"));
                push_emit_override(&mut args, emit_override);
            }
            "compile" => {
                args.push(OsString::from("workflow.yaml"));
                args.push(OsString::from("--emit"));
                args.push(OsString::from(emit_override.unwrap_or("ir")));
                args.push(OsString::from("--out"));
                args.push(OsString::from("workflow.out"));
            }
            "run" => {
                args.push(OsString::from("workflow.yaml"));
                args.extend(
                    ["--input-bin", "input.bin", "--durability", "none"].map(OsString::from),
                );
                push_emit_override(&mut args, emit_override);
            }
            "run-compiled" => {
                args.push(OsString::from("workflow.vbir"));
                args.extend(
                    ["--input-bin", "input.bin", "--durability", "none"].map(OsString::from),
                );
                push_emit_override(&mut args, emit_override);
            }
            "ipc-serve" => {
                args.extend(["--socket", "socket.sock", "--db", "journal-db"].map(OsString::from));
            }
            "inspect" | "events" | "replay" | "trace" | "retry" | "resume" | "incident"
            | "ai-context" => {
                args.push(OsString::from("1"));
                args.extend(["--db", "journal-db"].map(OsString::from));
                push_emit_override(&mut args, emit_override);
            }
            "doctor" => {
                push_emit_override(&mut args, emit_override);
            }
            "answer" => {
                args.push(OsString::from("1"));
                args.extend(
                    [
                        "--step",
                        "1",
                        "--value-file",
                        "answer.bin",
                        "--db",
                        "journal-db",
                    ]
                    .map(OsString::from),
                );
                push_emit_override(&mut args, emit_override);
            }
            "diff" => {
                args.extend(["1", "2", "--db", "journal-db"].map(OsString::from));
                push_emit_override(&mut args, emit_override);
            }
            "submit" => {
                args.push(OsString::from("workflow.yaml"));
                args.extend(
                    [
                        "--input-bin",
                        "input.bin",
                        "--db",
                        "journal-db",
                        "--durability",
                        "none",
                    ]
                    .map(OsString::from),
                );
                push_emit_override(&mut args, emit_override);
            }
            "cancel" => {
                args.push(OsString::from("1"));
                args.extend(["--db", "journal-db"].map(OsString::from));
                push_emit_override(&mut args, emit_override);
            }
            "status" | "system status" | "action list" => {
                push_emit_override(&mut args, emit_override);
            }
            "action inspect" => {
                args.push(OsString::from("1"));
                push_emit_override(&mut args, emit_override);
            }
            _ => {}
        }
        args
    }

    fn push_emit_override(args: &mut Vec<OsString>, emit_override: Option<&str>) {
        if let Some(raw_emit) = emit_override {
            args.push(OsString::from("--emit"));
            args.push(OsString::from(raw_emit));
        }
    }

    #[test]
    fn context_exposes_agent_context_command() {
        let context = build("0.1.0");
        let command = context
            .get("commands")
            .and_then(|commands| commands.get("agent-context"));

        assert!(command.is_some());
    }

    #[test]
    fn context_exposes_active_gates_and_known_blockers() {
        let context = build("0.1.0");

        assert!(
            context
                .get("active_gates")
                .and_then(serde_json::Value::as_object)
                .is_some(),
            "agent context must expose active verification gates"
        );
        assert!(
            context
                .get("known_blockers")
                .and_then(serde_json::Value::as_object)
                .is_some(),
            "agent context must expose known blocker classes"
        );
    }
}
