# API Compatibility Report

STATUS: PASS

- `API-001`: `cargo +nightly test -p vb_compile --all-targets --all-features` — PASS; crate-root public API use sites compiled and tests passed.
- `API-002`: `cargo +nightly test -p velvet-ballastics-workspace-tests --test integration_compile_codegen_pipeline --test integration_compile_codegen_runtime_e2e --test integration_compile_error_message_quality --test integration_validate_yaml_parsing` — PASS; selected workspace integration callers compile against crate-root `vb_compile` APIs.
- Split contract API gate: `cargo +nightly test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract` — PASS; crate-root API parity and private-module privacy checks passed.
