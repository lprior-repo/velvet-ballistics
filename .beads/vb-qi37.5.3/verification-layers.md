# Verification Layers - vb-qi37.5.3

- Unit tests: storage proof metadata derivation/rejection and runtime admission rejection/carry behavior.
- Source clippy: `rtk cargo clippy -p vb_runtime -p vb_storage --lib -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used`.
- Kani: `rtk cargo kani -p vb_compile --harness idempotency_gate_parity` for all-45 idempotency decision parity.
- Full test-target clippy: attempted, classified `DEFERRED_GLOBAL` due pre-existing test lint debt outside touched source files.
