# Machine Gate Report

STATUS: PASS

- Preflight: PASS — required artifacts exist; `contract-verification-review.md` approved; proof, traceability, delivery-scope, and planned-obligation JSONL parse.
- `cargo +nightly fmt --all --check`: PASS.
- `cargo +nightly check -p vb_compile --all-targets --all-features`: PASS.
- `cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings`: PASS.
- Strict source static scan (`-D unsafe_code`, unwrap/expect/panic/todo/dbg/indexing/arithmetic/as-conversions, etc.): PASS.
- `cargo +nightly test -p vb_compile --all-targets --all-features`: PASS.
- `cargo +nightly test -p velvet-ballastics-workspace-tests --test integration_compile_codegen_pipeline --test integration_compile_codegen_runtime_e2e --test integration_compile_error_message_quality --test integration_validate_yaml_parsing`: PASS.
- `cargo +nightly test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract`: PASS, 8 passed.
- Dependency-edge scan after repair: PASS, 0 forbidden errors-to-validation / validation-to-lowering-or-core / include-body matches.
- `bash scripts/check-source-length.sh`: PASS with DEFERRED_GLOBAL notices only for pre-existing unrelated files.
- `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet`: PASS.
- `moon ci`: PASS, 23 tasks completed; nextest summary 10771 passed, 44 skipped.
- Optional `cargo +nightly miri test -p vb_compile`: DEFERRED_GLOBAL environment failure; not a blocking required gate.
