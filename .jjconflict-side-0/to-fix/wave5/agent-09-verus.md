# Wave 5 / Agent-09 / Verus Review Report

Scope: 4 bug IDs in `/tmp/wave5-chunk-09.txt` — `vb-p7zza`, `vb-qagk2`, `vb-qapik`, `vb-qmomy`.
Mode: Read-only, no source modifications, no beads created.

## Verus Registry Pass

`bash scripts/verify-verus.sh` ran against `contracts/proof_obligations.yaml`.
Exit status: success (`VERUS_REGISTRY_OK evidence=.evidence/verus`).
The full registry drive passed; however the four bugs in this chunk each touch
production code that is **not** modeled by any production-bound Verus artifact.

## Per-Bug Verus Review

| bug-id | pri | verus-artifact | vacuum-proof | source-fix | test | verus-cmd | verus-result | cargo-result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| vb-p7zza | P2 | none for `cli_envelope.rs` (closest is `vb_ahfl_metadata_envelope_production.rs`, models `vb_ui_model::EnvelopeKind` not `vb_cli::cli_envelope::Kind`) | n/a (no artifact to evaluate) | NOT FIXED — `crates/vb_cli/src/cli_envelope.rs:44,91,132,169` still carry 4× `#[allow(dead_code)]`; `Kind::from_str`, `build_envelope`, and `EnvelopeError` all still present in production module | `cargo test -p velvet-ballistics --lib --no-fail-fast` → 214 passed; cli_envelope tests are gated behind `mod cli_envelope` only inside `src/main.rs` and are not collected by `--lib` | n/a (no production-bound Verus artifact for cli_envelope.rs exists in the registry; closest mirror `vb_ahfl_metadata_envelope_production.rs` passes its own verify, but for unrelated vb_ui_model types) | n/a | 214 passed, 0 failed (lib only; the cli_envelope `#[cfg(test)] mod tests` is in `main.rs` and not collected here) | NOT-PATCHED | `cli_envelope.rs:44,91,132,169` `#[allow(dead_code)]`; bug close reason asserts the annotations should have been removed |
| vb-qagk2 | P1 | `verification/verus/vb_jpq724_events_for_run_production.rs` (`verus --crate-type=lib` → 5 verified, 0 errors) | partial — spec models `events_for_run`/`events_for_run_from` snapshot authority + sequence validation, but does **not** model the claimed `run_seq_gap.contains_key(...)` gap-marker fix (no `run_seq_gap` keyspace exists in production) | NOT FIXED — (a) `cross_crate_adversarial.rs:626` still uses `flags = 0x1234` (close reason required `0x0034`); (b) `lifecycle_integration.rs:1316` still calls `vb_cli::lifecycle::cancel(run, &journal)` (close reason required removal); (c) `crates/vb_storage/src/journal/replay.rs:72-119` (`events_for_run_bounded` + `events_for_run_from`) has **no** `run_seq_gap` keyspace check (no such keyspace exists anywhere in `crates/vb_storage/`) | `cargo test -p velvet-ballistics --test cross_crate_adversarial runtime_to_ipc_frame_header_roundtrip_preserves_all_fields` → ok; `cargo test -p velvet-ballistics --test lifecycle_integration replay_with_ --no-fail-fast` → 2 passed, 0 failed | `verus --crate-type=lib verification/verus/vb_jpq724_events_for_run_production.rs` | 5 verified, 0 errors (artifact passes, but does not model the claimed `run_seq_gap` keyspace fix and has no `requires`/`ensures` binding to the not-yet-implemented production gap-marker detection) | cross_crate_adversarial 70/70 passed; lifecycle_integration `replay_with_*` 2/2 passed; production decode at `frame_types.rs:67-120` does NOT check `flags` reserved bits, and `inject_seq_gap` in `injection.rs:37-54` actually injects into the **events** keyspace, so `PostcardDecodeFailed` (not a `SequenceGap` from a separate keyspace) drives the missing-event path | NOT-PATCHED | production replay.rs source unchanged; test source unchanged (0x1234, cancel call still present); close reason is misleading because the described `run_seq_gap` keyspace and `run_seq_gap_key` do not exist in the codebase |
| vb-qapik | P4 | `verification/verus/recovery_hydration_contracts.rs` (10 verified, 0 errors) and `verification/verus/recovery_verification.rs` (verifies clean) — both model abstract `SpecRecoveryInput`/`SpecRecoveryFrameSeed` decision lattice, NOT `write_recovered_snapshot` or `encode_snapshot_slots`/`encode_snapshot_taint` | partial — neither artifact has any `requires`/`ensures` that bind to the function named in SR-019; the bug fix talks about `encode_snapshot_slots` and `encode_snapshot_taint` projections, neither name exists anywhere in the production code | NOT FIXED — file `crates/vb_storage/src/recovery/snapshot_write.rs` does not exist; the duplicated-encoding lives in the test helper `crates/vb_storage/src/recovery/tests.rs:2233-2248`, where `snapshot_with_slots` still does `postcard::to_allocvec(&slots)` for **both** `slots_bytes` and `taint_bytes` (same `slots: Vec<(SlotIdx, SlotValue, Taint)>` value is encoded twice — the SR-019 doubling is still present) | `cargo test -p vb_storage --lib hydrate_run_frame_reconstructs_frame_from_snapshot_and_tail_events --no-fail-fast` → ok (tests tolerate the doubled payload because `decode_snapshot_slots` is symmetric) | `verus --crate-type=lib verification/verus/recovery_hydration_contracts.rs` (and `recovery_verification.rs`) | 10 verified, 0 errors (and the other passes too); neither contains `write_recovered_snapshot`/`encode_snapshot_slots`/`encode_snapshot_taint` symbols, so there is no production-bound check that the bug has been fixed | vb_storage recovery tests pass; the doubled postcard payload in `snapshot_with_slots` is still emitted | NOT-PATCHED | `recovery/tests.rs:2238-2240` still encodes the same `slots` Vec for both `slots_bytes` and `taint_bytes`; the file `crates/vb_storage/src/recovery/snapshot_write.rs` referenced by the bug does not exist |
| vb-qmomy | P1 | none — no Verus artifact references `IpcCommand`, `IpcFrameHeader`, or `MaxPayloadBytes` (verified via `grep` across `verification/`) | n/a (no artifact to evaluate) | CLOSED VIA DELETION — file `crates/vb_ipc/tests/red_queen_capabilities.rs` does NOT exist in the current `main` branch (only on remote-only branches `bug-batch/5-p4-simplifications`, `cleanup-30r`, `femdation-round-9-20260622`, `push-vxqqsnpknxts`, `vb-7cz93-fix`, `vb-dr8k7`, `vb-etlnt-fix`, `wave-15-push`); close reason claiming "all 19 tests in crates/vb_ipc/tests/red_queen_capabilities.rs pass" is **factually wrong** because the file is absent from HEAD's tree (`git ls-tree HEAD crates/vb_ipc/tests/` shows only `ipc_command_properties.rs` and `proptest_ipc_error_codes.rs`) | `cargo test -p vb_ipc --no-fail-fast` → 15 passed, 0 failed (no red_queen_capabilities suite present, so 0 tests are "passing" in that suite) | `verus --crate-type=lib verification/verus/ipc_runtime_transitions.rs` (closest IPC artifact; 7 verified, 0 errors, but models terminal/shutdown transitions only — unrelated to the deleted test file) | 7 verified, 0 errors (unrelated to the bug) | vb_ipc 15/15 passed | NOT-PATCHED (close-reason integrity) | `git ls-tree HEAD --name-only crates/vb_ipc/tests/` returns only two files; close reason claims a 19-test file that is not on disk; bug technically mitigated by file absence but the bead closure narrative is incorrect |

## Aggregate Summary

- bugs-checked: **4** (all 4 IDs in chunk)
- pass / fail / partial / unknown counts:
  - PATCHED: 0
  - NOT-PATCHED: 3 (`vb-p7zza`, `vb-qapik`, `vb-qmomy`)
  - PARTIAL: 1 (`vb-qagk2` — tests pass but source fix narrative is wrong; production code unchanged from the pre-fix state)
  - UNKNOWN: 0
- vacuum-proof cases: **0 confirmed vacuum proofs in this chunk.** The artifacts that do exist
  (`vb_jpq724_events_for_run_production.rs`, `recovery_hydration_contracts.rs`,
  `recovery_verification.rs`, `ipc_runtime_transitions.rs`) all verify, but none bind via
  `requires`/`ensures` to the specific production functions named in any of the four bugs.
  The closest case is `vb_jpq724_events_for_run_production.rs` — its spec maps to
  `events_for_run` / `events_for_run_from`, but the bug fix narrative introduces a
  `run_seq_gap.contains_key(...)` check that the production code does NOT implement and the
  spec does NOT model. So while the artifact is not a literal vacuum proof, it has drifted
  from the production seam in a way that allows a "PASS" verdict to coexist with an
  un-implemented fix.

## Top-3 NOT-PATCHED with reason

1. **vb-qapik** — `crates/vb_storage/src/recovery/tests.rs:2238-2240` still encodes the
   same `Vec<(SlotIdx, SlotValue, Taint)>` for both `slots_bytes` and `taint_bytes`. The
   file named in the bug (`crates/vb_storage/src/recovery/snapshot_write.rs`) does not
   exist; the duplication lives in a test helper. No Verus artifact references the
   encode functions, so the doubling is invisible to formal verification.

2. **vb-p7zza** — `crates/vb_cli/src/cli_envelope.rs:44,91,132,169` still carry four
   `#[allow(dead_code)]` annotations. `Kind::from_str`, `build_envelope`, and the
   `EnvelopeError` enum are all still in production scope. The close reason required
   removing the annotations and gating `from_str`/`build_envelope` behind `#[cfg(test)]`,
   none of which happened. No Verus artifact for `cli_envelope`.

3. **vb-qagk2** — `crates/vb_storage/src/journal/replay.rs:72-119` (`events_for_run_bounded`)
   has no `run_seq_gap.contains_key` check; no `run_seq_gap` keyspace or `run_seq_gap_key`
   function exists anywhere in the codebase. The two test files still contain the patterns
   the close reason said were fixed: `cross_crate_adversarial.rs:626` uses `flags = 0x1234`,
   and `lifecycle_integration.rs:1316` still calls `cancel(run, &journal)`. The Verus
   artifact `vb_jpq724_events_for_run_production.rs` verifies but does not model the
   claimed gap-marker detection. (Mentioned here as NOT-PATCHED despite tests passing,
   because the source-fix narrative is materially false.)

## File Path Written

`/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-09-verus.md`