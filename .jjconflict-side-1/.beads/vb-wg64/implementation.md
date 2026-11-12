# vb-wg64 Implementation

- Repaired clean-clone `moon ci --base HEAD --head HEAD --force` failures from isolated workspace `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`.
- Fixed `xtask/src/forbidden_scan.rs` formatting, print macro lint, checked counters/line arithmetic, safe glob indexing, and `&Path` argument shape.
- Replaced CLI direct stderr prints with `Write`-based helpers and fixed `ai-context` JSON output nesting plus accepted-artifact IR decoding.
- Removed unused recovery BDD imports/locals without deleting assertions.
- Repaired additional clean-clone blockers exposed by forced `moon ci`: fuzz workspace isolation, stale fuzz target ignore, workspace test API drift, mode activation test fixtures, budget runtime limits, accepted artifact v1 gate bounds, and benchmark package name.

## Files Changed

- `.gitignore`
- `.moon/tasks/all.yml`
- `Cargo.toml`
- `crates/vb_cli/src/app_impl.rs`
- `crates/vb_cli/src/commands_ai_context.rs`
- `crates/vb_cli/src/main.rs`
- `crates/vb_cli/src/mode_error.rs`
- `crates/vb_cli/tests/cli_verify_integration.rs`
- `crates/vb_cli/tests/mode_activation_integration_tests.rs`
- `crates/vb_core/src/budget.rs`
- `crates/vb_core/src/budget/tests.rs`
- `crates/vb_runtime/src/error/equality.rs`
- `crates/vb_storage/src/admission.rs`
- `crates/vb_storage/tests/recovery_bdd_tests.rs`
- `crates/vb_ui_model/src/canonical.rs`
- `crates/vb_ui_model/src/redact.rs`
- `crates/workspace_tests/tests/fixtures/valid/minimal.yaml`
- `crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs`
- `crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs`
- `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`
- `fuzz/Cargo.toml`
- `fuzz/src/lib.rs`
- `xtask/src/forbidden_scan.rs`
