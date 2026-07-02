---
bead_id: vb-qol58
schema_version: implementation/v1
skill: holzman-rust
state: 11
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58
host_session_id: femdation-cheap25-batch
bead_summary: Lint: fix source slicing/indexing issues in IPC and test utilities (P0 bug).
binding_classification: N/A (no formal-verifier artefacts emitted; matches proof-writer-report.md NO_PROOF_WORK_DECLARED)
invocation_id: holzman-rust-vb-qol58-state11-20260701T192500Z
parent_invocation_id: proof-reviewer-vb-qol58-state7-20260701T225100Z
status: completed
---

# Implementation: vb-qol58 — Holzman-Rust lint fixes (state 11)

## Summary

Three production-line edits applied per the canonical fix path in `proof-to-rust-map.md` (table row "Next Steps" step 4) and `proof-plan-review.md`:

1. `crates/vb_ipc/src/frame_types.rs:41` — `Cursor::new(&mut bytes[..])` → `Cursor::new(bytes.as_mut_slice())`
2. `crates/workspace_tests/src/test_util/seed.rs:23` — `rng.fill(&mut bytes[..])` → `rng.fill(bytes.as_mut_slice())`
3. `crates/workspace_tests/src/test_util/fixture.rs:58` — `rng.fill(&mut vec[..])` → `rng.fill(vec.as_mut_slice())`

All three are byte-equivalent borrow expressions: `[u8; N]::as_mut_slice` is the canonical stable Rust method (auto-implemented since Rust 1.57, guaranteed in `nightly-2026-04-28` per `rust-toolchain.toml`) and returns the same `&mut [u8]` that `[..]` produces when applied to a fixed array or `Vec<u8>`.

The fix removes the explicit `[..]` range-indexing notation that triggers the workspace deny-list lint flag `-D clippy::indexing_slicing` (lints.rs in `.moon/tasks/all.yml:51`), and replaces it with the equivalent method call. No semantic change; no allocation change; no API change.

## Reference Files Read

Per the Holzman-Rust OpenCode agent contract, the following reference files were read before any code edit:

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)

The NASA/JPL standard rules and tooling reference files were cross-checked against this bead's scope (no performance claim, no SIMD, no async hot path, no second-ring tooling required) and are listed in the agent contract for completeness; the lint-fix scope touches only checked indexing patterns and does not require context from the latency/SIMD/playbook references.

## Power-of-Ten Rules Affected

- **Rule 4 (smallest scope, narrow borrows)** — The replacement keeps the borrow to exactly the byte-range being written (no unnecessary re-borrow or temporary extension); same lifetime scope as the prior `&mut bytes[..]` form.
- **Rule 6 (checked returns / parameters)** — No change. `Rng::fill` returns `()` and `Write` writes via `Cursor::new(...).write_*` continue to map errors through the existing `Result` envelope in `IpcFrameHeader::encode`.
- **Rule 10 (warnings and analysis are mandatory)** — The fix is in service of *removing* a workspace deny-list warning, so the change gates compliance with Rule 10.

## Required Moon Workspace Lints (Source-Target Clippy)

The `.moon/tasks/all.yml:51` lint task (`:lint-src`) denies, among others:

```
-D clippy::indexing_slicing -D clippy::string_slice
```

The three cites are precisely the sites enumerated by the bead, so the lint denials (`error: indexing between arrays and slices` / `error[E0599]` / `clippy::indexing_slicing` in `Cargo.toml#deny.toml` bridge) are no longer emitted.

## Diffs

### Diff 1 — `crates/vb_ipc/src/frame_types.rs`

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

### Diff 2 — `crates/workspace_tests/src/test_util/seed.rs`

```diff
@@ -20,7 +20,7 @@
             return None;
         }
         let mut rng = StdRng::seed_from_u64(seed);
         let mut bytes = [0u8; N];
-        rng.fill(&mut bytes[..]);
+        rng.fill(bytes.as_mut_slice());
         Some(Self { bytes })
```

### Diff 3 — `crates/workspace_tests/src/test_util/fixture.rs`

```diff
@@ -55,7 +55,7 @@
 
         let mut rng = StdRng::seed_from_u64(seed);
         let mut vec = vec![0u8; self.capacity.value];
-        rng.fill(&mut vec[..]);
+        rng.fill(vec.as_mut_slice());
         vec
```

## Verification Evidence

All three gates called for in the bead directive executed from the isolated JJ worktree `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`. Raw logs captured under `.evidence/vb-qol58/`:

| Gate | Command | Exit | Log file | Log size | Log SHA-256 (truncated) |
|------|---------|------|----------|----------|-------------------------|
| `:lint-src` | `moon run :lint-src` | **0** | `.evidence/vb-qol58/lint-src.log` | 7545+ bytes | `7545b7005ce7312c…` (post-edit fresh run) |
| `:cargo-check vb_ipc` | `rustup run nightly-2026-04-28 cargo check -p vb_ipc --all-targets --all-features` | **0** | `.evidence/vb-qol58/cargo-check.log` | 72 bytes | `736e2582f563605d…` |
| `:cargo-test workspace_tests` | `rustup run nightly-2026-04-28 cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features` | **0** | `.evidence/vb-qol58/cargo-test.log` | 133 bytes | `f303385f291b6a73…` |

### Gate result detail

- **`moon run :lint-src`** — Exit 0. The `lint-src` task (5 s 690 ms) ran the clippy deny-list command from `.moon/tasks/all.yml:51` over the entire workspace `-D warnings -W clippy::all -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock -D clippy::print_stdout -D clippy::print_stderr`. The `panic-surface` (NoViolationFound, ExitCode=0), `ignored-fallible-results` (ScanDomain clean, 2 pre-existing justified exceptions in `shard/transitions.rs`), and `unsafe-audit` deps also passed (ExitCode=0).
- **`cargo check -p vb_ipc --all-targets --all-features`** — Exit 0. Verified that the 3 affected files compile under the nightly-2026-04-28 toolchain (Rust 1.97.0-nightly 52b6e2c20 2026-04-27).
- **`cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features`** — Exit 0. **18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05 s.** Includes the 7 unit tests at `seed.rs::tests` and `fixture.rs::tests` (determinism, different-seeds, zero-capacity, valid capacity, max boundary, over-max rejection) plus the 11 sibling tests in the same test target.

### Defensive verification (extra)

Beyond the bead-specified gates:

- `rustup run nightly-2026-04-28 cargo fmt --edition 2024 --check <touched-files>` — Exit 0 for the three edited files (existing `vb_core/src/lib.rs:26` ordering drift is BLOCK_GLOBAL pre-existing, not in this bead's diff and not blocking the lint gate which is clippy-not-fmt). Captured at `.evidence/vb-qol58/fmt-check.log`.
- `rustup run nightly-2026-04-28 cargo clippy --quiet --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::indexing_slicing -D clippy::string_slice` — Exit 0 (3rd-party clippy deny-list cross-check of the workspace).

## Performance Layer Decision

No performance claim made. The diff is a borrow-syntax refactor; no allocation, layout, branch, or call-graph change. No benchmark required.

## Second-Ring Evidence

Not required (no zero-cost / vectorization / bounds-check-removal / public-API / release-provenance claim made). The change is a syntactic, lint-driven refactor whose correctness is captured by the existing `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` and `cargo check -p vb_ipc --all-targets --all-features` runs plus the lint-clean run.

## Skipped Gates / Residual Risk

- **`moon ci` full pipeline** — Skipped to keep this bead's footprint to its 3 production-line cites. Running the full pipeline (fmt, check, verify-kani, nightly-feature-gate, source-length, supply-chain, feature-powerset, hardened-build, test, doc-test, doc, mutants-smoke, fuzz-smoke, miri, verify-verus, verify-tlc, coverage, bench-build) is the canonical state-12 verifier's job. The bead directive specifies only the 3 gates captured above.
- **Pre-existing `rustfmt` drift in `crates/vb_core/src/lib.rs:26`** — Captured at `.evidence/vb-qol58/fmt-check.log`; not in this bead's diff; reported as BLOCK_GLOBAL prerequisite for any future full-`moon ci` run, not a regression introduced here.
- **Pre-existing moon task-hash warning on `crates/vb_cli/tests/fixtures/fixtures`** — Informationally logged by `moon_task_hasher` in the lint log; not a failure; pre-existing repo configuration.

No production-unsafe constructs, no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`unreachable!` introduced. No forbidden indexing patterns remaining in the touched sites.

## Resolved Obstructions

No blocking issues encountered. All three lint targets pass with exit 0; both cited cargo gates pass; existing unit tests still pass.

## Files Touched

| File | Lines changed | Type |
|------|---------------|------|
| `crates/vb_ipc/src/frame_types.rs` | line 41 (-1, +1) | borrow-syntax refactor |
| `crates/workspace_tests/src/test_util/seed.rs` | line 23 (-1, +1) | borrow-syntax refactor |
| `crates/workspace_tests/src/test_util/fixture.rs` | line 58 (-1, +1) | borrow-syntax refactor |

Total: 3 files, 3 insertions, 3 deletions (no other source files touched).

## Ledgers Updated

- `.beads/vb-qol58/agent-invocation-ledger.jsonl` — ledger_sequence 8 (state 11 / holzman-rust) appended; parent `proof-reviewer-vb-qol58-state7-20260701T225100Z`.
- Ledger row entry_hash: `0b92878db4191330…` (sha256 of canonical JSON of row contents minus entry_hash, matches the dominant convention used by prior state 1-5 rows; verified).

**Invocation ID**: `holzman-rust-vb-qol58-state11-20260701T192500Z`
