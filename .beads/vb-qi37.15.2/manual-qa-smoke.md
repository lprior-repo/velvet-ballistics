bead_id: vb-qi37.15.2
bead_title: cli: Add submit command and job ledger
phase: State 7
updated_at: 2026-05-11T00:00:00Z

STATUS: PASS

# Manual QA Smoke

Command: `rtk cargo run -p velvet_ballastics --bin vb -- submit /tmp/tmp.2Da1hBOuR6/workflow.yaml --input-bin /tmp/tmp.2Da1hBOuR6/input.bin --db /tmp/tmp.2Da1hBOuR6/db --durability journaled --json`

Verbatim output excerpt:

```text
Running `target/debug/vb submit /tmp/tmp.2Da1hBOuR6/workflow.yaml --input-bin /tmp/tmp.2Da1hBOuR6/input.bin --db /tmp/tmp.2Da1hBOuR6/db --durability journaled --json`
{
  "digest": "44c82610230607e6290d48132be5dd3437b9faddcdb3bce09ba9d27608590519",
  "run_id": 1778504434198101414,
  "status": "submitted",
  "step_count": 2
}
```

Decision: PASS. Submit no longer fails with Fjall lock in manual smoke.
