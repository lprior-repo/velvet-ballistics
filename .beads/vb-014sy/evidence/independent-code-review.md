# Independent code review: vb-014sy candidate 505be4ef

**Verdict: REJECTED**

Reviewed branch: `codex/vb-014sy-yaml-cli-candidate` at `505be4ef5d239ea7f721602575176033581bda84`.

No production or test source files were edited in the review workspace.

## Contract assessment

`velvet-ballistics-MASTER.md` section 9 specifies `when.event.type`. The active YAML AST parser in `crates/vb_yaml/src/yaml_ast/parse_trigger.rs` reads `type`, rejects unrecognized event keys, requires a string value, and rejects an empty value. The candidate's fixture changes from `name` to `type` are correct.

## Findings

1. **MEDIUM — stale empty-value diagnostic path.** In `crates/vb_yaml/src/yaml_ast/parse_trigger.rs:87-90`, an empty event type reports `when.event.name`, although the grammar and missing-field diagnostic use `when.event.type`. Correct the path and cover the empty value through the public YAML parse path.
2. **MEDIUM — restored test does not exercise YAML parsing.** `crates/workspace_tests/tests/vb_test_validate_yaml_parsing_behavior.rs:756-770` constructs `FieldValue`/`WorkflowDoc` manually and calls `vb_validate::validate_trigger`. It does not submit YAML to the parser. The candidate adds no raw-YAML negative coverage for missing `type`, scalar body/value, empty type, extra event fields, or unknown trigger kinds.
3. **HIGH — existing compiler digest test fails on the candidate.** `CARGO_BUILD_JOBS=2 cargo test -p vb_compile --features test-util trigger_event_type_changes_compiled_digest -- --nocapture` exits 101 because event types `push` and `pull_request` produce identical `WorkflowDigest` values at `crates/vb_compile/tests/finish_digest_integration.rs:527`. The test file and digest implementation are unchanged by this candidate; this finding records an existing failure and does not attribute it to the patch. Without `--features test-util`, the package test build fails earlier because `WorkflowSourceParts` is unavailable in `tests/common/mod.rs`.
4. **LOW — evidence stdout whitespace.** `git diff --check 856603b8 505be4ef` reports an added blank line at EOF in `.beads/vb-014sy/evidence/local-direct/vb-validate-trigger.stdout.txt:69` and `.beads/vb-014sy/evidence/local-direct/workspace-event-trigger.stdout.txt:12`.

## Verification

- `CARGO_BUILD_JOBS=2 cargo test -p vb_validate validate_trigger -- --nocapture`: exit 0; 13 validator unit tests passed and the selected proptest passed.
- `CARGO_BUILD_JOBS=2 cargo test -p velvet-ballistics-workspace-tests --test vb_test_validate_yaml_parsing_behavior validate_trigger_accepts_event_trigger_with_name -- --nocapture`: exit 0; 1 test passed.
- `moon run :lint-src`: exit 0. Moon emitted remote-cache upload warnings.
- Exact-file `rustfmt --check --edition 2024` on changed Rust files: exit 0.
- `git diff --check`: the two evidence-file EOF whitespace findings above.

Independent approval is not granted.
