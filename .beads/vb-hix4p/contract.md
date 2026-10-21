```json
{
  "fixtures": [
    {
      "file": "crates/workspace_tests/tests/vb_test_validate_policy_enforce_behavior.rs",
      "helper": "make_parts",
      "input_slots": "Box::new([])",
      "reason": "Policy validation fixtures construct WorkflowParts without input_slots; no external input is used by these policy tests."
    },
    {
      "file": "crates/vb_ipc/src/server/trace.rs",
      "helper": "chain_workflow",
      "input_slots": "Box::new([])",
      "reason": "IPC trace test chains ten Nop nodes then Finish(slot0); no external input is consumed."
    }
  ],
  "preserve": [
    "All existing fixture data (node types, edge connections, slot assignments)",
    "All existing assertions and expected outcomes",
    "Legacy test code structure and naming"
  ],
  "tests": [
    "cargo test -p velvet-ballistics-workspace-tests --test vb_test_validate_policy_enforce_behavior",
    "cargo test -p vb_ipc --lib"
  ]
}
```
Rejected v2 contract invented a different repository and nonexistent trace path. It was not applied. Final v3 contract names exact actual files and test commands.
