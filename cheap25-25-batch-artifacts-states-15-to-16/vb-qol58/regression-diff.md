---
bead_id: vb-qol58
schema_version: regression-diff/v1
state: 12
skill: formal-verifier
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58
formal_verifier_invocation_id: formal-verifier-vb-qol58-state12-20260701T225200Z
behavior_change: false
edits_count: 3
---

# Regression Diff: vb-qol58

## Summary

Three production-line edits applied in the parent commit's `working copy` (`@  vvzkpqnn 5e6431a1` per `jj log`). All three are byte-equivalent borrow expressions:

- `&mut <var>[..]` (full-array / full-vec slice indexing) → `<var>.as_mut_slice()` (canonical stable Rust method, auto-implemented since Rust 1.57, guaranteed in `nightly-2026-04-28` per `rust-toolchain.toml`).
- `[u8; N]::as_mut_slice` and `Vec<u8>::as_mut_slice` return the same `&mut [u8]` that `[..]` produces when applied to a fixed array or `Vec<u8>`.
- No allocation change, no layout change, no API change, no semantics change.

This removes the workspace deny-list lint flag `-D clippy::indexing_slicing` (and `-D clippy::string_slice`) emissions at the 3 cited sites. The deny-list at `.moon/tasks/all.yml:51` itself is byte-identical (SHA-256 `423e84fa22c28ad863a089a7e4ae2c6dfce4ae827f5db0d2cea991fca1f6134d` unchanged pre/post).

## Parent → Working Copy Diff (3 production-line edits)

All 3 diffs were independently re-derived via `diff(1) <(jj file show -r @- <path>) <(jj file show -r @ <path>)` and re-captured at `.evidence/vb-qol58/verifier/regression-diff.txt` (SHA-256 `901648b15ab4878864cb238896f0b7852ba3dbaf8ac0aaf2d6290bdc618f7aca`). Source: `diff(1)` direct from isolated JJ workspace.

### Edit 1 — `crates/vb_ipc/src/frame_types.rs:41`

```diff
@@ -38,7 +38,7 @@
     /// Encodes the header using the §21 little-endian wire layout.
     pub fn encode(self) -> Result<[u8; IPC_HEADER_LEN], IpcError> {
         let mut bytes = [0u8; IPC_HEADER_LEN];
-        let mut cursor = std::io::Cursor::new(&mut bytes[..]);
+        let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());
         cursor
             .write_u32::<LittleEndian>(IPC_MAGIC)
             .map_err(|_| IpcError::HeaderEncodeFailed)?;
```

| Field | Value |
|---|---|
| Production line | `crates/vb_ipc/src/frame_types.rs:41` |
| Function | `IpcFrameHeader::encode` |
| Before | `let mut cursor = std::io::Cursor::new(&mut bytes[..]);` |
| After  | `let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());` |
| Behavior change | none (`bytes.as_mut_slice()` returns the same `&mut [u8]` that `&mut bytes[..]` produces for `[u8; IPC_HEADER_LEN]`) |
| Lines touched | 1 (line 41) |
| Sites anchored | `IPC_MAGIC` write at line 43; `IPC_VERSION` write at line 46; 5 subsequent `write_u16`/`write_u64`/`write_u32` calls at lines 49–62; all 7 sites map `Err(_)` → `IpcError::HeaderEncodeFailed` per `error-taxonomy.md §1.1` — all byte-identical pre/post refactor |
| Ledger row | `PO-qol58-002-LEAD` (PASS) |

### Edit 2 — `crates/workspace_tests/src/test_util/seed.rs:23`

```diff
@@ -20,7 +20,7 @@
         }
         let mut rng = StdRng::seed_from_u64(seed);
         let mut bytes = [0u8; N];
-        rng.fill(&mut bytes[..]);
+        rng.fill(bytes.as_mut_slice());
         Some(Self { bytes })
```

| Field | Value |
|---|---|
| Production line | `crates/workspace_tests/src/test_util/seed.rs:23` |
| Function | `SeededBytes::<N>::new` |
| Before | `rng.fill(&mut bytes[..]);` |
| After  | `rng.fill(bytes.as_mut_slice());` |
| Behavior change | none (`bytes.as_mut_slice()` returns the same `&mut [u8]` that `&mut bytes[..]` produces for `[u8; N]`) |
| Lines touched | 1 (line 23) |
| Sites anchored | `N == 0` short-circuit at line 18-20 (returns `None`) — preserved verbatim; `StdRng::seed_from_u64(seed)` at line 21 — preserved verbatim; return at line 24 — preserved verbatim |
| Ledger row | `PO-qol58-003-LEAD` (PASS) |

### Edit 3 — `crates/workspace_tests/src/test_util/fixture.rs:58`

```diff
@@ -55,7 +55,7 @@
 
         let mut rng = StdRng::seed_from_u64(seed);
         let mut vec = vec![0u8; self.capacity.value];
-        rng.fill(&mut vec[..]);
+        rng.fill(vec.as_mut_slice());
         vec
```

| Field | Value |
|---|---|
| Production line | `crates/workspace_tests/src/test_util/fixture.rs:58` |
| Function | `FixtureBuilder::build_bytes` |
| Before | `rng.fill(&mut vec[..]);` |
| After  | `rng.fill(vec.as_mut_slice());` |
| Behavior change | none (`vec.as_mut_slice()` returns the same `&mut [u8]` that `&mut vec[..]` produces for `Vec<u8>` because `vec.capacity() == vec.len()` is preserved by the `vec![0u8; cap]` initialization) |
| Lines touched | 1 (line 58) |
| Sites anchored | `FixtureBuilder::with_capacity` constructor at line 47 (preserved verbatim); `FixtureCapacity::new` validation at `fixture.rs:19-33` (returns `Err(TestSetupError::InvalidCapacity(_))` for `0` or `> MAX_CAPACITY` — preserved verbatim) |
| Ledger row | `PO-qol58-003-LEAD` (PASS) |

## Files NOT Touched (verified via `jj diff`)

```
$ jj diff --summary
M crates/vb_ipc/src/frame_types.rs            # Edit 1 (above)
M crates/workspace_tests/src/test_util/seed.rs # Edit 2 (above)
M crates/workspace_tests/src/test_util/fixture.rs # Edit 3 (above)
```

Three files modified; total 3 insertions, 3 deletions. No other source files were touched. The deny-list at `.moon/tasks/all.yml` is byte-identical pre/post (verified via `sha256sum(jj file show -r @- .moon/tasks/all.yml)` == `sha256sum(jj file show -r @ .moon/tasks/all.yml)` == `423e84fa22c28ad863a089a7e4ae2c6dfce4ae827f5db0d2cea991fca1f6134d`).

## Live ripgrep Audit of `[..]` Pattern (zero matches in touched files)

```
$ rg -n '\[\.\.\]' crates/vb_ipc/src/frame_types.rs crates/workspace_tests/src/test_util/seed.rs crates/workspace_tests/src/test_util/fixture.rs
(no matches)
```

Confirms the deny-list pattern was removed from all 3 touched sites.

## Live ripgrep Audit of `as_mut_slice` Pattern (3 matches in touched files)

```
$ rg -n 'as_mut_slice' crates/vb_ipc/src/frame_types.rs crates/workspace_tests/src/test_util/seed.rs crates/workspace_tests/src/test_util/fixture.rs
crates/workspace_tests/src/test_util/fixture.rs:58: rng.fill(vec.as_mut_slice());
crates/workspace_tests/src/test_util/seed.rs:23: rng.fill(bytes.as_mut_slice());
crates/vb_ipc/src/frame_types.rs:41: let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());
```

Confirms the canonical verb is present at exactly the 3 expected production-line cites.

## Behavior Parity Verification

The byte-equivalence claim is verified by:

1. **Live `--diff` between `jj file show -r @-` and `jj file show -r @`** for each of the 3 paths: zero changes other than the 3 single-line borrows (captured at `.evidence/vb-qol58/verifier/regression-diff.txt`).
2. **`cargo test -p velvet-ballistics-workspace-tests --lib --all-features`** running 18 unit tests with **0 failed** (captured at `.evidence/vb-qol58/verifier/cargo-test.log`). The determinism tests `seeded_bytes_*` exercise `StdRng::seed_from_u64(seed)` → `rng.fill(bytes.as_mut_slice())` byte-for-byte; the capacity tests `*capacity_*` exercise `FixtureBuilder::build_bytes` → `rng.fill(vec.as_mut_slice())` with the seeded RNG preserved end-to-end.
3. **`cargo check -p vb_ipc --all-targets --all-features`** exit 0 under `-D warnings` (captured at `.evidence/vb-qol58/verifier/cargo-check.log`). The 24-byte IPC header byte layout is preserved verbatim; the 7 `IpcError::HeaderEncodeFailed` mapping sites are byte-identical pre/post.
4. **`moon run :lint-src`** exit 0 (captured at `.evidence/vb-qol58/verifier/lint-src.log`). The 16 deny-list `-D clippy::*` flags remain unchanged post-refactor.

## Status

- **3 edits applied; behavior parity preserved; lint deny-list preserved; gates exit 0.**
- **0 regressions detected.**
- **0 pre-existing global failures introduced by these edits.**
