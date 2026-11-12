bead_id: vb-qi37.13.4
phase: State 15 preflight
updated_at: 2026-05-11T00:00:00Z

# Last-Mile Acceptance Decision

STATUS: LANDING_BLOCKED

## Decision

The prior bead-local YAML blocker has been repaired and verified. Do not close yet: State 15 integration is blocked by sibling CLI workspace overlap that requires an explicit integration bead/merge pass.

## Prior Blocker Resolution

Previous classification: BLOCK_LOCAL
Current classification: repaired; remaining machine-gate debt is DEFERRED_GLOBAL.

The master CLI output contract promises canonical structured YAML for `--emit yaml`:

- `velvet-ballistics-MASTER.md` says `--emit yaml` is canonical structured text for v1 and JSON may be a later separate adapter.
- It also says machine-readable output via `--emit yaml` is mandatory for reporting commands.

The implementation now emits YAML-shaped status output for `status --emit yaml` and the black-box test asserts it does not start with `{`, includes canonical YAML keys, and parses through `serde_saphyr`.

## Repair Evidence

```text
cargo +nightly fmt -p velvet_ballistics --check -> exit 0
rtk cargo test -p velvet_ballistics --test cli_integration cli_emit_yaml_contract_is_not_silent_when_master_emit_mode_is_requested -> 1 passed, 77 filtered out
rtk cargo test -p velvet_ballistics --test cli_integration cli_help_is_bounded_and_non_interactive -> 1 passed, 77 filtered out
rtk cargo test -p velvet_ballistics --test cli_integration cli_status_json_writes_payload_to_stdout_only -> 1 passed, 77 filtered out
rtk cargo test -p velvet_ballistics --test cli_integration cli_unknown_command_returns_stderr_diagnostic_without_stack_trace -> 1 passed, 77 filtered out
rtk cargo check -p velvet_ballistics --all-targets -> 0 errors, 1 duplicate-package warning
rtk cargo run -p velvet_ballistics --bin vb -- status --emit yaml -> YAML output beginning with schema_version: velvet-ballistics/cli-output/v1
```

## Non-blocking debt

The `moon ci` missing `main` revision failure remains DEFERRED_GLOBAL. Latest `moon ci` also reports unrelated repo-wide `FORMAT`/`lint-src` debt in `crates/vb_proof_kernels`, `crates/vb_storage`, `fuzz`, and `xtask`; see `moon-report.md` and `regression-diff.md`.

## Landing blocker

Classification: LANDING_BLOCKED

Sibling workspaces `vb-qi37.13.4`, `vb-qi37.15.1`, and `vb-qi37.15.2` all append independent test modules at EOF in `crates/velvet_ballistics/tests/cli_integration.rs` from the same parent `qwxtlxqq 5fb2d246`. Sequential integration is expected to conflict at that append site. Overlapping source file:

- `crates/velvet_ballistics/tests/cli_integration.rs`

Additional overlapping file touched by all three but with non-adjacent command-handler hunks:

- `crates/velvet_ballistics/src/main.rs`

Keep workspace `/home/lewis/src/Velvet-ballistics-vb-qi37-13-4-go`; do not close/forget until an integration pass combines the sibling CLI test modules and reruns gates.

## 2026-05-11 integration retry

Integration workspace `/home/lewis/src/Velvet-ballistics-vb-8iwj-wave3-integration` combines this change with `vb-qi37.15.1` and `vb-qi37.15.2` as merge parents. The `cli_integration.rs` 3-sided conflict was manually resolved by preserving all sibling test modules. Scoped evidence:

- `cargo +nightly fmt -p velvet_ballistics --check`: pass.
- `rtk cargo check -p velvet_ballistics --all-targets`: 0 errors, 1 duplicate-package warning.
- `cli_emit_yaml_contract_is_not_silent_when_master_emit_mode_is_requested`: 1 passed, 85 filtered out.
- final manual QA `status --emit yaml`: PASS, real YAML starts with `schema_version: velvet-ballistics/cli-output/v1`.

Still not closed: source not landed to canonical remote/main, no safe push/bookmark policy was provided, and original workspace remains intentionally retained.

## 2026-05-11 additional State 15 preflight

The original integration directory was absent on resume; a new non-landing preflight workspace was created from `tqypyqys 57f44923`:

```text
/home/lewis/src/Velvet-ballistics-vb-8iwj-wave3-preflight
zmryxnnv e3b5bb45 (empty) vb-8iwj: run wave 3 landing preflight
```

Additional evidence:

- `moon run :quick`: PASS.
- `moon run :test`: first 300s attempt timed out; 600s retry PASS, 9863 tests passed.
- `moon ci`: completed non-zero; failures match `vb-w823` global debt (`vb_proof_kernels`, `vb_storage`, `fuzz`, `xtask` fmt/lint), classified `DEFERRED_GLOBAL`.

Still not closed: no approved landing policy; original workspace `/home/lewis/src/Velvet-ballistics-vb-qi37-13-4-go` remains retained.
