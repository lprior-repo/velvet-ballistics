bead_id: vb-qi37.16.4
bead_title: cli/runtime: Implement durable answer command
phase: state-5
updated_at: 2026-05-11T00:00:00Z

# Red-Phase Evidence

Red-phase tests were added to `crates/vb_runtime/src/shard/lifecycle.rs` under names beginning `red_` for durable answer behavior.

Command:

```bash
rtk cargo test --package vb_runtime --lib -- shard::lifecycle::tests::red_
```

Observed result:

```text
test result: FAILED. 0 passed; 12 failed; 0 ignored; 0 measured; 1337 filtered out; finished in 0.00s
```

Classification: expected RED phase. Failures exercise unimplemented or incomplete durable answer semantics including ask fixture compilation/hydration, secret-taint rejection/redaction, duplicate answer rejection, journal ordering, payload boundary rejection, and replay/idempotency.
