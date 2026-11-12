bead_id: vb-qi37.16.2
bead_title: cli/runtime durable resume transition
phase: state-13
updated_at: 2026-05-11T00:00:00Z

STATUS: APPROVED

## Scope

Reviewed the durable resume transition after State 12 approval in isolated workspace
`/home/lewis/src/Velvet-ballistics-vb-qi37-16-2-go`. The source checkout
`/home/lewis/src/Velvet-ballistics` was not used.

## Commands and outcomes

- `pwd -P && test "$(pwd -P)" = "/home/lewis/src/Velvet-ballistics-vb-qi37-16-2-go" && case "$(pwd -P)" in "/home/lewis/src/Velvet-ballistics" |"/home/lewis/src/Velvet-ballistics"/*) exit 77; ; esac && test -d ".beads/vb-qi37.16.2" && rtk ls ".beads/vb-qi37.16.2"` — PASS; workspace guard proved the active path is the isolated sibling and listed State 1-12 artifacts.
- `python3 ... State12 status/JSONL verifier` — PASS; `contract-verification-review.md`, `formal-verification-report.md`, and `verus-report.md` have approved status lines; `verification-ledger.jsonl` parsed with 13 records: 12 PASS and 1 WAIVED; `proof-obligations.jsonl` parsed with 13 records; `delivery-scope.jsonl` parsed with 1 record.
- `python3 ... all Rust file line count scan` — OBSERVED; repo contains 311 Rust files over 300 lines. This is established global architectural debt and not introduced by this State 13 review.
- `python3 ... scoped changed file line counts` — OBSERVED; scoped production/test files include pre-existing large modules (`args.rs`, `journal.rs`, `shard/lifecycle.rs`, `cli_integration.rs`) plus new test file `crates/vb_runtime/tests/durable_resume_red_phase.rs` at 721 lines. No State 13 code edits were made.
- `jj diff --stat` and `jj diff --git crates/vb_runtime/src/shard/types.rs crates/vb_runtime/src/shard/lifecycle.rs crates/vb_runtime/src/journal.rs xtask/src/main.rs xtask/src/proof.rs` — REVIEWED; durable-resume changes are explicit state transition/data changes, not hidden runtime side effects.
- scoped forbidden-construct scan — OBSERVED; no new production `unsafe`; test code has `unwrap`/assertions consistent with existing test-suite style and already covered by prior State 8-12 gates.

## DDD / architecture review

- Domain states are explicit: `RuntimeState::{Initial, Running, Resumable, Resuming, Failed}` and `RuntimeState::is_resumable` encode valid resume state instead of stringly flags.
- Resume API has typed outcome/error surfaces: `ResumeResult`, `ResumeStatus`, and `ResumeError` preserve illegal-state checks and fail-closed error taxonomy.
- Workflow is modeled as named state transitions: `handle_resume` validates existence/state, appends journal evidence, drives the run, and lets `apply_drive_result` own the post-drive lifecycle state.
- Journal evidence is append-only via the new `RuntimeJournalEvent::Resumed { run, timestamp }` event and is handled by existing journal projection helpers without reordering/deletion semantics.
- CLI/proof xtask additions stay at the shell/tooling boundary and do not introduce JSON/YAML/HTTP into the runtime core.

## Decision

No State 13 code changes were made. The bead can advance directly to State 14. Existing repo-wide line-count debt remains outside this bead-local approval and should be handled by separate architectural debt beads, not by this durable-resume landing gate.
