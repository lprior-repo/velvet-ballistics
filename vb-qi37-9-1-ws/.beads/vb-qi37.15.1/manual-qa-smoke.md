bead_id: vb-qi37.15.1
bead_title: cli: Add simulate command
phase: State 7
updated_at: 2026-05-11T00:00:00Z

STATUS: PASS

# Manual QA Smoke

Command: `rtk cargo run -p velvet_ballastics --bin vb -- simulate /tmp/tmp.FtR48HRB6h/workflow.yaml --json`

Verbatim output excerpt:

```text
Running `target/debug/vb simulate /tmp/tmp.FtR48HRB6h/workflow.yaml --json`
{
  "kind": "simulate",
  "schema_version": "velvet-ballastics/v1",
  "success": true,
  "total_actions": 0,
  "total_branches": 0,
  "total_steps": 2,
  "trace": [
    { "description": "Set constant value", "kind": "set_const", "step": 0 },
    { "description": "Finish -- would complete run", "kind": "finish", "step": 1 }
  ]
}
```

Decision: PASS.
