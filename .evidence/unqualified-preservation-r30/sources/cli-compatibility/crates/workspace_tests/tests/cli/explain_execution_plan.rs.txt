#![forbid(unsafe_code)]
use std::process::{Command, Output};
const SOURCE: &str = "version: velvet-ballistics/v1\nname: factual_plan\nwhen: {manual: {}}\nsecrets: {token: PLAN_OPAQUE_KEY}\nsteps:\n  - id: seed\n    set: {output: value, value: '42'}\n  - id: call\n    try_again: {max_attempts: 2, delay: '100ms'}\n    do: {action: '1', input: '$secrets.token'}\n  - id: done\n    finish: {result: '$step.seed'}\n";
fn run(path: &std::path::Path, emit: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("explain")
        .arg(path)
        .args(["--emit", emit])
        .env("PLAN_OPAQUE_KEY", "HOST_SECRET_MUST_NOT_APPEAR")
        .output()
        .expect("CLI")
}
#[test]
fn explain_reports_actual_current_plan_and_deferred_verification() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("plan.yaml");
    std::fs::write(&path, SOURCE).expect("source");
    let output = run(&path, "yaml");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::str::from_utf8(&output.stdout).expect("UTF8");
    assert!(!text.contains("HOST_SECRET_MUST_NOT_APPEAR"));
    let report: serde_json::Value = serde_saphyr::from_str(text).expect("YAML");
    assert_eq!(report["execution_plan"]["trigger"]["kind"], "manual");
    assert_eq!(report["verification"]["status"], "partial");
    assert_eq!(report["execution_plan"]["actions"][0]["action"], 1);
    assert_eq!(
        report["execution_plan"]["actions"][0]["retry"]["max_attempts"],
        2
    );
    assert_eq!(
        report["execution_plan"]["actions"][0]["contract"]["retry_safety"],
        "idempotent"
    );
    assert_eq!(
        report["execution_plan"]["actions"][0]["contract"]["side_effect"],
        "pure"
    );
    assert_eq!(
        report["execution_plan"]["secrets"][0]["reference"],
        "PLAN_OPAQUE_KEY"
    );
    assert!(
        report["execution_plan"]["budget"]["max_total_steps"]
            .as_u64()
            .is_some_and(|n| n > 0)
    );
    assert!(report["execution_plan"]["resource_contract"]["max_journal_batch_bytes"].is_number());
    assert!(
        report["execution_plan"]["graph"]["nodes"]
            .as_array()
            .is_some_and(|nodes| nodes.len() >= 3)
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("source unchanged"),
        SOURCE
    );
    assert_eq!(std::fs::read_dir(dir.path()).expect("dir").count(), 1);
}

#[path = "../../../velvet_ballistics/src/cli_postcard.rs"]
mod cli_postcard;
#[test]
fn explain_postcard_and_text_preserve_same_factual_plan() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("plan.yaml");
    std::fs::write(&path, SOURCE).expect("source");
    let yaml: serde_json::Value =
        serde_saphyr::from_str(std::str::from_utf8(&run(&path, "yaml").stdout).expect("UTF8"))
            .expect("YAML");
    let binary = run(&path, "postcard");
    assert!(binary.status.success());
    let (_, packet) = cli_postcard::decode_postcard_json(&binary.stdout).expect("Postcard");
    assert_eq!(packet, yaml);
    let text = run(&path, "text");
    assert!(text.status.success());
    let text: serde_json::Value =
        serde_saphyr::from_str(std::str::from_utf8(&text.stdout).expect("UTF8"))
            .expect("text plan");
    assert_eq!(text, yaml);
}

#[test]
fn explain_nested_together_has_literal_control_and_layout() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("nested.yaml");
    std::fs::write(
        &path,
        include_str!("../vb_f2_flow_regressions/together_nested_single_result.yaml"),
    )
    .expect("source");
    let output = run(&path, "yaml");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_saphyr::from_str(std::str::from_utf8(&output.stdout).expect("UTF8")).expect("YAML");
    let plan = &report["execution_plan"];
    assert_eq!(plan["graph"]["nodes"][1]["kind"], "TogetherStart");
    assert!(
        plan["graph"]["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["label"] == "join")
    );
    assert_eq!(
        plan["layout"],
        serde_json::json!({"slots": 4, "expressions": 1, "accessors": 0, "constants": 1, "symbols": 0})
    );
    assert_eq!(plan["budget"]["max_total_steps"], 7);
    assert_eq!(plan["graph"]["nodes"].as_array().expect("nodes").len(), 7);
    assert_eq!(
        plan["graph"]["edges"],
        serde_json::json!([
            {"from": 0, "to": 1, "label": "next"}, {"from": 1, "to": 2, "label": "branch"},
            {"from": 1, "to": 5, "label": "join"}, {"from": 2, "to": 3, "label": "entry"},
            {"from": 2, "to": 5, "label": "join"}, {"from": 3, "to": 4, "label": "next"},
            {"from": 4, "to": 5, "label": "next"}, {"from": 5, "to": 6, "label": "next"}
        ])
    );
    assert_eq!(plan["suspension_points"], serde_json::json!([]));
}

#[test]
fn review_explain_emits_every_whole_workflow_budget_field() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("plan.yaml");
    std::fs::write(&path, SOURCE).expect("source");
    let output = run(&path, "yaml");
    assert!(output.status.success());
    let report: serde_json::Value =
        serde_saphyr::from_str(std::str::from_utf8(&output.stdout).expect("UTF8")).expect("YAML");
    let budget = report["execution_plan"]["budget"]
        .as_object()
        .expect("budget");
    let mut expected = vec![
        "max_total_steps",
        "max_total_slots",
        "max_fanout",
        "max_nesting_depth",
        "max_steps_executable",
        "max_action_tickets",
        "max_parallel_in_flight",
        "max_retries_per_action",
        "max_gather_pages",
        "max_gather_items",
        "max_for_each_iterations",
        "max_together_branches",
        "max_repeat_attempts",
        "max_run_time_seconds",
        "max_result_bytes",
        "max_total_slots_written",
        "max_timer_entries",
        "max_trace_events",
        "max_journal_batch_bytes",
        "max_queue_depth",
        "max_ipc_payload_bytes",
        "max_blob_bytes",
        "max_input_bytes",
        "max_open_gates",
        "max_signal_subscriptions",
        "max_incident_emissions",
        "max_transitions",
        "max_journal_bytes_total",
        "max_value_store_entries",
        "max_cost_units",
    ];
    expected.sort_unstable();
    let actual: Vec<_> = budget
        .keys()
        .filter(|key| key.starts_with("max_"))
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(budget["max_action_tickets"], 1);
    assert_eq!(budget["max_retries_per_action"], 3);
    assert_eq!(budget["max_cost_units"], 4);
}

fn report_for(source: &str) -> serde_json::Value {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("plan.yaml");
    std::fs::write(&path, source).expect("source");
    let output = run(&path, "yaml");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_saphyr::from_str(std::str::from_utf8(&output.stdout).expect("UTF8")).expect("YAML")
}

fn source_with_error_and_unused_secret() -> String {
    SOURCE
        .replace(
            "secrets: {token: PLAN_OPAQUE_KEY}",
            "secrets: {token: PLAN_OPAQUE_KEY, spare: PLAN_UNUSED_KEY}",
        )
        .replace("    do:", "    on_error: {handler: done}\n    do:")
}

#[test]
fn review_action_retry_suspensions_and_secret_reads_are_positive() {
    let source = source_with_error_and_unused_secret();
    let report = report_for(&source);
    let plan = &report["execution_plan"];
    assert_eq!(
        plan["suspension_points"],
        serde_json::json!([
            {"step": 1, "kind": "action"}, {"step": 1, "kind": "retry_timer"}
        ])
    );
    assert_eq!(
        plan["secrets"],
        serde_json::json!([
            {"alias": "token", "reference": "PLAN_OPAQUE_KEY", "input_slot": 2,
             "direct_reference_steps": [{"step": 1, "name": "call"}]},
            {"alias": "spare", "reference": "PLAN_UNUSED_KEY", "input_slot": 3,
             "direct_reference_steps": []}
        ])
    );
    for source in [
        source.replace("max_attempts: 2", "max_attempts: 1"),
        source.replace("delay: '100ms'", "delay: '0ms'"),
        source.replace("    try_again: {max_attempts: 2, delay: '100ms'}\n", ""),
    ] {
        assert_eq!(
            report_for(&source)["execution_plan"]["suspension_points"],
            serde_json::json!([{"step": 1, "kind": "action"}])
        );
    }
}

#[test]
fn review_graph_payload_and_complete_resources_match_public_compilation() {
    let source = source_with_error_and_unused_secret();
    let compiled =
        vb_compile::compile_workflow(source.as_bytes()).expect("public compiled fixture");
    let report = report_for(&source);
    let plan = &report["execution_plan"];
    assert_eq!(
        plan["graph"]["nodes"],
        serde_json::json!([
            {"step": 0, "name": "seed", "kind": "SetConst", "parameters": {"SetConst": {"value": 0}}, "output_slot": 0, "next_step": 1, "on_error_step": null, "retry": null},
            {"step": 1, "name": "call", "kind": "Do", "parameters": {"Do": {"action": 1, "input": 2}}, "output_slot": 1, "next_step": 2, "on_error_step": 2, "retry": {"delay_ms": 100, "max_attempts": 2}},
            {"step": 2, "name": "done", "kind": "Finish", "parameters": {"Finish": {"result": 0}}, "output_slot": null, "next_step": null, "on_error_step": null, "retry": null}
        ])
    );
    let parts = compiled.to_parts();
    for (reported, node) in plan["graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .zip(parts.nodes.iter())
    {
        assert_eq!(
            reported["parameters"],
            serde_json::to_value(&node.kind).expect("kind")
        );
        assert_eq!(
            reported["output_slot"],
            serde_json::to_value(node.output.slot().map(vb_core::SlotIdx::get)).expect("output")
        );
        assert_eq!(
            reported["next_step"],
            serde_json::to_value(node.routing.next.step().map(vb_core::StepIdx::get))
                .expect("next")
        );
        assert_eq!(
            reported["on_error_step"],
            serde_json::to_value(node.routing.error.handler().map(vb_core::StepIdx::get))
                .expect("error")
        );
        assert_eq!(
            reported["retry"],
            serde_json::to_value(node.retry).expect("retry")
        );
    }
    assert_eq!(
        plan["resource_contract"],
        serde_json::to_value(compiled.resource_contract()).expect("all resource fields")
    );
    assert_eq!(
        plan["graph"]["edges"],
        serde_json::json!([
            {"from": 0, "to": 1, "label": "next"}, {"from": 1, "to": 2, "label": "next"}, {"from": 1, "to": 2, "label": "on_error"}
        ])
    );
}

#[test]
fn review_timer_event_and_ask_suspensions_are_positive() {
    let source = "version: velvet-ballistics/v1\nname: positive_suspensions\nwhen: {manual: {}}\nvars: {delay: 10, signal: 1, question: Proceed}\nsteps:\n  - id: timer\n    wait: {timeout: '$vars.delay'}\n  - id: signal\n    wait: {event: '$vars.signal', timeout: '$vars.delay'}\n  - id: ask_for\n    ask: {prompt: '$vars.question', timeout: '$vars.delay'}\n  - id: done\n    finish: {result: '$vars.delay'}\n";
    let report = report_for(source);
    assert_eq!(
        report["execution_plan"]["suspension_points"],
        serde_json::json!([
            {"step": 3, "kind": "timer"}, {"step": 4, "kind": "event"}, {"step": 5, "kind": "ask"}
        ])
    );
}

#[test]
fn review_all_current_trigger_forms_are_factual() {
    for (trigger, expected) in [
        ("manual: {}", serde_json::json!({"kind": "manual"})),
        ("webhook: {}", serde_json::json!({"kind": "webhook"})),
        (
            "schedule: {cron: '*/5 * * * *'}",
            serde_json::json!({"kind": "schedule", "cron": "*/5 * * * *"}),
        ),
        (
            "event: {type: 'order.ready'}",
            serde_json::json!({"kind": "event", "event_type": "order.ready"}),
        ),
    ] {
        let source = SOURCE.replace("when: {manual: {}}", &format!("when: {{{trigger}}}"));
        let report = report_for(&source);
        assert_eq!(report["execution_plan"]["trigger"], expected);
    }
}
