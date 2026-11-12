# Codec Closure Evidence Summary — 2026-07-09

Scope: RecordKind/schema-1/trailing-byte codec parity closure bundle

Shared raw evidence store: `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/`.

## Local code/test changes in this continuation

- Repaired `crates/workspace_tests/tests/postcard_envelope_wire_tests.rs` so `po_3t44_022_step_succeeded_roundtrip` encodes `JournalEvent::StepSucceeded` under `RecordKind::StepSucceeded` instead of stale `RecordKind::SlotWritten`.
- Removed generated proptest regression seed files produced by failed global test attempts; no generated regression seeds are intentional evidence artifacts.
- Prior worktree code changes pre-existed this continuation and remain broad across `crates/vb_storage`, `fuzz`, and `velvet-ballistics-MASTER.md`; see `jj status`/`jj diff --stat` evidence from the session transcript.

## Passing targeted evidence

- `cargo test -p vb_storage --lib codec -- --nocapture` → exit 0; 175 passed.
  - Raw: `cargo-test-vb-storage-lib-codec.*`
- `cargo test -p vb_storage --lib schema_one -- --nocapture` → exit 0; 10 passed.
  - Raw: `cargo-test-vb-storage-schema-one.*`
- `cargo test -p vb_storage --lib trailing -- --nocapture` → exit 0; 8 passed.
  - Raw: `cargo-test-vb-storage-trailing.*`
- `cargo test -p vb_storage --lib adversarial -- --nocapture` → exit 0; 64 passed.
  - Raw: `cargo-test-vb-storage-adversarial.*`
- `cargo test -p vb_ipc --lib -- --nocapture` → exit 0; 531 passed.
  - Raw: `cargo-test-vb-ipc-lib.*`
- `cargo test -p velvet-ballistics-workspace-tests --test postcard_envelope_wire_tests -- --nocapture` failed once on stale test parity, then reran after repair → exit 0; 22 passed.
  - Failing raw: `cargo-test-workspace-postcard-envelope-wire.*`
  - Passing raw: `cargo-test-workspace-postcard-envelope-wire-rerun.*`
- `cargo fmt --check` and rerun → exit 0.
  - Raw: `cargo-fmt-check.*`, `cargo-fmt-check-rerun.*`
- `moon run :fmt` and rerun → exit 0.
  - Raw: `moon-run-fmt.*`, `moon-run-fmt-rerun.*`
- `moon run :check` and rerun → exit 0.
  - Raw: `moon-run-check.*`, `moon-run-check-rerun.*`
- `moon run :lint-src` → exit 0.
  - Raw: `moon-run-lint-src.*`
- `cargo fuzz list` → exit 0; `vb_storage_codec` listed.
  - Raw: `cargo-fuzz-list.*`
- `cargo fuzz build vb_storage_codec --target x86_64-unknown-linux-gnu` → exit 0.
  - Raw: `cargo-fuzz-build-vb-storage-codec.*`
- Direct `vb_storage_codec` libFuzzer smoke with copied temp corpus and `-max_total_time=1` → exit 0; 76,915 runs in 2 seconds, no crash.
  - Raw: `fuzz-smoke-vb-storage-codec.*`
- Kani positive slices already captured in shared raw logs:
  - `kani_record_kind` → exit 0; raw `kani-record-kind.*`.
  - Decode-order individual harnesses `bad_magic`, `bad_version`, `unknown_kind`, `family_mismatch`, `bad_header_len`, `payload_too_large` → exit 0; raw `kani-decode-order-*.*`.
  - `kani_typed_partitioned_ids::vb_eepg_unknown_record_kind_error_contract` → exit 0; raw `kani-typed-ids-unknown_record_kind.*`.
  - `kani_typed_partitioned_ids::vb_eepg_record_kind_contracts` → exit 0; raw `kani-typed-ids-record_kind_contracts.*`.

## Failing or blocked evidence

- `moon run :test` remains blocked by unrelated/global `vb_compile` failures around choose body/otherwise/fallthrough/width and repeat digest tests.
  - Latest raw: `moon-run-test-rerun.*`; exit 1; summary 1,616 passed, 9 failed, 40 skipped, 12,438 not run.
- `cargo test -p velvet-ballistics-workspace-tests -- --nocapture` remains blocked outside the repaired postcard lane:
  - `wal_crash_helper` standalone helper build cannot resolve crates when compiled directly.
  - `integration_validate_yaml_parsing` has two failing assertions (`compile_rejects_invalid_version_string`, `compile_rejects_wrong_version_prefix`).
  - Raw: `cargo-test-workspace-tests.*`; exit 101.
- `moon run :source-length` failed through `test-integrity` dependency:
  - `WeakenedAssertion|crates/vb_storage/src/tests.rs|removed_exact=45 added_exact=12 added_weak=0`
  - `WeakenedAssertion|fuzz/fuzz_targets/kind_validation.rs|removed_exact=4 added_exact=0 added_weak=0`
  - Raw: `moon-run-source-length.*`; exit 1.
- Direct `bash scripts/check-source-length.sh` failed:
  - `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs` hot function has 29 logical lines (limit 25).
  - Duplicate exception for `crates/vb_core/src/frame/parts/kani_f1_exhaustive.rs`.
  - `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` has 529 physical lines (hard limit 300).
  - Raw: `script-check-source-length.*`; exit 1.
- `moon ci` was attempted and timed out at the shell-tool 1,200,000 ms limit.
  - Raw: `moon-ci.stdout.txt`, `moon-ci.stderr.txt`, `moon-ci.exitcode`, `moon-ci-timeout-note.md`.
  - Before timeout, logs show `test-integrity` fail, `benchmark-proof` `--save-baseline` option error, `benchmark-regression-policy` stale evidence, global `vb_compile` test failures, and a long `kani-baseline` lane.
- Kani blockers already captured:
  - Aggregate decode-order command timed out (`kani-decode-order.*`, exit 124).
  - `kani_vb_u8gi_storage_numeric_fields::vb_u8gi_storage_numeric_fields_arbitrary` timed out (`kani-numeric-fields.*`, exit 124).
  - Aggregate typed partitioned IDs commands timed out (`kani-typed-ids.*`, `kani-typed-ids-typed_ids*.*`, exit 124 where recorded).
  - Legacy postcard-wire harnesses exit 1/undetermined; do not claim proof closure from them.

## Performance layer

No performance optimization claim is made. Fuzz execution count is correctness/adversarial smoke evidence only, not a throughput benchmark. No profiler or benchmark evidence is attached.

## Second-ring evidence

No zero-cost abstraction, vectorization, bounds-check-removal, public API compatibility, or release-provenance claim is made. No second-ring assembly/IR/API/SBOM evidence was required for the local test repair.

## Residual risks

- Full `moon ci` is not green.
- Workspace-wide tests are not green.
- Some Kani obligations time out or are undetermined.
- Historical source-length/test-integrity failures are superseded by the passing `moon run :source-length` addendum rerun; full `moon ci` was not rerun.
- Beads were not closed or pushed in this continuation.

## Addendum — choose-body compile-validation repair rerun

- Shared raw logs are in `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/`.
- The compile-workflow validation seam now uses canonical `choose.branches` / `choose.otherwise` validation after `parse_workflow_source`; legacy `parse_ast` validation remains unchanged.
- Passing reruns: `cargo test -p vb_compile --lib choose -- --nocapture` (25 passed), workspace choose behavior filter (4 passed), vb_storage codec/schema_one/trailing/adversarial, vb_ipc lib, postcard envelope wire lane, `moon run :fmt`, `moon run :check`, `moon run :lint-src`, and `moon run :source-length`.
- New raw log prefixes: `cargo-test-vb-compile-choose-body-validation-rerun.*`, `cargo-test-workspace-choose-body-validation-final.*`, `cargo-test-vb-storage-*-choose-repair.*`, `cargo-test-vb-ipc-lib-choose-repair.*`, `cargo-test-workspace-postcard-envelope-wire-choose-repair.*`, and `moon-run-*-choose-repair-final.*`.
- Full `moon ci` was not rerun; prior timeout evidence remains historical residual risk.

## Addendum — webhook trigger contract repair

- Master Trigger Contract §9 requires `webhook: {}` only. The compile validation seam now accepts empty webhook mappings and rejects any webhook configuration fields instead of requiring `path` and `method`.
- Changed files in this addendum: `crates/vb_compile/src/mod_compile_validation/part_05.rs`, `crates/vb_compile/src/mod_compile_validation/part_06.rs`, and `crates/workspace_tests/tests/vb_test_compile_parse_validate_behavior.rs`.
- Passing evidence in shared raw store `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/`: `cargo-test-workspace-webhook-accept-empty.*`, `cargo-test-workspace-webhook-reject-extra.*`, `cargo-test-workspace-compile-parse-validate-file-webhook-repair.*`, `moon-run-fmt-webhook-repair.*`, `moon-run-check-webhook-repair.*`, `moon-run-lint-src-webhook-repair.*`, `moon-run-source-length-webhook-repair.*`, and `root-jj-check-webhook-repair.*`.
- `rtk cargo test -p vb_compile --all-features` remains blocked by existing repeat-digest fixtures with invalid hyphenated workflow names; raw summary `cargo-test-vb-compile-all-features-webhook-repair.*`.

## Addendum — repeat-digest public-name fixture repair

- Shared raw logs are in `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/`.
- Repaired repeat digest public-name fixtures: `repeat_digest_integration`, `repeat_proptest`, `repeat_proptest_alt`; also normalized together digest fixture-generated public names exposed by the full `vb_compile` rerun.
- Passing repeat evidence: `cargo-test-vb-compile-repeat-digest-integration-final-repeat-fixture-repair.*` (10 passed) and `cargo-test-vb-compile-repeat-digest-proptest-final-repeat-fixture-repair.*` (5 passed).
- `cargo test -p vb_compile --all-features` now fails after repeat digest, in `together_digest_sensitivity` with the canonical `together.branches` versus legacy integer branch-target shape mismatch; raw `cargo-test-vb-compile-all-features-final-repeat-fixture-repair.*`.
- Workspace compile/validate behavior and validate-yaml behavior tests passed; `integration_validate_yaml_parsing` still has the two existing version-rejection assertion failures. Moon fmt/check/lint-src/source-length passed. Root/JJ check passed. No push/close/Dolt sync.

## Addendum — weakened assertion and source-length closure

- Shared evidence note: `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/weakened-assertion-closure-evidence.md`.
- Fixed `test-integrity` honestly by adding exact per-kind/magic-family assertions in `crates/vb_storage/src/tests.rs` and exact typed-error assertions in `fuzz/fuzz_targets/kind_validation.rs`.
- Split `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs` helper after the now-unblocked source-length gate exposed a 29-line hot function; test behavior unchanged.
- Passing reruns: `moon run :test-integrity`, targeted vb_storage tests, `cargo fuzz build kind_validation`, codec/schema/trailing/adversarial tests, lifecycle targeted test, `moon run :source-length`, `moon run :fmt`, `moon run :check`, `moon run :lint-src`, and final root/JJ checks.
- No push/close/Dolt sync.

## Addendum — codec black-hat blocker closure

- Shared evidence note: `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/black-hat-codec-blocker-closure-evidence.md`.
- Postcard envelope tests now assert exact `JournalError` variants/fields for wrong magic, payload digest mismatch, payload-too-large, truncation, and header checksum mismatch, including a dedicated proof that header checksum mismatch wins over payload digest mismatch when both are corrupt.
- RecordKind coverage now includes journal IDs 31..35, and compile tests now use exact typed error/message matches instead of string `.contains(...)` checks where feasible.
- Passing reruns: targeted/full postcard file, vb_storage codec/schema_one/trailing/adversarial, v1 primitive lowering, digest repeat unit, full `vb_compile --all-features`, and Moon fmt/check/lint-src/source-length.
- No push/close/Dolt sync.

## Addendum — current-source repackage 2026-07-09 (bead: vb-gzmcl)

- Re-ran the four targeted cargo test lanes, full `cargo test -p vb_compile --all-features`, and the four `moon run` gates against the current source state in `/home/lewis/src/isoloated/velvet-ballistics-w25-codec` (working copy `@` change id `zqzqkmsl`, parent `lumrtywu` / commit `c190b285` on `main@origin`). All ten commands exited 0 from a single Python-driven batch that captured stdout, stderr, and exit code per command.
- Raw evidence (new prefix `current-source-20260709-repackage-`):
  - `cargo test -p velvet-ballistics-workspace-tests --test postcard_envelope_wire_tests -- --nocapture` → exit 0; 23 passed (`current-source-20260709-repackage-postcard-envelope-wire-full.stdout.txt`, `*.stderr.txt`, `*.exitcode`).
  - `cargo test -p vb_compile --all-features` → exit 0; 1,479 passed in lib + 4 ignored + passing 38 test binaries (no `FAILED` or `error:`) (`current-source-20260709-repackage-vb-compile-all-features.*`).
  - `cargo test -p vb_storage --lib codec -- --nocapture` → exit 0; 175 passed (`current-source-20260709-repackage-vb-storage-codec.*`).
  - `cargo test -p vb_storage --lib schema_one -- --nocapture` → exit 0; 10 passed (`current-source-20260709-repackage-vb-storage-schema-one.*`).
  - `cargo test -p vb_storage --lib trailing -- --nocapture` → exit 0; 8 passed (`current-source-20260709-repackage-vb-storage-trailing.*`).
  - `cargo test -p vb_storage --lib adversarial -- --nocapture` → exit 0; 64 passed (`current-source-20260709-repackage-vb-storage-adversarial.*`).
  - `moon run :fmt` → exit 0; 1 task completed in 2.581 s (`current-source-20260709-repackage-moon-run-fmt.*`).
  - `moon run :check` → exit 0; `hot-cold-forbidden-apis` returned `FixturePass`; `agent-cli-contract` cached (`current-source-20260709-repackage-moon-run-check.*`).
  - `moon run :lint-src` → exit 0; `ignored-fallible-results`, `panic-surface`, `unsafe-audit` clean; `lint-src` (37d14901) green (`current-source-20260709-repackage-moon-run-lint-src.*`).
  - `moon run :source-length` → exit 0; `test-integrity` PASS; source-length gate summary shows `over_limit=0` for every category (`current-source-20260709-repackage-moon-run-source-length.*`).
- Manifest: `current-source-20260709-repackage-manifest.jsonl` records command, cwd, stdout path, stderr path, and exit code for each of the ten commands.
- `Cargo.lock` checksum is unchanged after the batch; `fuzz/Cargo.lock` is a new artifact already present in the working copy (`A fuzz/Cargo.lock` in `git status`), so the new manifest does not depend on lock churn.
- No code, JJ, or bead changes; no `bd dolt push`, `bd close`, subagent, or go-skill lifecycle was triggered.

## Cross-reference — Bundled behavior changes in the events file split

The events file split touched `JournalEvent`, `RecordKind`, and the wire encode/decode path used by every codec-closure bead in this bundle. The full disclosure of the three bundled behavior changes (F1 new `StepFailed` variant, F2 schema-2 RecordKind id remap, F3 serde-default removals on new-write required fields: `action_abi_digest` and `SlotWrittenEvent.extra`) lives in the canonical addendum at `.beads/vb-3ps37/evidence/2026-07-09-codec-closure/evidence-summary.md` and the schema-migration ledger at `.beads/vb-cwc0t/evidence/2026-07-09-codec-closure/schema-migration-ledger.md`.

This bead (vb-gzmcl, RecordKind/schema-1/trailing-byte codec parity closure bundle) inherits the same `RecordKind` id remap and schema-1 fallback path; the `cargo test -p vb_storage --lib trailing` and `cargo test -p vb_storage --lib adversarial` lanes specifically exercise the shared-envelope compat path documented in the schema-migration ledger. Each behavior change is split into its own bead via `bd create --deps discovered-from:vb-cwc0t`; this bead does not change scope.
