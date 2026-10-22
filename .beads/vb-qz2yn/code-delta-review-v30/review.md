# vb-qz2yn CODE Delta Review v30 — Independent Local 5090 Reviewer

**Review scope:** Delta between v20 source packet and v30 commit `7e71ff4bbc7fe06b20fea3f07a770980e0ab0064`. Two production files changed since v20; four previously reviewed production files unchanged.

## Verdict: APPROVE (narrow CODE delta)

Gates and acceptance (V33 source gate, test rerun, storage/cross-reexec integration) remain pending and are not assessed here.

## Delta Summary

### 1. journal/artifact_acceptance.rs — Write Guard + Borrow Fix

**Change A: Lexical write guard binding**
- `let write_guard = ...` → `let _write_guard = ...`
- Removed explicit `drop(write_guard)` at function end
- RAII drop at `Ok(stamped)` return preserves identical lifetime

**Change B: Needless-borrow removal**
- `batch.insert(&journal.compiled_ir, &artifact_key, encoded)` → `batch.insert(&journal.compiled_ir, artifact_key, encoded)`
- `artifact_key` is `[u8; DIGEST_KEY_BYTES]` (Copy); Fjall 3.1.4 `insert` takes `K: Into<UserKey>` by value
- No key bytes change; resolves Clippy needless-borrow lint

### 2. journal/core.rs — Mechanical Cleanup

- Import reorder (rustfmt): crate imports now precede std imports
- `#[cfg(test)]` remains atomically attached to its import
- 7 blank lines removed between methods
- 306 lines → 299 lines (within 300 contract)
- No code, comment, or logic changes

## Verification Checks

### Guard Lifetime (issue_artifact, lines 173-184)
- ✅ `_write_guard` bound at line 177 via `Mutex::lock().map_err(...)`
- ✅ All operations execute within locked scope:
  - `read_counter(journal)` — keyspace read
  - `prepare_artifact(last, artifact)` — pure compute
  - `encode_record(&stamped)` — pure compute
  - `build_batch(...)` — batch staging
  - `commit_batch(journal, batch, policy)` — policy durability + commit
- ✅ Guard released automatically at function return via RAII
- ✅ No early release, no extra block, no lock recursion
- ✅ Error paths (counter read, preparation, encoding, batch build, commit) all return before `Ok(stamped)`, dropping guard correctly
- ✅ Identical semantics to v20 explicit-drop version

### Borrow Fix Correctness
- ✅ `compiled_ir_key()` returns `Result<[u8; DIGEST_KEY_BYTES], JournalError>` — fixed array, Copy
- ✅ Fjall 3.1.4 `OwnedWriteBatch::insert<K: Into<UserKey>, V: Into<UserValue>>` accepts values directly
- ✅ `[u8; N]` and `&[u8; N]` both convert to `UserKey` via `Into` with identical bytes
- ✅ No ownership or bytes change in key storage

### core.rs Mechanical Changes
- ✅ All 12 declared keyspaces intact (declared_keyspaces, lines 163-180)
- ✅ probe_health includes artifact_acceptance probe (line 241)
- ✅ All 3 test fault hooks unchanged (persist, compiled_ir_readback, batch_commit)
- ✅ Import structure canonical: crate first, std second, cfg(test) atomic
- ✅ 299 physical lines (< 300 contract)

### Prior v20 Contracts (unchanged files)
- ✅ Single private batch + lock — still satisfied
- ✅ StrictSyncAll policy parity — still satisfied
- ✅ Error variants and envelope — still satisfied
- ✅ Digest binding — still satisfied
- ✅ Counter corruption/overflow protection — still satisfied
- ✅ 12 keyspaces / probe / readback — still satisfied

## Prior Review Applicability

- v20 review (local 3090) approved all 6 production files against contract
- v20 wrong claims (Fjall version, CompiledIrReadback) were already corrected in v20
- This delta only fixes v20's DISCARD005 violation (explicit drop) and line count violation (>300)
- No new production logic; purely mechanical repair

## Scope Limitations

- CODE delta only; no test-suite approval
- Tests (1643+71) were run against v20 before this delta; must rerun against v30
- Source gate V33 running separately; no pass claim inferred
- No formal verification, performance, or power-loss claims
- CLI and packed paths remain out of scope (parent bead vb-0qwfq)
