# Test Report

STATUS: PASS

- `cargo +nightly test -p vb_compile --all-targets --all-features`: PASS — 245 lib tests, 9 idempotency tests, 15 primitive-lowering tests, 10 strict-yaml tests.
- `cargo +nightly test -p velvet-ballastics-workspace-tests --test integration_compile_codegen_pipeline --test integration_compile_codegen_runtime_e2e --test integration_compile_error_message_quality --test integration_validate_yaml_parsing`: PASS — 15, 23, 21 passed/4 ignored, and 29 tests respectively.
- `cargo +nightly test -p velvet-ballastics-workspace-tests --test integration_compile_error_message_quality`: PASS — 21 passed, 4 ignored.
- `cargo +nightly test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract`: PASS — 8 passed, including dependency-edge and recursive source-length tests.
- `moon ci`: PASS — 23 tasks completed; nextest summary 10771 passed, 44 skipped.
