# Black Hat Review: vb-qol58 — State 13 Attempt 1

**Bead**: vb-qol58 — Lint: fix source slicing/indexing issues in IPC and test utilities (P0 bug)  
**State**: 13  
**Reviewer**: black-hat-reviewer  
**Source checkout**: `/home/lewis/src/velvet-ballistics` (coord, untouched)  
**Isolated workspace**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`  
**Attempt**: 1  
**Reviewer invocation**: `black-hat-reviewer-vb-qol58-state13-20260701T225500Z`  
**Reviewed at**: 2026-07-01T22:55:00Z  
**Parent invocation**: `formal-verifier-vb-qol58-state12-20260701T225200Z`

## Gate Result

**STATUS: APPROVED** (zero findings; zero defects; all 5 phases PASS).

---

## PHASE 1: Contract & Bead Parity

The bead's design goal (per `delivery-scope.jsonl` rows 1, 2, 3 and `contract.md §C-1..§C-4`) is precisely captured by the 3 production-line edits.

| Requirement | Status | Evidence |
|-------------|--------|----------|
| `frame_types.rs:41` — `Cursor::new(&mut bytes[..])` → `Cursor::new(bytes.as_mut_slice())` | PASS | Live diff at `.evidence/vb-qol58/verifier/regression-diff.txt` (sha256 `901648b15ab4878864cb238896f0b7852ba3dbaf8ac0aaf2d6290bdc618f7aca`); `sed -n '41p'` returns exact post-edit content; live `rg '\[\.\.\]'` returns no matches in this file |
| `seed.rs:23` — `rng.fill(&mut bytes[..])` → `rng.fill(bytes.as_mut_slice())` | PASS | Live `sed -n '23p'` confirms; ripgrep confirms zero `&mut bytes[..]` remnants |
| `fixture.rs:58` — `rng.fill(&mut vec[..])` → `rng.fill(vec.as_mut_slice())` | PASS | Live `sed -n '58p'` confirms; ripgrep confirms zero `&mut vec[..]` remnants |
| `.moon/tasks/all.yml:51` deny-list byte-identical pre/post | PASS | `sha256sum(jj file show -r @- .moon/tasks/all.yml)` == `sha256sum(jj file show -r @ .moon/tasks/all.yml)` == `423e84fa22c28ad863a089a7e4ae2c6dfce4ae827f5db0d2cea991fca1f6134d` |
| 7 IPC encode sites (lines 42-62) byte-identical pre/post | PASS | `proof-findings.jsonl` row 4 `FIND-qol58-PRODUCTION_CITATIONS_VERIFIED` (severity: observation; disposition: fixed_with_evidence); `error-taxonomy.md §1.1` enumerate the 7 `IpcError::HeaderEncodeFailed` mapping sites |
| Bead scope: 3 production-line edits, source-lint + cargo-check + cargo-test lanes only | PASS | `proof-plan-review.md` and `proof-writer-report.md NO_PROOF_WORK_DECLARED`; 23 verifier-lane-decisions (5 `required: proptest`, 18 `not_applicable`); zero Verus/Kani/Flux/Loom/Miri/proptest-property harnesses emitted |
| Behavior parity (test result equality) | PASS | `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` → 18 passed; 0 failed; 0 ignored (regression diff in `.evidence/vb-qol58/verifier/cargo-test.log`, sha256 `bd577d55f236b941832cfce54c469379addf9726f39f5d442594892b2ea25b79`) |
| Toolchain compatibility (nightly-2026-04-28) | PASS | `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)` accepted the syntax; same toolchain as `.rust-toolchain.toml` workspace pin |
| **VACUUM Verus proof check** | PASS | `scripts/check-verus-production-binding.sh` exit 2 ("verification/verus does not exist"); per the skill rule, this is the canonical "no Verus spec to validate → no VACUUM risk by construction" disposition. **No VACUUM-bucket spec file exists.** |
| **Production-inner drift check** | PASS | `scripts/check-production-inner-drift.sh` exit 128 (workspace-tooling noise from `git rev-parse --show-toplevel` on a JJ-only workspace; not a drift finding); re-derived via `diff(1) <(jj file show -r @-) <(jj file show -r @)` confirms zero drift at the 3 cites; no `production_inner/*` mirror exists (none required) |

The contract enforceability check passes: `IpcError::HeaderEncodeFailed` continues to map at every one of the 7 cursor-write sites at lines 44, 47, 50, 53, 56, 59, 62; no precondition/postcondition was weakened by the refactor. Test parity per `martin-fowler-tests.md` (boundary, determinism, capacity rejection) is preserved by the 18 passing tests.

**Verdict: PASS.** Contract is preserved; bead scope is exact; production-binding discipline auto-satisfied by lane omission (no Verus obligations in `proof-obligations.planned.jsonl`).

---

## PHASE 2: Farley Engineering Rigor

**Hard constraint: function size ≤25 lines.**

| Function | File | Lines | Limit | Status |
|----------|------|-------|-------|--------|
| `IpcFrameHeader::encode` | `crates/vb_ipc/src/frame_types.rs:39-64` | 26 | 25 | ⚠️ pre-existing drift (not introduced by this bead) |
| `IpcFrameHeader::new` | `crates/vb_ipc/src/frame_types.rs:29-36` | 8 | 25 | PASS |
| `SeededBytes::<N>::new` | `crates/workspace_tests/src/test_util/seed.rs:17-25` | 9 | 25 | PASS |
| `FixtureBuilder::with_capacity` | `crates/workspace_tests/src/test_util/fixture.rs:47-49` | 3 | 25 | PASS |
| `FixtureCapacity::new` | `crates/workspace_tests/src/test_util/fixture.rs:19-33` | 15 | 25 | PASS |
| `FixtureBuilder::build_bytes` | `crates/workspace_tests/src/test_util/fixture.rs:52-60` | 9 | 25 | PASS |

**Note on `IpcFrameHeader::encode`:** this function is **26 lines, exceeding the 25-line limit by 1**. This drift is **pre-existing** (per `codebase-map.md §3.1`, the function body has been 26 lines since before the lint-fix bead was opened). The current bead touches only **line 41** (a borrow-syntax refactor inside the function body) and does not introduce, extend, or alter the function's line count. The drift is logged as a known pre-existing size-limit violation, **NOT** a defect introduced by vb-qol58. It is a candidate for future decomposition (e.g., extract the 7 cursor writes into a `write_header_words(&mut Cursor<&mut [u8]>)` helper) but is **out of scope** for this bead. The current bead is **closed** for size compliance by Farley's "do not introduce a new violation" rule.

**Hard constraint: function parameters ≤5.**

| Function | Parameters | Limit | Status |
|----------|------------|-------|--------|
| `IpcFrameHeader::encode` | 0 (self) | 5 | PASS |
| `IpcFrameHeader::new` | 4 (command, flags, correlation, payload_len) | 5 | PASS |
| `SeededBytes::<N>::new` | 1 (seed) | 5 | PASS |
| `FixtureBuilder::with_capacity` | 1 (cap: FixtureCapacity) | 5 | PASS |
| `FixtureCapacity::new` | 1 (cap) | 5 | PASS |
| `FixtureBuilder::build_bytes` | 2 (self, seed) | 5 | PASS |

**Pure-vs-IO separation:** preserved verbatim (no new I/O surface; `Cursor::new(bytes.as_mut_slice())` is identical to `Cursor::new(&mut bytes[..])` in terms of I/O coupling). The pre-existing functional-core/imperative-shell boundary is untouched.

**Test design (behavior vs implementation):** the 7 unit tests named in `PO-qol58-003` `domain_claim` continue to assert behavior:

- `seeded_bytes_determinism` — asserts byte-equality of two same-seed outputs (behavior)
- `seeded_bytes_different_seeds` — asserts byte-inequality of two different-seed outputs (behavior)
- `seeded_bytes_zero_capacity` — asserts `SeededBytes::<0>::new(42)` returns `None` (behavior)
- `zero_capacity_rejected` — asserts `FixtureCapacity::new(0)` returns `Err` (behavior)
- `valid_capacity_accepted` — asserts `FixtureCapacity::new(100)` returns `Ok` (behavior)
- `max_capacity_boundary` — asserts `FixtureCapacity::new(MAX)` returns `Ok` (boundary behavior)
- `over_max_capacity_rejected` — asserts `FixtureCapacity::new(MAX+1)` returns `Err` (boundary behavior)

None of these tests asserts **how** the RNG or slice is invoked (no `assert!(cursor.position() == ...)` or similar implementation-detail assertions); they assert what the function returns. **PASS.**

**Verdict: PASS.** No new size/parameter violations introduced by the 3 touched sites. The pre-existing 26-line function drift is logged but out of scope. Test assertions remain behavior-level.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status | Evidence |
|------|--------|----------|
| **Rule 1: Zero `unsafe`** | PASS | Workspace crates `#![forbid(unsafe_code)]` (`vb_ipc`, `workspace_tests`); no `unsafe` introduced |
| **Rule 2: Zero `.unwrap()`/`.expect()`** | PASS | `rg '\.unwrap\(\)|\.expect\('` in the 3 touched files returns no matches at line 41/23/58. Pre-existing `unwrap()` calls in the test bodies (e.g., `seeded_bytes_determinism` line 34) are **test code**, not production code (`#[cfg(test)] mod tests`); per AGENTS.md, test code may use `.unwrap()` for assertion-of-failure-on-bug |
| **Rule 3: Zero `panic!`/`todo!`/`dbg!`** | PASS | `rg 'panic!\|todo!\|dbg!\|unimplemented!'` in production sites returns zero matches at the touched lines |
| **Rule 4: Narrow borrows** | PASS | `[u8; N]::as_mut_slice` and `Vec<u8>::as_mut_slice` both return the smallest `&mut [u8]` that covers the full slice — **same borrow scope** as `[..]` (which is the maximum-borrow-by-default in Rust) |
| **Rule 5: Bounded loops / no unbounded iteration** | PASS | No new loops introduced; `IPC_HEADER_LEN = 24` (compile-time constant), `[u8; N]` for `N: usize` const generic, `Vec<u8>` with `FixtureCapacity::MAX_CAPACITY = 1 MiB` upper bound — all bounded by construction |
| **Rule 6: Checked returns** | PASS | `rng.fill(...)` returns `()` (no Result); the IPC encode path's 7 cursor writes continue to map `Err(_)` → `IpcError::HeaderEncodeFailed` via `?` (lines 44, 47, 50, 53, 56, 59, 62 — all byte-identical pre/post). `SeededBytes::<N>::new` returns `Option<SeededBytes<N>>` and the `N == 0` short-circuit at lines 18-20 is preserved verbatim |

**Bonus: Parse-don't-validate.** The borrow expression `bytes.as_mut_slice()` is a *parsing* operation (`[u8; N]` → `&mut [u8]`) rather than a *validation* operation. Same applies to `vec.as_mut_slice()` for `Vec<u8>`. No validation logic touched.

**Bonus: Types-as-documentation.** The method call `bytes.as_mut_slice()` is a more explicit, self-documenting borrow expression than `&mut bytes[..]`. The refactor improves readability at the type level without changing semantics.

**Verdict: PASS.** All 6 Holzman rules satisfied; no `unsafe`/`.unwrap()`/`.expect()`/`.panic!`/`.todo!`/`.dbg!` introduced.

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status | Evidence |
|-------|--------|----------|
| **No Option-based state machines** | PASS | The pre-existing `Option<SeededBytes<N>>` is not a state machine; it is a "valid-or-empty" witness for the `N == 0` case. The refactor preserves this verbatim |
| **CUPID — Composable** | PASS | `as_mut_slice` returns the canonical `&mut [u8]` slice type, which is composable with all slice-handling APIs (`Cursor::new`, `rng.fill`, `Write::write_*`, etc.) |
| **CUPID — Unix-philosophy** | PASS | The change replaces one method-like indexing form with another method-like method form; the spirit of "do one thing" is preserved |
| **CUPID — Predictable** | PASS | `as_mut_slice()` is a stable Rust method since 1.57, available on every `[T; N]` and `Vec<T>`; behavior is identical to `[..]` for these types |
| **CUPID — Idiomatic** | PASS | The Rust community's official lint guidance (`clippy::indexing_slicing` in the workspace deny-list; `clippy_lints::INDEXING_SLICING` documentation) explicitly recommends `.as_slice()` / `.as_mut_slice()` over `[..]` |
| **CUPID — Domain-based** | PASS | The change does not alter the domain model; `IPC_HEADER_LEN`, `FixtureCapacity::MAX_CAPACITY`, and `SeededBytes::<N>` remain unchanged |
| **No clever abstractions** | PASS | This is the **anti-clever** change: the only modification is to use the canonical method call. No new traits, no newtype wrappers, no hidden async shims, no complex type machinery |
| **No "future-use" code (YAGNI)** | PASS | Nothing built for hypothetical future requirements; the change exactly targets the deny-list lint failure at the 3 cited sites |
| **No `Option` chains that should be `Result`** | PASS | The `N == 0` early-return is **`None`**, not `Err` — semantically correct because `SeededBytes::<0>` cannot exist (the byte array is empty); `None` is the canonical "no instance" sentinel, not a fallback error |

**Verdict: PASS.** The change is the paragon of "no new abstractions; pure canonical-verb substitution."

---

## PHASE 5: The Bitter Truth

**Clinical assessment:**

This is the cleanest possible `indexing_slicing` lint fix. The author did not:

- Introduce a new helper module to "wrap" the slice operation (no `slice_utils.rs`).
- Add a generic `AsByteSlice` trait bound (no extension-trait cleverness).
- Allocate a temporary `Vec<u8>` to copy the bytes (no performance regression).
- Add documentation comments asserting why the change is correct (the method name speaks for itself; adding a doc-comment would be paternalistic).
- Reformat surrounding code (the diff is exactly 1 line per touched file, byte-equivalent apart from the borrow substitution).
- Add a `// SAFETY:` comment (no unsafe, no comment needed).
- "Future-proof" the function with a const generic parameter when `[u8; N]` was already const-generic.
- Refactor unrelated code "while we're here" (the 3 diffs are minimal-touch).
- Add `--explain` notes to the deny-list (the deny-list is byte-identical; the lint explanation comes from `clippy`'s built-in `--explain`).

The 3 production-line edits are exactly what the workspace deny-list lint demands — no more, no less. The byte-equivalent borrow expression is the canonical stable Rust method documented since 1.57, guaranteed in the pinned `nightly-2026-04-28` toolchain.

**Did the author try to prove how smart they are?** No. The change is junior-readable. A first-year Rust programmer can audit the diff in 5 seconds.

**Is there hidden cleverness in the deny-list?** No. `bash scripts/check-*.sh` was run during state-12 formal verification and confirmed the deny-list at `.moon/tasks/all.yml:51` is byte-identical pre/post (SHA-256 `423e84fa22c28ad863a089a7e4ae2c6dfce4ae827f5db0d2cea991fca1f6134d`). No flag was weakened, removed, or rephrased.

**Is the test suite fraudulent?** No. `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` reports 18 passed (raw log at `.evidence/vb-qol58/verifier/cargo-test.log`, sha256 `bd577d55f236b941832cfce54c469379addf9726f39f5d442594892b2ea25b79`). All 7 tests named in `PO-qol58-003` `domain_claim` are live in the test surface and asserted behavior-level (not implementation-detail).

**Is the cargo check fraudulently silent?** No. `--quiet` cache hit produces 0-byte output (raw log sha256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` = canonical-empty). Exit status 0 is the truth that `cargo check` returned no warnings under `-D warnings` and no errors. The cached compile is real; the rustc toolchain is real; the package boundary is real.

**Verdict: PASS.** The change is uncompromisingly boring. The author resisted every temptation to add clever scaffolding. This is the right way to fix a P0 clippy lint.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| (none) | — | — | — |

`defects.md` is **empty** (zero defects; zero findings).

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `moon run :lint-src` | PASS | Exit 0; raw log `.evidence/vb-qol58/verifier/lint-src.log` (sha256 `59abb44a322e16f118956bda5cb9c798a2b2d8f8582a9157a93999700ca90b33`); 4 sub-tasks (`unsafe-audit`, `ignored-fallible-results`, `panic-surface`, `lint-src`) all green; deny-list at `.moon/tasks/all.yml:51` byte-identical |
| `cargo check -p vb_ipc --all-targets --all-features` | PASS | Exit 0; raw log `.evidence/vb-qol58/verifier/cargo-check.log` (sha256 canonical-empty, expected for `--quiet` cache hit); 7 IPC encode sites byte-identical pre/post |
| `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` | PASS | Exit 0; 18 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out (≥18 threshold); raw log `.evidence/vb-qol58/verifier/cargo-test.log` (sha256 `bd577d55f236b941832cfce54c469379addf9726f39f5d442594892b2ea25b79`); 7 named tests + 11 sibling tests |
| `scripts/check-verus-production-binding.sh` | PASS (N/A) | Exit 2; no Verus spec exists; no VACUUM risk by construction |
| `scripts/check-production-inner-drift.sh` | PASS (N/A) | Exit 128 (workspace-tooling noise); no `production_inner/*` mirror exists; live `diff(1)` confirms zero drift |
| Live ripgrep: `[\.\.]` in touched files | PASS | 0 matches |
| Live ripgrep: `as_mut_slice` in touched files | PASS | 3 matches at exactly the cited lines (frame_types.rs:41, seed.rs:23, fixture.rs:58) |

---

## Pre-Existing Out-of-Scope Items (logged, not blocking)

These are pre-existing items at the repo level — not introduced by vb-qol58 — and are NOT defects attributable to this bead:

1. **`IpcFrameHeader::encode` is 26 lines** (1 over the 25-line Farley limit). Pre-existing. **Out of scope** for this bead; candidate for future refactor to a helper function (e.g., `write_header_words`). The bead touches only line 41 inside this function and does not alter the line count.
2. **`crates/vb_core/src/lib.rs:26` rustfmt drift** (pre-existing; non-touching site; not in this bead's 3-line diff). Logged as `BLOCK_GLOBAL` per holzman-rust state 11 transcript.
3. **`crates/vb_runtime/src/shard/transitions.rs` DISCARD-006 justified exceptions** at lines 199 and 86 (pre-existing; explicit in `scripts/ignored-fallible-results.allow`; accepted by the `:lint-src` sub-tasks).
4. **`moon_task_hasher` warnings on `crates/vb_cli/tests/fixtures/fixtures`** (pre-existing tooling noise; non-failure).

None of these was introduced by vb-qol58; the bead's 3-line diff does not interact with any of them.

---

## Verdict

**STATUS: APPROVED.**

### Summary

The bead delivers exactly what was promised: a 3-line canonical-verb spelling change that resolves the `-D clippy::indexing_slicing` lint failure at the 3 cited sites (`frame_types.rs:41`, `seed.rs:23`, `fixture.rs:58`) without altering any production behavior, signature, allocation, or API. The deny-list at `.moon/tasks/all.yml:51` is byte-identical pre/post (SHA-256 `423e84fa22c28ad863a089a7e4ae2c6dfce4ae827f5db0d2cea991fca1f6134d`); the 18-test suite passes; the workspace lint gate stays green. No `unsafe`/`.unwrap()`/`.expect()`/`.panic!()`/`.todo!()`/`.dbg!()` introduced. No clever abstractions. No "future-use" code. Zero findings, zero defects, all 5 review phases PASS, all 7 quality gates PASS.

---

## Required Repair Actions (if REJECTED)

None. The bead is **APPROVED** as-is.

---

## Reviewer Provenance

- **Reviewer Skill**: `black-hat-reviewer`
- **Reviewer Invocation**: `black-hat-reviewer-vb-qol58-state13-20260701T225500Z`
- **Parent Invocation**: `formal-verifier-vb-qol58-state12-20260701T225200Z`
- **Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`
- **JJ workspace**: `cheap25-vb-qol58` (isolated; no Git toplevel, no coord contamination)
- **Workspace anti-hallucination checks**: `pwd -P` and `jj root` both resolve to the isolated path; `jj status` confirms 3 working-copy modifications matching `implementation.md` and `regression-diff.md`; coord checkout `/home/lewis/src/velvet-ballistics` confirmed clean.
- **Inputs read**: `formal-verification-report.md`, `verification-ledger.jsonl`, `proof-test-source-alignment.{jsonl,md}`, `regression-diff.md`, `proof-plan-review.md`, `proof-review.md`, `proof-to-rust-review.md`, `proof-writer-report.md`, `implementation.md`, `proof-obligations.planned.jsonl`, `verifier-lane-decisions.jsonl`, `proof-findings.jsonl`, `codebase-map.md`, `error-taxonomy.md`, `type-contracts.md`, `boundary-map.md`, `delivery-scope.jsonl`, `contract.md`, `.moon/tasks/all.yml`.
- **Live verification commands**: `rg`, `sed -n`, `jj file show`, `diff(1)`, `moon run :lint-src`, `bash scripts/check-verus-production-binding.sh`, `bash scripts/check-production-inner-drift.sh`. All captured at `.evidence/vb-qol58/verifier/` and re-readable.
- **Reviewed-artifacts-existed-before-start**: true.

**Status: APPROVED. Bead ready for State 14 (evidence-packaging + truth-serum).**

STATUS: APPROVED
