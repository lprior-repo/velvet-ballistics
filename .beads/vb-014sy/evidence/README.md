# vb-014sy evidence

The v1 trigger contract is `when.event.type`; see `velvet-ballistics-MASTER.md` section 9. The active schema-support validator formerly required `name`, causing the event acceptance test's legacy fixture to fail. The candidate now requires `type`, rejects legacy `name`, restores the workspace test, and updates stale unit fixtures and the AST field comment.

## Focused test commands

Raw outputs and actual process exit codes are recorded under `local-direct/`:

- `CARGO_BUILD_JOBS=2 cargo test -p vb_validate validate_trigger` — exit 0. Validator library: 13 passed, 0 failed, 857 filtered. Other package targets were also launched by Cargo; their selected counts are in the raw log.
- `CARGO_BUILD_JOBS=2 cargo test -p velvet-ballistics-workspace-tests --test vb_test_validate_yaml_parsing_behavior validate_trigger_accepts_event_trigger_with_name` — exit 0, 1 passed, 0 failed, 61 filtered.

## Local CLI review routes

- OMP v18.12.1: `local-5090-ninfer/qwen3.8-27b-uncensored` ran without a custom profile. It reported the assigned workspace path and accurately returned the section 9 grammar. The initial invocation's output is in the task transcript; a second captured invocation was interrupted after remaining at `Working...` and exited 130.
- A custom OMP profile `vb014sy-yaml-review` did not contain the selected model route and returned exit 1; no credentials or configuration were inspected.
- OpenCode's catalog listed local Qwen models, but review invocations to both `qwen36-5090/Qwen3.6-35B-A3B-UD-Q5_K_XL` (with and without `.gguf`) and `qwen36-3090/Qwen3.6-35B-A3B-UD-Q4_K_XL` failed before a tool call. The 5090 route returned model-not-found / then an unexpected server error; the 3090 route returned an unexpected server error. OpenCode review is therefore not evidence of acceptance.

No independent code-review or test-review approval is claimed here; this candidate is published for that review.
