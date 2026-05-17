bead_id: vb-qi37.16.3
phase: state-7
status: PASS

# Hands-on Smoke Evidence

Commands:

```bash
rtk cargo test -p vb_runtime --test durable_retry_red_phase
rtk cargo test -p vb_runtime --lib
rtk cargo test -p vb_runtime --test '*'
```

Observed:

```text
cargo test: 9 passed (1 suite, 0.01s)
cargo test: 1337 passed (1 suite, 0.28s)
cargo test: 18 passed (2 suites, 0.00s)
```

Manual conclusion: durable retry POST-005 evidence now exercises `ticket_with_retry_capacity` and passes focused runtime smoke.
