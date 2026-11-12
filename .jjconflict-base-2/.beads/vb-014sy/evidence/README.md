# vb-014sy evidence

The v1 trigger contract is `when.event.type`; see `velvet-ballistics-MASTER.md` section 9. The active schema-support validator formerly required `name`, causing the event acceptance test's legacy fixture to fail. The candidate now requires `type`, rejects legacy `name`, restores the workspace test, and updates stale unit fixtures and the AST field comment.

## Focused test commands

Raw outputs and actual process exit codes are recorded under `local-direct/`:

- `CARGO_BUILD_JOBS=2 cargo test -p vb_validate validate_trigger` — exit 0. Validator library: 13 passed, 0 failed, 857 filtered. Other package targets were also launched by Cargo; their selected counts are in the raw log.
- `CARGO_BUILD_JOBS=2 cargo test -p velvet-ballistics-workspace-tests --test vb_test_validate_yaml_parsing_behavior validate_trigger_accepts_event_trigger_with_type` — exit 0, 1 passed, 0 failed, 61 filtered.
- `CARGO_BUILD_JOBS=2 cargo test -p vb_yaml raw_yaml_event_trigger -- --test-threads=2` — exit 0, 2 passed, 0 failed, 158 filtered. The raw YAML parser API tests accept `when.event.type` and reject missing type, scalar event body, non-string type, empty type, extra event field, and unknown trigger with exact `YamlError` variants and field paths.

The test stdout files preserve their command output; one surplus trailing blank line was removed from Cargo stdout files so `git diff --check` is clean.

## Local CLI review routes

- OMP v18.12.1: `local-5090-ninfer/qwen3.8-27b-uncensored` ran without a custom profile and returned the assigned workspace path plus the correct section 9 grammar. The retained output has no separate tool-call event record, so bash invocation is unverified and this is partial review evidence only. A later invocation remained at `Working...` and was interrupted (exit 130).
- A custom OMP profile `vb014sy-yaml-review` did not contain the selected model route and returned exit 1; no credentials or configuration were inspected.
- OpenCode's catalog listed local Qwen models, but invocations to `qwen36-5090/Qwen3.6-35B-A3B-UD-Q5_K_XL` (with and without `.gguf`) and `qwen36-3090/Qwen3.6-35B-A3B-UD-Q4_K_XL` all failed before a tool call. The `.gguf` route returned model-not-found; the other routes returned unexpected server errors. OpenCode review is not evidence of acceptance.

The first independent review rejected the initial candidate because it had no raw YAML parser API coverage and the empty-event diagnostic pointed to `when.event.name`. This revision addresses those findings. The revised candidate still requires independent code and test review.
