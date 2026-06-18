#![forbid(unsafe_code)]
#![cfg_attr(not(kani), allow(dead_code, unused_mut, unused_variables))]

use serde_json::{Map, Value};

mod primitives;

const SCHEMA_VERSION: &str = "1";
const AGENT_CONTEXT_KIND: &str = "AgentContext";
const CLI_NAME: &str = "velvet-ballistics";
const LANGUAGE_VERSION: &str = "velvet-ballistics/v1";

/// Build the machine-readable CLI surface for AI agents.
pub(crate) fn build(version: &str) -> Value {
    serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": AGENT_CONTEXT_KIND,
        "cli": CLI_NAME,
        "binary_aliases": [CLI_NAME],
        "version": version,
        "language_version": LANGUAGE_VERSION,
        "agent_contract": {
            "non_interactive_by_default": true,
            "prompt_bypass_flag": "--force",
            "structured_output_flag": "--emit yaml",
            "machine_output_flag": "--emit postcard",
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
            "canonical_destructive_bypass_flag": "--force",
            "canonical_resource_verbs": ["get", "list", "create", "update", "delete"],
            "banned_verbs": ["info", "ls"],
            "banned_flags": ["--json", "--jsonl", "--format=json", "--output=json", "--skip-confirmations"]
        },
        "active_gates": active_gates(),
        "known_blockers": known_blockers(),
        "exit_codes": exit_codes(),
        "enums": enums(),
        "commands": commands(),
        "planned_agent_primitives": primitives::planned_agent_primitives()
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
        "1": "validation failed",
        "2": "verification failed",
        "3": "compile failed",
        "4": "runtime failed",
        "5": "storage error",
        "6": "ipc error",
        "7": "action policy error",
        "8": "replay divergence"
    })
}

fn enums() -> Value {
    serde_json::json!({
        "emit": ["text", "yaml", "postcard"],
        "compile_emit": ["ir", "yaml", "postcard"],
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
                "flags": {
                    "--deliver": deliver_flag()
                },
                "outputs": ["json"]
            }),
        ),
        command(
            "validate",
            serde_json::json!({
                "summary": "Validate a workflow definition.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "verify",
            serde_json::json!({
                "summary": "Verify a workflow with a bounded profile.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"},
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "explain",
            serde_json::json!({
                "summary": "Dry-run a workflow and report the execution plan with semantic parity to run.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "compile",
            serde_json::json!({
                "summary": "Compile a workflow to an artifact.",
                "positionals": ["workflow.yaml"],
                "flags": {
                    "--emit": compile_emit_flag(),
                    "--out": {"type": "path", "required": true}
                }
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
                    "--emit": output_emit_flag()
                }
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
                    "--emit": output_emit_flag()
                }
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
            serde_json::json!({
                "summary": "List durable events for a run.",
                "positionals": ["run_id"],
                "flags": events_flags()
            }),
        ),
        command(
            "replay",
            run_id_db_command("Replay a run from the journal."),
        ),
        command(
            "trace",
            serde_json::json!({
                "summary": "Show step-by-step execution trace.",
                "positionals": ["run_id"],
                "flags": trace_flags()
            }),
        ),
        command(
            "retry",
            serde_json::json!({
                "summary": "Retry a failed run from its last successful step.",
                "positionals": ["run_id"],
                "flags": retry_flags()
            }),
        ),
        command("resume", run_id_db_command("Resume a suspended run.")),
        command(
            "cancel",
            serde_json::json!({
                "summary": "Cancel a durable run.",
                "positionals": ["run_id"],
                "flags": {
                    "--db": {"type": "path", "required": true},
                    "--reason": {
                        "type": "string",
                        "max_length": crate::args::run_ops::CANCEL_REASON_MAX_CHARS,
                        "length_unit": "characters"
                    },
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "bench-run",
            serde_json::json!({
                "summary": "Benchmark a workflow fixture.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "doctor",
            serde_json::json!({
                "summary": "Run diagnostic checks.",
                "flags": optional_db_output_flags()
            }),
        ),
        command(
            "answer",
            serde_json::json!({
                "summary": "Answer a suspended ask step.",
                "positionals": ["run_id"],
                "flags": {"--slot": {"type": "u16", "required": true}, "--value": {"type": "path", "required": true, "format": "postcard-encoded SlotValue bytes"}, "--db": {"type": "path", "required": true}, "--emit": output_emit_flag()}
            }),
        ),
        command(
            "graph",
            serde_json::json!({
                "summary": "Output the control-flow graph.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command("diff", primitives::diff_command()),
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
                    "--emit": output_emit_flag()
                }
            }),
        ),
        command(
            "simulate",
            serde_json::json!({
                "summary": "Dry-run a workflow without executing actions.",
                "positionals": ["workflow.yaml"],
                "flags": output_flags()
            }),
        ),
        command(
            "help",
            serde_json::json!({
                "summary": "Print this message.",
                "outputs": ["text"],
                "aliases": ["--help", "-h"]
            }),
        ),
        command(
            "version",
            serde_json::json!({
                "summary": "Print version.",
                "outputs": ["text"],
                "aliases": ["--version", "-V"]
            }),
        ),
        command(
            "status",
            serde_json::json!({
                "summary": "Report runtime shard status (with live Fjall probe when --db is supplied).",
                "flags": status_flags()
            }),
        ),
        command(
            "system status",
            serde_json::json!({
                "summary": "Report bounded system health (probes Fjall when --db is supplied).",
                "flags": system_status_flags()
            }),
        ),
        command(
            "action list",
            serde_json::json!({
                "summary": "List registered action contracts.",
                "flags": action_registry_flags()
            }),
        ),
        command(
            "action inspect",
            serde_json::json!({
                "summary": "Show one registered action contract.",
                "positionals": ["action-name"],
                "flags": action_registry_flags()
            }),
        ),
        command(
            "ai-context",
            serde_json::json!({
                "summary": "Emit machine-readable per-run context.",
                "positionals": ["run_id"],
                "flags": db_output_flags()
            }),
        ),
    ]))
}

fn command(name: &str, value: Value) -> (String, Value) {
    (name.to_owned(), value)
}

fn output_flags() -> Value {
    serde_json::json!({"--emit": output_emit_flag()})
}

fn compile_emit_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["ir", "yaml", "postcard"], "required": true})
}

fn deliver_flag() -> Value {
    serde_json::json!({
        "type": "string",
        "required": false,
        "accepted_forms": ["stdout", "file:<absolute-path>", "webhook:<url>"],
        "supported_forms": ["stdout", "file:<absolute-path>"],
        "currently_refused_forms": ["webhook:<url>"]
    })
}

fn action_registry_flags() -> Value {
    serde_json::json!({
        "--emit": output_emit_flag(),
        "--registry": {
            "type": "enum",
            "values": ["registered", "empty", "uninitialized"],
            "default": "registered"
        }
    })
}

fn db_output_flags() -> Value {
    serde_json::json!({"--db": {"type": "path", "required": true}, "--emit": output_emit_flag()})
}

fn optional_db_output_flags() -> Value {
    serde_json::json!({"--db": {"type": "path", "required": false}, "--emit": output_emit_flag()})
}

fn events_flags() -> Value {
    serde_json::json!({
        "--db": {"type": "path", "required": true},
        "--status": event_status_flag(),
        "--limit": {"type": "i64", "required": false},
        "--emit": output_emit_flag()
    })
}

fn retry_flags() -> Value {
    serde_json::json!({
        "--db": {"type": "path", "required": true},
        "--step": {"type": "u16", "required": false},
        "--emit": output_emit_flag()
    })
}

fn trace_flags() -> Value {
    serde_json::json!({
        "--db": {"type": "path", "required": true},
        "--step": {"type": "u16", "required": false},
        "--action": {"type": "u16", "required": false},
        "--status": trace_status_flag(),
        "--since-seq": {"type": "u64", "required": false},
        "--until-seq": {"type": "u64", "required": false},
        "--limit": {"type": "usize", "required": false},
        "--emit": output_emit_flag()
    })
}

fn event_status_flag() -> Value {
    serde_json::json!({
        "type": "enum",
        "required": false,
        "values": ["pending", "active", "waiting_answer", "cancelled", "completed", "failed"]
    })
}

fn trace_status_flag() -> Value {
    event_status_flag()
}

fn status_flags() -> Value {
    let config = vb_runtime::shard::ShardConfig::default();
    serde_json::json!({
        "--active-runs": {"type": "usize", "max": config.max_active_runs},
        "--queue-depth": {"type": "usize", "max": config.command_queue_capacity},
        "--trace-dropped": {"type": "u64"},
        "--db": {"type": "path", "required": false},
        "--emit": text_yaml_emit_flag()
    })
}

fn system_status_flags() -> Value {
    serde_json::json!({
        "--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"},
        "--server": {"type": "enum", "values": ["none"], "default": "none"},
        "--db": {"type": "path", "required": false},
        "--emit": text_yaml_emit_flag()
    })
}

fn output_emit_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["text", "yaml", "postcard"], "default": "text"})
}

fn text_yaml_emit_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["text", "yaml"], "default": "text"})
}

fn run_id_db_command(summary: &str) -> Value {
    serde_json::json!({
        "summary": summary,
        "positionals": ["run_id"],
        "flags": db_output_flags()
    })
}

#[cfg(kani)]
pub(crate) mod kani_shape {
    use super::{AGENT_CONTEXT_KIND, CLI_NAME, LANGUAGE_VERSION, SCHEMA_VERSION};

    const ACTIVE_GATE_COUNT: usize = 5;
    const EXIT_CODE_COUNT: usize = 9;
    const ENUM_COUNT: usize = 4;
    const COMMAND_COUNT: usize = 30;
    const POLICY_BLOCKER_COUNT: usize = 8;
    const RESOURCE_BLOCKER_COUNT: usize = 3;
    const CAPABILITY_BLOCKER_COUNT: usize = 3;
    const BOOL_CONTRACT_COUNT: usize = 5;
    const VOCABULARY_ARRAY_COUNT: usize = 3;
    const STATIC_SERIALIZED_UPPER_BOUND: usize = 4096;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) struct AgentContextShape {
        version_len: usize,
    }

    pub(crate) const fn build_shape(version_len: usize) -> AgentContextShape {
        AgentContextShape { version_len }
    }

    impl AgentContextShape {
        pub(crate) const fn has_required_fields(self) -> bool {
            !SCHEMA_VERSION.is_empty()
                && !AGENT_CONTEXT_KIND.is_empty()
                && !CLI_NAME.is_empty()
                && !LANGUAGE_VERSION.is_empty()
        }

        pub(crate) const fn has_runtime_policy_fields(self) -> bool {
            ACTIVE_GATE_COUNT == 5 && POLICY_BLOCKER_COUNT == 8
        }

        pub(crate) const fn includes_agent_context_command(self) -> bool {
            COMMAND_COUNT >= 1
        }

        pub(crate) const fn exit_code_count(self) -> usize {
            EXIT_CODE_COUNT
        }

        pub(crate) const fn blocker_category_count(self) -> usize {
            3
        }

        pub(crate) const fn serialized_size_upper_bound(self) -> usize {
            match STATIC_SERIALIZED_UPPER_BOUND.checked_add(self.version_len) {
                Some(total) => total,
                None => usize::MAX,
            }
        }

        pub(crate) const fn deterministic_fingerprint(self) -> usize {
            STATIC_SERIALIZED_UPPER_BOUND
                ^ ACTIVE_GATE_COUNT
                ^ EXIT_CODE_COUNT
                ^ ENUM_COUNT
                ^ COMMAND_COUNT
                ^ POLICY_BLOCKER_COUNT
                ^ RESOURCE_BLOCKER_COUNT
                ^ CAPABILITY_BLOCKER_COUNT
                ^ BOOL_CONTRACT_COUNT
                ^ VOCABULARY_ARRAY_COUNT
                ^ self.version_len
        }

        pub(crate) const fn structural_fingerprint(self) -> usize {
            ACTIVE_GATE_COUNT
                ^ EXIT_CODE_COUNT
                ^ ENUM_COUNT
                ^ COMMAND_COUNT
                ^ POLICY_BLOCKER_COUNT
                ^ RESOURCE_BLOCKER_COUNT
                ^ CAPABILITY_BLOCKER_COUNT
                ^ BOOL_CONTRACT_COUNT
                ^ VOCABULARY_ARRAY_COUNT
        }

        pub(crate) const fn policy_blocker_count(self) -> usize {
            POLICY_BLOCKER_COUNT
        }

        pub(crate) const fn resource_blocker_count(self) -> usize {
            RESOURCE_BLOCKER_COUNT
        }

        pub(crate) const fn capability_blocker_count(self) -> usize {
            CAPABILITY_BLOCKER_COUNT
        }

        pub(crate) const fn command_count(self) -> usize {
            COMMAND_COUNT
        }

        pub(crate) const fn output_is_object(self) -> bool {
            true
        }

        pub(crate) const fn enum_count(self) -> usize {
            ENUM_COUNT
        }

        pub(crate) const fn bool_contract_count(self) -> usize {
            BOOL_CONTRACT_COUNT
        }

        pub(crate) const fn vocabulary_array_count(self) -> usize {
            VOCABULARY_ARRAY_COUNT
        }
    }
}

#[cfg(any(test, kani))]
mod tests;
