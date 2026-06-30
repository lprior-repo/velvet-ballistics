# Verification Layers: vb-m5gp

## Boundary

- Verus-owned kernel: waived for pure structural extraction; no new pure algorithm is specified.
- TLA+ temporal model: waived; no temporal behavior exists.
- Theorem projection: waived; no tiny theorem kernel exists.
- Runtime shell: `vb_compile` public facade and private module implementation after split.
- External systems excluded from formal proof: Cargo/Moon tooling internals and generated Rust compiler internals beyond existing tests.

## Required Layer Assignment

- PRE-001 -> static-scan workspace path check and state evidence.
- PRE-002 -> static-scan Cargo/config diff check.
- PRE-003 -> source-structure review: active code moved; stale scaffolding not wired without parity proof.
- PRE-004, POST-002, INV-001 -> API compatibility via crate and downstream compile/test use sites.
- POST-001, INV-002 -> source-structure/static scan for private modules and facade re-exports.
- POST-003, INV-004 -> `vb_compile` tests, workspace compile integration tests, fuzz target compile, Kani idempotency parity when available.
- POST-004, ERR-001, ERR-002, ERR-003, INV-005 -> `vb_compile` diagnostic tests and compile error message integration tests.
- POST-005 -> source scan proving no new public internal module declarations.
- POST-006 -> source-length governance check for `lib.rs` and new modules.
- INV-003 -> source review proving validation moved without fork and stale scaffolding remains unwired or parity-proven.
- INV-006 -> static scan/clippy for forbidden constructs.
- INV-007 -> review of visibility changes and test movement.

## Exact Evidence Commands Known From Explore

- Format: `cargo +nightly fmt --all --check`
- Clippy: `cargo +nightly clippy -p vb_compile --all-targets --all-features -- -D warnings`
- Crate tests: `cargo +nightly test -p vb_compile --all-targets --all-features`
- Workspace compile integrations: `cargo +nightly test -p workspace_tests --test integration_compile_codegen_pipeline --test integration_compile_codegen_runtime_e2e --test integration_compile_error_message_quality --test integration_validate_yaml_parsing`
- Miri if budget/toolchain allows: `cargo +nightly miri test -p vb_compile`
- Kani if available: exact harness path from explore is `kani/idempotency_gate_parity.rs`; command is blocked until State 4 discovers the repository's supported Kani invocation.
- Canonical rollup: `moon ci`

## Waivers

- TLA-WAIVER-001: no temporal model; compensated by structural/API/behavior checks.
- THM-WAIVER-001: no Lean/Aeneas/Hax kernel; compensated by parity checks.
- VERUS-WAIVER-001: no new pure invariant or algorithm; if implementation changes semantics, rerun State 3 and add Verus/Kani obligations.

## Review Gate

An independent contract verification review must approve these artifacts before State 4 proof planning consumes them. This State 3 artifact does not approve itself.
