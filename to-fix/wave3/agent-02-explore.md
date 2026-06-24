# Wave 3 Agent-02 Explore Report: Storage/Recovery/Codec/Digest Bead Sweep

Scope: vb_storage bug sweep (7 IDs). Read-only verification of bead closure
against current `crates/vb_storage/src/**` source. Status of all 7 beads in
`bd` is `CLOSED`; this report records whether the actual source and tests
match each bead's closure claim.

## Per-bug mapping

| bug-id | pri | files-touched | test-file | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|
| vb-1rqz7.2  | P0 | `crates/vb_storage/src/journal/injection.rs:30,52` | (missing — claimed `journal/regression_tests_vb_1rqz7.rs`) | `cargo test -p vb_storage --lib inject_raw_event --no-fail-fast` | 0 tests filtered | NOT-PATCHED | `injection.rs:30` `inject_raw_event` still calls `self.events.insert(key.to_vec(), value)?` directly with no `self.write_lock` acquisition and no `contains_key` duplicate check; `injection.rs:52` `inject_seq_gap` does the same. The companion `append_unfsynced` in `journal/internal.rs:38-59` already demonstrates the correct pattern (lock + `contains_key` + `DuplicateEvent`). Regression test file `journal/regression_tests_vb_1rqz7.rs` does not exist anywhere in the tree (only `tests/` at storage crate root and `vb_storage/tests/` integration dir). |
| vb-1rqz7.20 | P0 | `crates/vb_storage/src/queue/batch.rs` (whole file, 37 lines) | `crates/vb_storage/src/queue/tests.rs` (lines 297-323, 827-849) | `cargo test -p vb_storage --lib batch_builder --no-fail-fast` | 7 passed, none cover capacity | NOT-PATCHED | `BatchBuilder::push(&mut self, event: JournalEvent)` is infallible and unbounded (`batch.rs:16-18`). No `try_push`, no `try_extend`, no `with_capacity(max)` constructor, no `Result`-returning variant. The four `batch_builder_*` tests only assert `len()`, `is_empty()`, `as_slice()` and `push` acceptance — none probe a capacity overflow or fallible growth path. The `JournalWriterQueue` separately enforces queue capacity (`writer.rs:63-68`), but the bead was scoped to `BatchBuilder` itself. |
| vb-1rqz7.21 | P0 | `crates/vb_storage/src/journal/source.rs:47-58` (`put_compiled_ir`); `crates/vb_storage/src/records.rs:244-251` (`CompiledIrRecord`) | (no dedicated "metadata_hash on read" test) | `cargo test -p vb_storage --lib submit_artifact --no-fail-fast` | 17 passed, none cover forged IR digest | NOT-PATCHED | `CompiledIrRecord` has only `{digest, ir}` fields (no `metadata_hash`). `put_compiled_ir` in `journal/source.rs:47-58` stores the record directly without calling `verify_content_digest(&record.ir, &record.digest.as_bytes())`. Compare to `put_workflow_source` immediately above (`source.rs:18-29`) which DOES call `verify_content_digest`. The proptest `submit_artifact_checksum_mismatch_rejected` (`proptests.rs:770`) goes through `submit_artifact`'s in-memory check (admission.rs:373-375), not the lower-level `put_compiled_ir` — bypassing admission still forges IR. The bead summary ("validate_compiled_ir_record must compare stored metadata_hash") has no corresponding `metadata_hash` field anywhere in the source. |
| vb-1rqz7.22 | P0 | `crates/vb_storage/src/admission.rs:317-420` (`submit_artifact`, `submit_artifact_with_contracts`); `crates/vb_storage/src/admission/tests.rs:238-272` | `crates/vb_storage/src/admission/tests.rs:238,315` | `cargo test -p vb_storage --lib submit_artifact --no-fail-fast` | 17 passed | PARTIAL | `submit_artifact_relaxed_persists_and_returns_artifact` (`admission/tests.rs:266-271`) now performs a `journal.compiled_ir(workflow.digest())` readback for Relaxed; `submit_artifact_journaled_roundtrip_bytes_match` (`admission/tests.rs:323-327`) does the same for Journaled. `submit_artifact_strict_is_durable` (line 295-308) does NOT perform an explicit `compiled_ir()` readback assertion — it only checks `verification.durable`. SA-009 ("relaxed must verify live readback like checked policies") is satisfied for Relaxed vs Journaled but the Strict policy readback parity is unverified. |
| vb-1rqz7.23 | P0 | `crates/vb_storage/src/kani_admission.rs` (whole file, 152 lines) | `crates/vb_storage/src/kani_admission.rs:72-102` | (Kani harness compile is gated; no cargo test target) | n/a — harness statically violates GOD RULE 1 | NOT-PATCHED | `kani_admission.rs:29-70` defines `minimal_valid_workflow()` that hardcodes a fixed `WorkflowParts` (two nodes, single `ConstValue::I64(42)`, `ResourceContract::DEFAULT`). The three harnesses `submit_artifact_kani` (line 72-81), `submit_artifact_with_contracts_kani` (line 83-92), and `admit_compiled_artifact_kani` (line 94-102) all feed this hardcoded workflow through `kani::any` only for the policy byte. AGENTS.md GOD RULE 1 explicitly forbids hardcoded Kani shapes for storage admission; `kani_journal_duplicate.rs` and `kani_storage_invariants.rs` in the same crate use `kani::any()` properly. SA-014 (XOR-only BLAKE3 stub) is moot here because the harness does not stub `compute_policy_digest` at all — it never proves the digest binding for arbitrary workflows. |
| vb-1rqz7.24 | P0 | `crates/vb_storage/src/admission.rs:218-243` (`compute_policy_digest`); `:336-420` (`submit_artifact_with_contracts`) | (no regression test asserting source error preservation) | `cargo test -p vb_storage --lib submit_artifact --no-fail-fast` | 17 passed | NOT-PATCHED | `compute_policy_digest` (`admission.rs:218-243`) calls `postcard::to_slice(...).map_err(\|_\| JournalError::ArtifactMalformed)` at line 223 and 230, silently discarding postcard error variants. The fallback path at lines 232-235 returns `WorkflowDigest::from_bytes([0u8;32])` with no diagnostic. `submit_artifact_with_contracts` continues the pattern at lines 340, 354, 366, 371, 385, 398, 411, 413, 420, 436, 471, 513, 521, 528. There is no `JournalError::PostcardEncodeFailed` or `PostcardSerializeFailed` variant surfaced from any of these sites; the bead's "preserve source errors" requirement is unmet. The `JournalError::PostcardDecodeFailed` variant exists (`error/mod.rs`) but encode failures are still squashed to `ArtifactMalformed`. |
| vb-1rqz7.25 | P0 | `crates/vb_storage/src/keys.rs:23-59` (`KeyspaceScanPolicy`); `crates/vb_storage/src/headers.rs:57-78` (`run_headers`); `crates/vb_storage/src/error/mod.rs:148-156` (`MalformedKeyspaceRow`) | `crates/vb_storage/src/tests.rs:1847-1890` (`cc002_run_headers_fails_closed_on_malformed_key`) | `cargo test -p vb_storage --lib cc002_run_headers_fails_closed_on_malformed_key --no-fail-fast` | 1 passed | PARTIAL | `KeyspaceScanPolicy::FailClosed` (default) and `SkipMalformed` added to `keys.rs:33-45` with `default_production()` and `default_doctor()` accessors. `run_headers()` in `headers.rs:57-78` now returns `JournalError::MalformedKeyspaceRow` on length mismatch. `preview_keyspace` in `preview.rs:41-98` keeps its original `Err(_) => continue` silent-skip on bad keys (line 63), and is intended as best-effort. However, no scan API actually exposes the `KeyspaceScanPolicy` parameter — `list_artifacts()` (`artifacts.rs:16-27`) returns `UnexpectedEof` on a truncated prefix scan instead of using the policy enum, and the trimming scans (`trimming/logic.rs:21, 83, 225, 254`) do not check key length at all. CC-002 test passes for `run_headers` only; cross-keyspace standardization is not done. |

## Summary

- bugs-checked: 7
- pass (PATCHED): 0
- fail (NOT-PATCHED): 4 (vb-1rqz7.2, vb-1rqz7.20, vb-1rqz7.21, vb-1rqz7.23, vb-1rqz7.24)
- partial (PARTIAL): 2 (vb-1rqz7.22, vb-1rqz7.25)
- unknown: 0
- tested-with-cargo: 4 (vb-1rqz7.2, vb-1rqz7.20, vb-1rqz7.21, vb-1rqz7.22, vb-1rqz7.24, vb-1rqz7.25)
- gated-behind-kani: 1 (vb-1rqz7.23)

## Top-3 NOT-PATCHED

1. **vb-1rqz7.2** (`crates/vb_storage/src/journal/injection.rs:30`, `:52`) —
   `inject_raw_event` and `inject_seq_gap` directly invoke
   `self.events.insert(...)` without acquiring `self.write_lock` and without
   a `contains_key` duplicate check, even though the sibling
   `append_unfsynced` (`journal/internal.rs:38-59`) already implements the
   correct pattern. The claimed regression test file
   `journal/regression_tests_vb_1rqz7.rs` does not exist.

2. **vb-1rqz7.20** (`crates/vb_storage/src/queue/batch.rs:16-18`) —
   `BatchBuilder::push` is infallible and unbounded; no `try_push`, no
   `with_capacity(max)`, no `Result`-returning API exists. All four existing
   `batch_builder_*` tests exercise only happy-path growth.

3. **vb-1rqz7.21** (`crates/vb_storage/src/journal/source.rs:47-58`) —
   `put_compiled_ir` stores `CompiledIrRecord` without calling
   `verify_content_digest(&record.ir, &record.digest.as_bytes())`, so any
   caller bypassing `submit_artifact` can persist forged IR under an
   arbitrary digest. The `CompiledIrRecord` struct
   (`records.rs:244-251`) has no `metadata_hash` field, so the bead's
   "validate metadata_hash on read" requirement cannot be met by reading
   any existing field.

(Honourable mention: vb-1rqz7.23 violates GOD RULE 1 — `kani_admission.rs`
hardcodes `minimal_valid_workflow()` instead of using `kani::any()`.)

## file-path written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-02-explore.md`