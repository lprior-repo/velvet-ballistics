---
section: 42
title: "Compile-Fail Contract"
parent: velvet-ballistics-MASTER.md
---

## 42. Compile-Fail Contract

`trybuild` compile-fail tests are product law for the SDK.

Required fixtures:

```text
retry_external_write_without_key_fails.rs
retry_unknown_action_fails.rs
retry_not_retry_safe_process_fails.rs
key_uses_secret_fails.rs
key_uses_attempt_number_fails.rs
key_uses_time_fails.rs
key_wrong_scope_fails.rs
external_write_without_timeout_fails.rs
ask_without_timeout_fails.rs
wait_without_timeout_fails.rs
for_each_without_bound_fails.rs
collect_without_bound_fails.rs
arbitrary_rust_loop_inside_workflow_fails.rs
std_net_inside_workflow_fails.rs
unknown_macro_inside_workflow_fails.rs
run_yaml_api_does_not_exist.rs
submit_requires_accepted_artifact.rs
required_capabilities_cannot_be_grants.rs
raw_ir_submit_rejected.rs
policy_name_without_digest_fails.rs
```

Each compile-fail fixture must assert the diagnostic code in stderr.

---

