#![forbid(unsafe_code)]

mod support {
    pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
}
#[path = "events_filters/wire.rs"]
mod wire;

use serde_json::{Value, json};
use std::process::Command;
use support::TestResult;

const TABLE: &str = concat!(
    "id\tidempotency\tretry_safety\tside_effect\tinput_slots\toutput_slots\ttimeout_ms\tname\n",
    "1\tdeterministic_pure\tidempotent\tpure\t1\t1\t1000\tnoop\n",
    "2\tidempotent_external\trequires_idempotency_key\texternal_write\t2\t1\t5000\twrite\n",
    "3\tat_least_once_external\tnot_retry_safe\texternal_write\t1\t0\t10000\tsend\n"
);
const WRITE_TEXT: &str = concat!(
    "action 2\n  name: write\n",
    "  input_slot_count: 2\n  output_slot_count: 1\n",
    "  max_input_bytes: 65536\n  max_output_bytes: 65536\n",
    "  timeout_ms: 5000\n  idempotency: idempotent_external\n",
    "  retry_safety: requires_idempotency_key\n  side_effect: external_write\n",
    "  idempotency_rule: external retries require a stable idempotency key\n",
    "  required_capabilities: \n",
    "  failure_codes: rejected,timeout,rate_limited,resource_exhausted,external_unavailable,invalid_input,permission_denied,conflict,unknown\n",
    "  example_input_schema: postcard(ActionInput::new(run, step, action, input, encoded_payload_bytes, contract, ticket))\n",
    "  example_output_schema: postcard(ActionOutcome::Ready|Suspended|Failed)\n"
);

fn registered_actions() -> [(Value, &'static str); 3] {
    [
        (
            json!({"id":1,"name":"noop","idempotency":"deterministic_pure","retry_safety":"idempotent","side_effect":"pure","input_slot_count":1,"output_slot_count":1,"timeout_ms":1000}),
            "pure deterministic actions may replay without an external key",
        ),
        (
            json!({"id":2,"name":"write","idempotency":"idempotent_external","retry_safety":"requires_idempotency_key","side_effect":"external_write","input_slot_count":2,"output_slot_count":1,"timeout_ms":5000}),
            "external retries require a stable idempotency key",
        ),
        (
            json!({"id":3,"name":"send","idempotency":"at_least_once_external","retry_safety":"not_retry_safe","side_effect":"external_write","input_slot_count":1,"output_slot_count":0,"timeout_ms":10000}),
            "actions declared not_retry_safe must not be retried automatically",
        ),
    ]
}

fn run(args: &[&str]) -> TestResult<Vec<u8>> {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(args)
        .output()?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "stderr={:?}", output.stderr);
    Ok(output.stdout)
}

fn inspection(row: &Value, rule: &str) -> TestResult<Value> {
    let mut fields = row.as_object().ok_or("expected action record")?.clone();
    let detail = json!({
        "max_input_bytes":65536,"max_output_bytes":65536,
        "required_capabilities":[],
        "failure_codes":["rejected","timeout","rate_limited","resource_exhausted","external_unavailable","invalid_input","permission_denied","conflict","unknown"],
        "idempotency_rule":rule,
        "example_input_schema":"postcard(ActionInput::new(run, step, action, input, encoded_payload_bytes, contract, ticket))",
        "example_output_schema":"postcard(ActionOutcome::Ready|Suspended|Failed)"
    });
    fields.extend(detail.as_object().ok_or("expected detail record")?.clone());
    Ok(json!({"success":true,"action":fields}))
}

fn assert_machine_discovery(emit: &str) -> TestResult {
    let rows: Vec<_> = registered_actions()
        .into_iter()
        .map(|(row, _)| row)
        .collect();
    assert_eq!(
        wire::decode(&run(&["action", "list", "--emit", emit])?, emit)?,
        json!({"success":true,"actions":rows})
    );
    registered_actions()
        .into_iter()
        .try_for_each(|(row, rule)| -> TestResult {
            let name = row
                .get("name")
                .and_then(Value::as_str)
                .ok_or("expected name")?;
            assert_eq!(
                wire::decode(&run(&["action", "inspect", name, "--emit", emit])?, emit)?,
                inspection(&row, rule)?
            );
            Ok(())
        })
}

#[test]
fn yaml_discovery_retains_complete_named_registry_records() -> TestResult {
    assert_machine_discovery("yaml")
}

#[test]
fn postcard_discovery_retains_complete_named_registry_records() -> TestResult {
    assert_machine_discovery("postcard")
}

#[test]
fn text_discovery_retains_old_fields_and_registered_names() -> TestResult {
    assert_eq!(String::from_utf8(run(&["action", "list"])?)?, TABLE);
    for (id, name) in [(1, "noop"), (2, "write"), (3, "send")] {
        let text = String::from_utf8(run(&["action", "inspect", name])?)?;
        assert!(text.starts_with(&format!("action {id}\n  name: {name}\n")));
    }
    assert_eq!(
        String::from_utf8(run(&["action", "inspect", "write"])?)?,
        WRITE_TEXT
    );
    Ok(())
}
