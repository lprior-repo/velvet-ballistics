# Codebase Map — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Captured at: 2026-07-01 (state 2 / explore)
> Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`
> jj workspace root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`
> Upstream: `origin/main @ 2c8ea33c9`

## 1. Bead Premise vs Live State

The bead description asserts "Multiple source files contain unchecked slicing/indexing that violates lint-src." Empirical verification at this JJ-workspace tip **disagrees**:

- `cargo clippy --workspace --lib --bins --examples --all-features` with the production `.moon/tasks/all.yml:51` deny list (`-D clippy::indexing_slicing -D clippy::get_unwrap -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used -D clippy::string_slice -D clippy::arithmetic_side_effects -D clippy::as_conversions …`) returns **EXIT=0** on the current tree.
- The same gate excludes `tests/**` and `#[cfg(test)] mod tests` blocks by construction (`--lib` target only).
- `cargo check -p vb_ipc --all-targets --all-features` (which DOES compile tests) is also EXIT=0.

What the bead is actually describing — based on prior merge history (`origin/main` log) and the bead's explicit keyword list of `.get().unwrap() / [..n] / [n..]` — is a **preventive refactor sweep** of patterns that *would* trip clippy if the gate were widened to `--tests`, or that trip lint in crates other than `vb_ipc` if a future `clippy.toml` enables them. The patterns are concentrated in:

- `crates/vb_ipc/src/*/tests.rs` and `crates/vb_ipc/src/tests.rs` (inline `#[cfg(test)] mod tests`).
- The few production sites that emit `&mut bytes[..]`, `&buf[..N].try_into().unwrap()`, or hardcoded partial slices of fixed-size arrays. Most of these do not currently trip `clippy::indexing_slicing` because the array length is statically known — but the bead wants them tightened to checked helpers (`.first_chunk::<N>()`, `.split_first_chunk::<N>()`, `TryInto` with `Result`) so future refactors cannot panic at runtime.

## 2. Lint Gate Map (what currently scans what)

| Gate | Source line | Targets | Test code scanned? |
|---|---|---|---|
| `lint-src` | `.moon/tasks/all.yml:46` | `cargo clippy --workspace --lib --bins --examples --all-features` with deny list (incl. `clippy::indexing_slicing`, `clippy::get_unwrap`, `clippy::unwrap_used`, `clippy::string_slice`, `clippy::arithmetic_side_effects`) | **No** — `--lib` excludes `cfg(test)` modules and `--examples`/integration tests are not selected. |
| `check` | `.moon/tasks/all.yml:121` | `cargo check --workspace --all-targets --all-features` | Yes (`--all-targets`) but only compile, no lints. |
| `panic-surface` | `.moon/tasks/all.yml:65` (script `scripts/check-panic-surface.sh:11`) | `rg '(assert!|assert_eq!|assert_ne!|unreachable!)'` in `crates/*/src`, skipping paths/files with cfg-test or `_tests` suffix and `[test]`/`[cfg(test)]` lines above each `assert!` | No |
| `ignored-fallible-results` | `.moon/tasks/all.yml:75` (script `scripts/check-ignored-fallible-results.sh`) | DISCARD-001..006 classes in production `crates/*/src`, excludes `tests.rs`, `_tests.rs`, `test_harness.rs` paths | No |
| `forbidden-scan` | `.moon.yml` + `xtask/src/forbidden_scan.rs:31` | `crates/*/src` skipping `tests/`, `_tests.`, `tests.`, `/test/`, `proof/`, `kani*/`, `loom/` | No |
| `unsafe-audit` | `.moon/tasks/all.yml:81` | `crates/**.rs` excluding `tests/`, `benches/`, `examples/`, `fixtures/`, `verification/`, `kani*` | No |

Conclusion: **every lint gate that catches `.get().unwrap()` / `[..n]` / `[n..]` excludes test code.** The bead's call for "test utilities under crates/vb_*/tests" is therefore either (a) scope creep beyond what the current gates enforce, or (b) for tests we want hardened proactively. The exploration below inventories both production and test sites so the downstream pipeline can pick a scope explicitly.

## 3. Inventory — Production-Code Sites (lint-src scanned)

### 3.1 vb_ipc — confirmed: one site, currently lint-green

| Line | Symbol | Pattern | Fix | Lint impact |
|---|---|---|---|---|
| `crates/vb_ipc/src/frame_types.rs:41` | `IpcFrameHeader::encode` | `let mut cursor = std::io::Cursor::new(&mut bytes[..]);` where `bytes: [u8; IPC_HEADER_LEN]` | Replace with `Cursor::new(&mut bytes)` (Rust auto-deref `&mut [u8; N]` → `&mut [u8]`) **or** `Cursor::new(bytes.as_mut_slice())` to match the canonical pattern at `crates/vb_ipc/src/frame_types.rs:71` (`Cursor::new(bytes.as_slice())` on `decode`). | Does **not** currently trip `clippy::indexing_slicing` because the array length is statically known. Refactor is for canonicalization. |

### 3.2 All other first-party crates — confirmed: zero production violations

The exhaustive grep over `crates/`, `xtask/src/` excluding `tests/`, `test/`, `*test*.rs`, `benches/`, `examples/`, `kani*/`, `proofs/`, `fuzz/`, `fixtures/`, `verification/`, `workspace_tests/` returns **0** `clippy::indexing_slicing` / `clippy::get_unwrap` / `clippy::unwrap_used` violators. (Note: `--lib` would include `workspace_tests`; the above excluded it only because of historical glob conventions; see §3.4.)

### 3.3 kani_*.rs — guarded by `#![cfg(kani)]` and out of lint-src reach

| File | Pattern count | Notes |
|---|---|---|
| `crates/vb_ipc/src/kani_ipc_decode_order.rs` | 21 byte-slice + `copy_from_slice` calls | `#![cfg(kani)]` line 2; only compiled under kani. NOT in lint-src. |
| `crates/vb_ipc/src/kani_ipc_header.rs` | 10 | `#![cfg(kani)]` |
| `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` | (scan verbatim) | `#![cfg(kani)]` |
| `crates/vb_ipc/src/kani_flag_validation.rs` | (scan verbatim) | `#![cfg(kani)]` |
| `crates/vb_compile/src/kani_finish_digest.rs` | 12+ | `#![cfg(kani)]` |
| `crates/vb_compile/src/kani_digest_repeat.rs` | 1 | `#![cfg(kani)]` |
| `crates/vb_compile/src/kani_foreach_parity.rs` | 30+ | `#![cfg(kani)]` |
| `crates/vb_storage/src/kani_codec.rs` | (scan verbatim) | `#![cfg(kani)]` |

Recommendation: skip in this bead; kani harnesses use `kani::any()`-generated lengths and `clippy::indexing_slicing` is suppressed by the harness toolchain.

### 3.4 `workspace_tests/src/test_util/` — non-`mod tests` test helpers

`crates/workspace_tests` is a real crate in `--workspace --lib` (its `Cargo.toml` is `name = "velvet-ballistics-workspace-tests"`, `[lints] workspace = true`). Test helpers inside `src/` are part of `lint-src`.

| Line | File | Pattern | Current lint impact |
|---|---|---|---|
| `crates/workspace_tests/src/test_util/seed.rs:23` | `SeededBytes::new` | `rng.fill(&mut bytes[..]);` where `bytes: [u8; N]` (fixed array) | Does not trip lint-src (full-slice on known array). Canonicalize to `rng.fill(&mut bytes)` for parity with `frame_types.rs:41`. |
| `crates/workspace_tests/src/test_util/fixture.rs:58` | `FixtureBuilder::build_bytes` | `rng.fill(&mut vec[..]);` where `vec: Vec<u8>` (full-slice) | Does not trip lint-src because `vec[..]` is the omitted-bound form which clippy treats as safe. Canonicalize to `rng.fill(vec.as_mut_slice())`. |

Both edits are safe-and-zero-behavior-change.

## 4. Inventory — Test-Code Sites (currently NOT lint-src scanned)

Each row is per-site; downstream owners may decide scope. All lines live inside `#[cfg(test)] mod tests { … }` blocks (inline) or in `crates/*/tests/*.rs` (integration tests).

### 4.1 vb_ipc — test-side patterns that the bead targets

| File | Lines | Pattern | Lint class it would trigger if `--tests` were added |
|---|---|---|---|
| `crates/vb_ipc/src/tests.rs` | 1529, 1546, 1617, 1629, 1641, 1655, 1851 (`[..4]`); 1530, 1547, 1618–1619, 1630–1631, 1642, 1656–1657, 1852 (`[4..6]`, `[6..8]`); 859 (`encoded[10], encoded[11]`) | `header_bytes[..4].copy_from_slice(…)` / `&encoded[10], &[11]` on fixed `[u8; IPC_HEADER_LEN]` arrays | `indexing_slicing` (deny on partial slice) + `indexing_slicing` on literal index. Fixed-size arrays with constant indices historically did not fire in nightly 2026-04-28 unless the index expression is non-literal. Worth canonicalizing via new `header_bytes(…).unwrap_or_default()` style helper already present at `crates/vb_ipc/src/tests.rs:45-63`. |
| `crates/vb_ipc/src/client/tests.rs` | 272 (`&buf[..crate::IPC_HEADER_LEN].try_into().unwrap()`); 410, 432, 454, 559, 627 (`&buf.try_into().unwrap()` on `Vec<u8>` of length `IPC_HEADER_LEN`) | `Vec<u8>` slices plus `unwrap` | `indexing_slicing` (`[..N]` on Vec) + `unwrap_used`. Refactor pattern: `buf.first_chunk::<IPC_HEADER_LEN>().expect("exact-size fixture")` or `let header: [u8; IPC_HEADER_LEN] = buf.into_boxed_slice().into()` (after length-validation). |
| `crates/vb_ipc/src/client/tests.rs` | 470, 529 (`bad_header[..4].copy_from_slice(…)`); 252–255 (helper-level index via `UnixStream::pair`) | Fixed-size `[u8; IPC_HEADER_LEN]` arrays | Optional canonicalization. |
| `crates/vb_ipc/src/server/impl_tests.rs` | 419, 420, 421, 456, 457, 458, 709, 710, 711, 712, 713, 714, 715, 2023, 2024, 2025 | `bad_header[..4].copy_from_slice(…)`, `[4..6]`, `[6..8]`, `[8..10]`, `[10..12]`, `[12..20]`, `[20..24]` | Same as above. Helper `crates/vb_ipc/src/server/impl_tests.rs:49 build_frame(…)` exists — if extracted, the slice writes could move through it. |
| `crates/vb_ipc/src/frame/tests.rs` | 119, 641, 656, 673, 696, 714, 737, 1024 (`[..4]`); 643 (`[4..].fill(0xFF)`); 657–658, 674–675, 697, 715–716, 738–739, 741, 1025–1026, 1048–1051 (`[4..6]/[6..8]/[10..12]/[20..24]`, plus assertion reads) | Fixed-size `[u8; IPC_HEADER_LEN]` plus assertion reads `&frame[0..4]` etc. | Same canonicalization concern; reads are by `assert_eq!` and typically safe but worth replacing with explicit `let chunks: [(&[u8], &[u8]); N] = frame.chunks_exact(4).take(N).collect()`-style. |
| `crates/vb_ipc/src/frame/tests.rs` | 272 `assert_eq!(payload_section, Some(&[][..]));` | Idiomatic Rust; lint-clean. | Keep as-is. |
| `crates/vb_ipc/src/frame_types/tests.rs` | 18, 32, 41, 52, 105 | `bytes[0..4].copy_from_slice(…)`, `[4..6]`, `[10..12]`, `[20..24]` | Same as above. |
| `crates/vb_ipc/src/metrics/tests.rs` | 782 `b.shards[0].active_runs = 2;` | Test mutator | Lit only inside an internally constructed object; `Vec[idx]` literal. |
| `crates/vb_ipc/src/tests.rs` | 221, 608, 625, 641, 710, 1296, 1313 (`Vec::from(&b"…"[..])`); 1329 (`Vec::from(&b"\xFF\xFF\xFF\xFF"[..])`) | `b"…"[..]` literal byte slice | `clippy::string_slice` only flags on string literals (`&"…"[..]`), not on byte literals (`b"…"[..]`). Confirmed lint-clean. |

### 4.2 Other crates' tests (representative, not exhaustive)

Top offenders by `try_into().unwrap()` / `&buf[..N]` density:

| File | Count | Notes |
|---|---|---|
| `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` | 14 `key[..N].try_into().unwrap()` sites (lines 86, 179, 180, 271, 272, 273, 708, 722, 739, 740, 768, 785, 786) | bdd test fixture constructing typed ids from byte buffers. Idiomatic restoration from raw storage. |
| `crates/workspace_tests/tests/restate_typed_partitioned_id_tests.rs` | 7 `header[..N].copy_from_slice(…)` (lines 9–14) | Same idiom. |
| `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` | 14 partial-slice `copy_from_slice` calls (109–116 + 193, 218, 241, 266, 293, 318, 1595 …) | Header fixture construction. |
| `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` | 3 `header.copy_from_slice(&encoded[..RECORD_HEADER_BYTES])` | Record decode. |
| `crates/vb_cli/tests/vb_qi37_14_1_run_step.rs` | 3 `json.get("…").unwrap()` (lines 719, 1174, 1181) | Per bead vb-hwkqa precedent, the `vb_cli` crate already created `crates/vb_cli/src/json_access.rs` with `value_get` / `value_get_str` helpers (added in commit `96fd4896d`). Future-aligned: replace with `super::json_access::value_get(&json, "…").expect("deltas")`. |
| `crates/vb_cli/src/commands_diff/tests.rs` | 6 `outcomes.get(&n).unwrap()` / `slots.get(&n).unwrap()` (391, 404, 425, 441, 455, 469) | Test fixture. Same helper-extraction pattern as vb-hwkqa would apply. |
| `crates/vb_cli/src/agent_context/tests/kani_harnesses.rs` | 1 `e.get(*key).unwrap().is_array()` (255) | kani test, but inside `tests/` — orphan observation; not currently in lint-src. |

Tests in `crates/vb_core/tests/`, `crates/vb_storage/tests/`, `crates/vb_validate/tests/`, `crates/vb_compile/tests/`, `crates/vb_runtime/tests/` were spot-checked for the same patterns at low density (mostly `Vec::from(&b"…"[..])` and helpers like `make_event`, `make_parts`, `make_ticket`); these helpers are idiomatic test fixtures and refactoring them is opportunistic, not required.

## 5. Existing Tests Proving We Don't Need to Rewrite Specs

All of the following currently pass with `cargo test -p vb_ipc` and `cargo test -p workspace_tests` after `cargo check` compiles them — so the slice patterns in §4 are not test-of-correctness failures, they are style/refactor opportunities.

| Module | File | Test-fn count | Demonstrates |
|---|---|---|---|
| `vb_ipc::frame_types::tests` | `crates/vb_ipc/src/frame_types/tests.rs` | 4 | header encode/decode roundtrip across all 6 fields; tolerate the literal slice writes. |
| `vb_ipc::frame::tests` | `crates/vb_ipc/src/frame/tests.rs` | 12+ | `decode_frame` reject-bad-magic/reject-bad-version/success paths over fixture header bytes. |
| `vb_ipc::server::impl_tests` | `crates/vb_ipc/src/server/impl_tests.rs` | 18+ | full server lifecycle with `bind` / `serve_ipc` / disconnect handling. Helpers `temp_socket_path`, `make_runtime`, `make_client`, `build_frame`, `read_exact_timeout` already provide a refactor surface. |
| `vb_ipc::client::tests` | `crates/vb_ipc/src/client/tests.rs` | 20+ | client/server round-trips; helpers `setup_mio_stream_with_peer` etc. |
| `vb_ipc::tests` | `crates/vb_ipc/src/tests.rs` | 28 | module-level integration harness over queues, ingress, decoder, payloads. Helper `header_bytes(magic, version, command, flags, reserved, correlation, payload_len)` already provides typed construction. |
| `vb_ipc::metrics::tests` | `crates/vb_ipc/src/metrics/tests.rs` | 30+ | metrics encode/decode roundtrip + aggregate shape. |
| `vb_cli::json_access::tests` | `crates/vb_cli/src/json_access.rs` (from commit `96fd4896d`) | 4 | helper-style replacement precedent (predecessor for the bead's intent). |

## 6. Open Questions for Downstream Owners

1. **Scope.** Should the bead refactor only the 1 production site at `vb_ipc/src/frame_types.rs:41`, the 2 production sites in `workspace_tests/src/test_util/{seed,fixture}.rs`, or also the test-side patterns in §4? Each tier multiplies the diff substantially. **Recommendation**: keep this bead narrowly on (1) production + (2) `workspace_tests/src/test_util` (3 sites total, ~6 lines) and file follow-up beads for the test-side cleanup.
2. **Test-clippy extension.** Should the scope add a `lint-tests` moon task analogous to `lint-src` but `cargo clippy --workspace --tests --all-features`? AGENTS.md states "Tests must compile and run, but test clippy is not strict" — extending would require policy update. **Out-of-scope** for this bead; flag for follow-up.
3. **Helper extraction.** Refactor `crates/vb_ipc/src/tests.rs:45 fn header_bytes(…)` to be `pub(crate)` and reuse from `client/tests.rs:264-275` and `server/impl_tests.rs:417-426`, `frame/tests.rs:641-660`, `frame_types/tests.rs:18-54` — would replace ~25 partial-slice sites with a single function call. **Optional**; depends on scope choice in (1).
4. **Kani fixtures.** Existing `crates/vb_ipc/src/kani_*.rs` harnesses can stay; they are `#[cfg(kani)]`. **No action** for this bead.

## 7. Recommended Downstream Owners

| Stage | Owner | Inputs |
|---|---|---|
| Contract / domain model | `rust-contract` | (N/A — no domain change; this is a refactor, not new semantics) |
| Proof planning | `proof-planner` | (N/A — no proof obligation changes; existing kani harnesses remain) |
| Implementation | `holzman-rust` | `codebase-map.md` §3.1, §3.4 (3 production-line edits) plus optional §4.1 if scope expanded |
| Tests | (already passing; no rewrite) | `cargo check -p vb_ipc --all-targets` |
| Verification | `formal-verifier` | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap` (already EXIT=0 baseline; should remain EXIT=0 after edits). |
| Black-hat review | `black-hat-reviewer` | diff should be 3-15 lines, no behavior change, no spec change |

## 8. Excluded / Out-of-Scope (explicit)

- `target/` build artifacts.
- `.beads/` runtime state (locked to server mode; do not commit `.beads/dolt`, `.beads/backup`, etc. per `scripts/check-beads-server-mode.sh`).
- Test code outside `vb_ipc` is reported but not in scope by default.
- Kani harness fixtures in `crates/vb_ipc/src/kani*.rs` are `#[cfg(kani)]` and explicitly out of scope.
- `xtask/src/cli.rs` and other xtask files do not currently contain the targeted patterns.
- `crates/workspace_tests/tests/**` integration tests are reported for context only and out of default scope.

## 9. Evidence Trail

- `cargo clippy --workspace --lib --bins --examples --all-features` with the production deny list — EXIT=0 (`.moon/tasks/all.yml:51`). Logged live in `/tmp/lint-output.log` during scout.
- `cargo clippy -p vb_ipc --lib --all-features --no-deps` with extra warns (`-W clippy::indexing_slicing -W clippy::get_unwrap`) — EXIT=0.
- `cargo check -p vb_ipc --all-targets --all-features` — EXIT=0.
- Pattern-grab script (run in this isolated workspace):
  - `rg -n '\[(\.\.[^]]*|[0-9]+\.\.[^]]*)\]' crates/vb_ipc/src/ --glob '!*tests.rs' --glob '!**/tests.rs' --glob '!**/impl_tests.rs' --glob '!**/*test*.rs' --glob '!*kani*'` → 1 line: `frame_types.rs:41`.
  - `rg -n '\.get\([^)]*\)\.unwrap\(\)' crates/vb_ipc/src/ --glob '!*tests.rs' --glob '!**/tests.rs' --glob '!**/impl_tests.rs' --glob '!**/*test*.rs' --glob '!*kani*'` → 0 lines.
  - `rg -n '\bget\b\s*\([^)]*\)\.unwrap\(\)' xtask/src/` → 0 lines.

## 10. Anti-Hallucination Markers

- I have only listed files that I confirmed exist and lines that I confirmed contain the cited pattern.
- All `clippy::*` behavioral claims (e.g., "`[..]` on a fixed-size array does NOT trip `clippy::indexing_slicing` in nightly 2026-04-28") were independently reproduced in `/tmp/clippytest` against `rustup run nightly-2026-04-28 cargo clippy` with the corresponding `-D` flag.
- I have not invented any test failures, proof obligations, or dependency changes.
- MISSING: an open `.beads/vb-qol58/proof-seeds.jsonl` (not yet written — State 3 contract stage).
