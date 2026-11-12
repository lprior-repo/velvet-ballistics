# Lean Contract - vb-qi37.5.3

Lean/theorem-kernel lane not required. The proof obligation is finite and executable in Rust:

```text
accepted(artifact) => artifact.verification.idempotency_verified
accepted(artifact) => keyed(artifact) subset attested(artifact)
admit_artifact_run(artifact) == Ok(run) => run.idempotency_attested == artifact.verification.idempotency_attested
```
