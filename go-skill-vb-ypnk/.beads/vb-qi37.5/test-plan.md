# State 7 Test Plan

STATUS: APPROVED

Scope: verify compile/validate idempotency decision parity, runtime idempotency-key checks, duplicate/stale replay model, and rejected side-effecting non-idempotent contracts.

Required tests:
- `cargo test -p vb_compile --test idempotency_parity`: all 45 combinations agree between public compile gate and static validator.
- `cargo test -p vb_validate`: validator decision table and diagnostics remain deterministic.
- `cargo test -p vb_core`: runtime idempotency key checks remain deterministic.

Formal companions:
- Kani full decision parity and runtime key harnesses.
- TLA duplicate/stale replay model.
- Verus decision, certificate summary, and replay tracker proofs.
