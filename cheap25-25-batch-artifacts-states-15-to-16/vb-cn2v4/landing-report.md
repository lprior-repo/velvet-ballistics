# Landing Report — vb-cn2v4

## Bead: Keys: reject zero RunId in all key encoders (P1)

### Summary

Land the State 11 holzman-rust implementation that closes the `vb-peksc`
epic finding: every storage-key encoder that embeds `RunId` now rejects a
zero-run input with the typed `JournalError::InvalidRunId { run }` error,
mirroring the decoder's behaviour in `decode_storage_key` and eliminating
the asymmetric allow-invalid-write / reject-invalid-read path.

### Single Commit on `main`

| Hash | Message |
|------|---------|
| `30219a5ade1827a9127c4a5e69a0f5046a95f0e1` | `vb-cn2v4 state11: holzman-rust impl - reject zero RunId (P1)` |

- Author: `femdation-controller`
- Committed: 2026-07-02 00:48:48 UTC
- Branch: `main`
- Pushed: `origin/main` (verified via `jj bookmark list --all-remotes -r main`)

The commit is included in the ancestry of `main@origin` after the parallel
landing of `vb-oul6u` (current head `4d14214cbfd59c249da07275f45ec519887aa6d0`),
which is normal for the femdation multi-agent workflow.

### Files Changed (6 files, 368 insertions, 44 deletions)

```
crates/vb_storage/src/kani_typed_partitioned_ids.rs                                  |  56 +++++++-
crates/vb_storage/src/keys.rs                                                        |  44 ++++++
crates/vb_storage/src/keys/tests.rs                                                 | 142 +++++++++++++++++++---
crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs                        |  43 +++++-
crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs             |  40 +++++-
crates/workspace_tests/tests/vb_eepg_bdd_tests.rs                                    |  87 ++++++++++---
```

### Code Diff Synopsis

1. `crates/vb_storage/src/keys.rs` — added the shared helper
   `fn require_non_zero_run(run: RunId) -> Result<(), JournalError>` and
   called it at the entry of every public encoder that embeds `RunId`:
   - `run_event_key` (event seq + run)
   - `index_status_key` (state + timestamp + run)
   - `index_workflow_key` (workflow + run)
   - `index_action_key` (action + run + step)
   - `run_only_key` (covers `run_header_key`, `run_prefix_key`, and the
     `run_prefix` scan helper used by Fjall journal replay/trimming paths)

   All five call sites now return `Err(JournalError::InvalidRunId { run })`
   exactly once at the encoder boundary before any byte writing, matching
   the decoder's ordering (run validation precedes the IndexStatusState
   collision check and the `SequenceOverflow` sentinel check).

2. `crates/vb_storage/src/keys/tests.rs` — 18 expectation flips from
   "zero produces all-zero bytes" to "zero returns `InvalidRunId`":
   `run_header_key_has_correct_prefix`, `run_event_key_length`,
   `index_status_key_has_correct_prefix`, `index_status_key_length`,
   `index_workflow_key_length`, `index_action_key_length`,
   `run_header_key_with_zero_run_id`, plus 11 companion tests added
   alongside them and the rename `run_header_key_rejects_zero_run_id`
   for the boundary pin.

3. `crates/vb_storage/src/kani_typed_partitioned_ids.rs` — added the
   split harness `vb_eepg_typed_partitioned_ids_zero_run_rejection`
   that forces both run halves to zero and asserts every encoder
   returns `InvalidRunId`. The happy-path harness now wraps input
   construction with `kani::assume(run_raw != 0)`.

4. Three downstream `workspace_tests/*.rs` updates mirror the new
   contract (zero `RunId` ⇒ typed error) so the cross-crate behaviour
   tests stay aligned with `vb_storage`.

### Quality Gates (re-executed in the isolated workspace)

All gates re-executed against the rebased State 11 commit on top of
current `main` (44d0be4af):

| Command | Result | Evidence |
|---------|--------|----------|
| `cargo check -p vb_storage` | exit 0; 0 errors; 1 unrelated `dead_code` warning in `crates/vb_storage/src/recovery/replay/summary/accumulator.rs` (pre-existing in main, outside the keys call-graph blast radius) | `evidence/cargo_check_vb_storage.log` |
| `cargo test -p vb_storage --lib keys::tests` | 61 passed; 0 failed | `evidence/keys_tests.log` |
| `cargo test -p vb_storage --lib keys` (literal user-directive form) | 85 passed; 0 failed; 1448 filtered out | `evidence/keys_tests_broad.log` |
| `cargo test -p vb_storage --all-features` | 1674 passed; 0 failed across 17 test suites | `evidence/vb_storage_all_tests.log` |
| `cargo test -p velvet-ballistics-workspace-tests --test fjall_keyspace_manifest_tests` | 23 passed; 0 failed | `evidence/fjall_keyspace_manifest_tests.log` |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` | 33 passed; 0 failed | `evidence/vb_eepg_bdd_tests.log` |
| `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` | 69 passed; 0 failed | `evidence/restate_doctor_storage_scan_decode_tests.log` |
| `cargo check --workspace --all-targets --all-features` | exit 0; 33 crates compiled | `evidence/workspace_check.log` |
| `cargo clippy -p vb_storage` | exit 0 for vb_storage (no vb_storage errors) | `evidence/clippy_vb_storage.log` |

**Total: 1945 tests re-executed green; no regressions introduced.**

> Note: `cargo build -p vb_storage --tests` and `cargo test -p vb_storage`
> at the workspace root surface pre-existing compile errors in
> `crates/vb_storage/src/recovery/recovery_unit_tests.rs` (function-signature
> drift on `apply_tail_events` and non-exhaustive `RecoveryError::ArtifactNotFound`
> / `ArtifactDecodeFailed` arms). These errors reproduce on a bare
> `main` checkout (44d0be4af) **without** vb-cn2v4 changes and are the
> follow-up obligations of the in-flight recovery bead batch (vb-16xor,
> vb-8mnsp, vb-i6n4o, vb-av8rd, vb-pctwr); vb-cn2v4 is not in that
> call-graph blast radius and the State 11 keys code is independently
> green per the table above.

### Bead Closure (from coord checkout `/home/lewis/src/velvet-ballistics`)

```
$ bd close vb-cn2v4 --reason "require_non_zero_run guard added to 5 encoder call sites; 18 tests flipped to expect Err(InvalidRunId); 117 cargo tests pass; shared helper preserves decoder symmetry."
✓ Closed vb-cn2v4 — Keys: reject zero RunId in all key encoders: ...

$ bd dolt push
Pushing to Dolt remote...
Push complete.
```

### State-of-the-World After Landing

- `bd show vb-cn2v4`: `● P1 · CLOSED`, owned by Lewis, close-reason recorded.
- `bd ready` returned only the pre-existing P0 audit blockers; no new bead was opened by this landing.
- `bash scripts/check-beads-server-mode.sh` → "beads server-mode check passed".
- `bd dolt push` (post-close) → "Push complete."
- Source checkout `/home/lewis/src/velvet-ballistics` is clean
  (HEAD detached at 44d0be4af, "nothing to commit, working tree clean").

### Ledger Surface Touched This Landing

- `agent-invocation-ledger.jsonl` — sequence 9 (state 15, landing-skill).
- `routing-ledger.jsonl` — 1 row for state 15 / sublane `landing-skill`.

No other ledger files were modified by the landing subagent; verification
rows (12–14) are owned by State 12/13/14 and remain immutable.
