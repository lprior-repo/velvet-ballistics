# Wave 3 — Agent 14: Ad-Hoc Digest-Binding Deep Dive

**Working dir:** `/home/lewis/src/velvet-ballistics`
**Bug chunk (9):** `vb-wi486, vb-widdi, vb-ww1ts, vb-y3az6, vb-y8tyj, vb-yfsc4, vb-zd2um, vb-zfyh5, vb-zlu3h`
**Scope:** digest-binding at the storage layer — workflow source, compiled IR, action ABI, policy digests, constant-time compare, pre-storage digest check.
**Tools:** read-only inspection of `crates/vb_storage/src/**`. No source modification. No beads.

## Per-bug table

| bug-id | pri | compiled-ir-digest-verified | full-digest-check | constant-time-compare | digest-pre-storage | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|
| vb-wi486 | P2 | n/a (core engine, not storage) | n/a | n/a | n/a | `cargo test -p vb_storage --lib --no-fail-fast` | 1270 passed / 0 failed (baseline healthy) | UNKNOWN | `crates/vb_core/src/replay/basic/handlers/mod.rs:49` — outside `vb_storage` digest scope; this is a typed-error mapping bug for invalid jump targets, no digest binding involved. |
| vb-widdi | P2 | n/a (codec trim path; no `record.ir`) | n/a | n/a | n/a | `cargo test -p vb_storage --lib latest_durable_snapshot_seq --no-fail-fast` | 4 passed / 0 failed | UNKNOWN | Trimming key-only reverse scan lives at `crates/vb_storage/src/trimming/logic.rs:20-37`; closure cites `vb-1rqz7.29 / SC-004` regression tests passing. Not a digest-binding path. |
| vb-ww1ts | P3 | n/a (runtime admission, not storage) | n/a | n/a | n/a | `cargo test --lib --workspace --no-fail-fast capability_count_mismatch` | 0 tests matched (no regression test exists for the OPEN bead) | UNKNOWN | `admission/guards.rs:29-40` (vb_runtime); bead is **OPEN**. No digest check on this path; the bug is about fabricated synthetic `Capability`/`ActionId` in error context. |
| vb-y3az6 | P0 | n/a (FrameSeedAccumulator is a recovery accumulator, no `record.ir` write) | n/a | n/a | n/a | `cargo test -p vb_storage --lib FrameSeedAccumulator --no-fail-fast` | 0 tests matched | PATCHED | Root-cause note in close-reason: `#[kani::proof_for(parse)]` and `journal/append/mod.rs:38-43` re-export fixes restored `vb_core` compile under `cfg(kani)`. FrameSeedAccumulator split was downstream symptom. No digest-binding regression — `replay/summary.rs:402` accumulator has no `record.ir` write path. |
| vb-y8tyj | P3 | n/a (rename only — no `put_compiled_ir` touched) | n/a | n/a | n/a | `cargo test -p vb_storage --lib append_queued_unfsynced --no-fail-fast` | 4 passed / 0 failed | PATCHED | Renamed `append_queued_unpersisted` → `append_queued_unfsynced` at `journal/internal.rs:70`; body byte-identical. SA-016 regression tests at `journal/tests.rs:2571, 2599` pass. No digest check added or removed on this path. |
| vb-yfsc4 | P0 | n/a (recover_full_journal reads events, not records) | **NO** (Full variant in `verify_digests` does NOT call `check_action_abi_digests` or `check_policy_digests`) | n/a | n/a | `cargo test -p vb_storage --lib recover_full_journal --no-fail-fast` | 1 passed / 0 failed | PARTIAL | `recovery/replay/recovery_ops.rs:51` now uses `events_for_run_full` (good); but `verify_digests` at `recovery/recover.rs:83-101` only invokes `check_workflow_source_digest` + `check_compiled_ir_digest`. Action ABI / policy digest checks are separately exported (`check_action_abi_digests`, `check_policy_digests`) but NOT called by `verify_digests` — this is the canonical **"full digest check omits action ABI/policy digests"** defect (`03-storage-recovery-defects.md:35-49`, `vb-mrwe.3`), outside this bug's blast radius. |
| vb-zd2um | P2 | n/a (inject_raw_event is journal-event injection, no compiled IR) | n/a | n/a | n/a | `cargo test -p vb_storage --lib inject_raw_event --no-fail-fast` | 0 tests matched (closure cites `journal/injection.rs:28-44, 64-77`) | PATCHED | `inject_raw_event`/`inject_seq_gap` at `journal/injection.rs:15, 37` now acquire `write_lock` and reject duplicates with `DuplicateEvent`. No digest-binding regression. |
| vb-zfyh5 | P3 | n/a (runtime admission mutex, not storage) | n/a | n/a | n/a | `cargo test --lib --workspace --no-fail-fast lock_admission` | 0 tests matched | UNKNOWN | Runtime admission at `vb_runtime` (mutex-poisoning recovery); outside `vb_storage` digest scope. Closure accepted — no digest path is exercised here. |
| vb-zlu3h | P2 | n/a (smoke-test replacement only) | n/a | n/a | n/a | `cargo test -p vb_storage --lib codec::tests --no-fail-fast` (163 tests, per close-reason) | closure cites 163 passed / 0 failed | PATCHED | `codec/tests.rs` lines 534, 674, 1756, 2459, 2655, 2739, 2863, 2996 now use `matches!(result, Ok(...))` with envelope-magic + byte-count bindings. No digest-binding regression. |

## Digest-binding violations surfaced during audit

These are NOT in the 9-bug chunk scope, but they are confirmed live defects that the agent should flag for downstream triage (matches the canonical `03-storage-recovery-defects.md` inventory):

1. **`put_compiled_ir` does NOT verify `record.ir` hashes to `record.digest` before insert.**
   - `crates/vb_storage/src/journal/source.rs:47-58` — direct path, no `verify_content_digest` call.
   - `crates/vb_storage/src/batch.rs:109-120` — batch path, no `verify_content_digest` call.
   - Compare with `put_workflow_source` at `source.rs:18-30` and `batch.rs:79-106` which DO call `verify_content_digest`.
   - Compare with `put_blob` at `blobs.rs:21` and `batch.rs:154` which DO call `verify_content_digest`.
   - Matches `03-storage-recovery-defects.md:22-33`: "Storage APIs can persist forged compiled IR under arbitrary digest keys." Suggested bead: `P0 reject forged compiled IR digest on direct and batch writes`.

2. **`verify_digests` (`DigestCheck::Full`) does NOT include action ABI or policy digest verification.**
   - `crates/vb_storage/src/recovery/recover.rs:83-101` — only checks workflow source + compiled IR.
   - `check_action_abi_digests` (line 109) and `check_policy_digests` (line 128) are exported but NOT called from `verify_digests`. Caller-supplied verification only.
   - Matches `03-storage-recovery-defects.md:35-49`: Section 18 requires "replay checks workflow source digest, compiled workflow digest, action ABI digest, and policy digest." Suggested bead: `P0 make full digest verification include action ABI and policy evidence` (`vb-mrwe.3`).

3. **Constant-time comparison concern (defense-in-depth only, not exploitable in this design).**
   - `crates/vb_storage/src/codec/payload.rs:13` uses `blake3::hash(payload).as_bytes() == &expected_digest` — Rust array `PartialEq` short-circuits on first mismatched byte, so this is NOT constant-time.
   - However: `verify_digest_match` is the wire-decode gate; `expected_digest` comes from the trusted on-disk record header (already authenticated via the Fjall keyspace → record pairing and the workflow-source admission `verify_content_digest` call). No attacker-controlled expected digest is compared against attacker-controlled content in the same call. This is acceptable for the current threat model; flag for review only if Section 18 mandates constant-time at this gate.

4. **`verify_digest_match` (decode-side) runs BEFORE typed decode, satisfying the "before persistent storage" requirement.** `codec/payload.rs:72` is invoked from `decode_record_payload` before any `postcard::from_bytes`. **PASS** for the decode side.

5. **Admission-side digest checks** (`verify_content_digest` at `journal/admission.rs:5-12`) run BEFORE insert for workflow_source (`source.rs:19`, `batch.rs:79`) and blob (`blobs.rs:21`, `batch.rs:154`) — **PASS**. **FAIL** for compiled_ir (item 1 above).

## Counts

- bugs-checked: 9
- PATCHED: 4 (vb-y3az6, vb-y8tyj, vb-zd2um, vb-zlu3h)
- PARTIAL: 1 (vb-yfsc4 — fix is correct for SR-001, but the broader full-digest omission defect remains)
- UNKNOWN: 4 (vb-wi486, vb-widdi, vb-ww1ts, vb-zfyh5 — all are bugs outside `vb_storage` digest scope; no digest binding path is broken on the affected production lines)
- FAIL / NOT-PATCHED: 0 within this chunk

## Top-3 NOT-PATCHED (with reason)

None of the 9 bugs in this chunk are NOT-PATCHED. However, three confirmed digest-binding violations exist in `vb_storage` that are NOT in this chunk:

1. **`put_compiled_ir` (direct path, `journal/source.rs:47`)** — does not call `verify_content_digest`. Storage can persist forged compiled IR under an arbitrary digest key. Closest bead ID: not in chunk; tracks to `03-storage-recovery-defects.md` P0 item 02.
2. **`put_compiled_ir` (batch path, `batch.rs:109`)** — same omission in the batched write surface.
3. **`verify_digests` `Full` variant (`recovery/recover.rs:83-101`)** — does not invoke `check_action_abi_digests` or `check_policy_digests`. Section 18 §8 requires all four digests; only two are checked. Tracks to `vb-mrwe.3` (`03-storage-recovery-defects.md` P0 item 03).

## File written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-14-adhoc-digest-binding.md`