bead_id: vb-qi37.13.4
phase: State 8

# Moon Report

Canonical command: `moon ci`
Result: non-zero / timed out after 120s in orchestrator shell, with failures already emitted.
Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e177ddb3c001x02cGpiJznbr5o`

Primary emitted failures:
```text
velvet-ballastics:fmt | Diff in crates/vb_proof_kernels/src/taint.rs
velvet-ballastics:fmt | Diff in crates/vb_storage/src/codec_miri_tests.rs
velvet-ballastics:fmt | Diff in crates/vb_storage/src/kani_codec.rs
velvet-ballastics:fmt | Diff in crates/vb_storage/src/lib.rs
velvet-ballastics:fmt | Diff in fuzz/fuzz_targets/decode_record.rs
velvet-ballastics:fmt | Diff in xtask/src/main.rs
velvet-ballastics:fmt | Diff in xtask/src/proof.rs
velvet-ballastics:lint-src | error: you should consider adding a `Default` implementation for `EnvelopeHeader`
velvet-ballastics:lint-src | --> crates/vb_proof_kernels/src/envelope_header.rs:26:5
```

Post-repair bead-local command evidence:
```text
cargo +nightly fmt -p velvet_ballastics --check -> exit 0
rtk cargo test -p velvet_ballastics --test cli_integration cli_emit_yaml_contract_is_not_silent_when_master_emit_mode_is_requested -> 1 passed, 77 filtered out
rtk cargo test -p velvet_ballastics --test cli_integration cli_help_is_bounded_and_non_interactive -> 1 passed, 77 filtered out
rtk cargo test -p velvet_ballastics --test cli_integration cli_status_json_writes_payload_to_stdout_only -> 1 passed, 77 filtered out
rtk cargo test -p velvet_ballastics --test cli_integration cli_unknown_command_returns_stderr_diagnostic_without_stack_trace -> 1 passed, 77 filtered out
rtk cargo check -p velvet_ballastics --all-targets -> 0 errors, 1 duplicate-package warning
```

Note: one invalid grouped `cargo test` invocation occurred during orchestration (`unexpected argument`); it did not test product behavior and was replaced by the four single-test invocations above.
