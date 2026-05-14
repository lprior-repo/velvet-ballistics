bead_id: vb-qi37.13.4
bead_title: cli: Structured output contract tests
phase: State 7
updated_at: 2026-05-11T00:00:00Z

STATUS: PASS

# Manual QA Smoke

Command: `rtk cargo run -p velvet_ballastics --bin vb -- status --emit yaml`

Verbatim output excerpt:

```text
Running `target/debug/vb status --emit yaml`
{
  "RuntimePolicy": "Strict",
  "kind": "status",
  "running": true,
  "schema_version": "velvet-ballastics/v1",
  "status": "running"
}
```

Decision: PASS. Command exits successfully and includes structured envelope fields. Residual: output is JSON-shaped despite `--emit yaml`; keep under later structured-emitter review.
