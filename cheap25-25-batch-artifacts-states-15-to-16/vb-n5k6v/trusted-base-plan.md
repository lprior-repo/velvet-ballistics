# Trusted Base Plan — vb-n5k6v

This plan enumerates the **trusted surfaces** (external
crates, stdlib APIs, build-system behaviors, and project
hygiene invariants) that the proof obligations in
`proof-obligations.planned.jsonl` rely on, the **model
reductions and assumptions** that the obligations make, the
**known stub boundaries** that downstream agents must
respect, and the **non-behavior waivers** (none in this bead).

The wire is a 3-line `#[cfg(test)] #[path = "..."] mod ...;`
declaration at `crates/vb_storage/src/lib.rs:182`. The
trusted base is therefore dominated by Rust stable build
graph semantics, the Fjall LSM-tree keyspace, the
`tempfile` crate, the `proptest` framework, and the
project's source-length exception ledger.

---

## 1. Trusted surfaces — Rust stable build graph

The wire is a module declaration. The trusted surfaces are:

- **Rust 2021 edition** module resolution:
  `#[cfg(test)] #[path = "<file>"] mod <name>;` correctly
  registers the file at `<file>` as the module `<name>` for
  `cargo test` builds. This is a stable, well-tested
  language feature; no nightly surface is required.

- **Cargo test harness discovery**:
  `cargo test -p vb_storage --lib` discovers and runs all
  `#[test]` fns in every `#[cfg(test)] mod` of the
  `vb_storage` crate. This is a stable cargo feature; the
  16 sibling declarations at `lib.rs:118-181` are the
  precedent and are clippy-clean.

- **Cargo lib-test tally**:
  `cargo test -p vb_storage --lib 2>&1 | tail -5` reports
  `test result: ok. N passed; 0 failed; 0 ignored; 0 measured;
  ...` where N is the count of `#[test]` fns. The pre-wire
  baseline is N=1530 (verified at 2026-07-01 by
  `PROPTEST_CASES=1 cargo test -p vb_storage --lib 2>&1 | tail -3`
  from the isolated workdir; the historical May 2026 captures
  at `.beads/vb-2bok/qa-report.md:5` and
  `.beads/vb-core-atomic-admission/STATE.md:1349` reported 924
  and are the `historic_2026_05_baseline`); the post-wire tally
  is N=1556 (1530 + 26).

- **Cargo check (build-only)**:
  `cargo check -p vb_storage --tests` compiles the test
  build without running tests. This is the static
  module-resolution check that fails if the `#[path]` or
  `mod` name is malformed.

- **Cargo clippy**:
  `cargo clippy -p vb_storage --tests -- -D warnings`
  enforces the lint-clean baseline. The 16 sibling
  declarations are lint-clean; the new declaration
  follows the same shape.

- **rust-toolchain.toml pin**:
  `rust-toolchain.toml` pins the nightly toolchain used
  for all cargo commands; the version is consistent
  across the workspace.

**Justification:** All five surfaces are stable Rust
language features or standard cargo subcommands. No nightly
or experimental surface is required for this bead.

## 2. Trusted surfaces — Fjall keyspace isolation

The 11 persistence tests and 4 concurrent tests exercise
the Fjall LSM-tree keyspace via `FjallJournal`. The trusted
surfaces are:

- **Fjall LSM-tree keyspace**:
  `FjallJournal::open(path, None)` creates a fresh keyspace
  in the given directory; the keyspace is automatically
  torn down on drop (via `Drop` impl). Per-test
  `tempfile::tempdir()` isolation ensures no cross-test
  contamination (verified at `edge_case_tests.rs` lines 30,
  77, 244, 311, 354, 397, 438).

- **Fjall `fail_next_persist_for_test` hook**:
  `FjallJournal::fail_next_persist_for_test()` (defined at
  `journal/core.rs:227`, `pub(crate)`) is a test-only seam
  that causes the next `persist_strict` call to return
  `Err(JournalError::StrictDurabilityFailed)`. This is the
  only `pub(crate)` test hook in the production crate, and
  it is safe because it is gated by `#[cfg(test)]` callers.

- **JournalWriterQueue serialization**:
  `JournalWriterQueue::state` is `Mutex<InnerState>`
  (verified at `queue/writer.rs:33`). All mutations to
  the queue state go through this `Mutex`, so the queue
  state machine is implicitly serialized under
  default-Rust threading.

- **FjallJournal append paths are `&self`**:
  `FjallJournal::append_journaled`, `append_strict`, and
  `persist_strict` (defined at `journal/append.rs:7,35,81`)
  all take `&self`. The interior mutability is provided by
  Fjall's internal locks, so multi-threaded `Arc<FjallJournal>`
  access is safe by default (verified at
  `journal/tests.rs:2598+` and `recovery/tests.rs`).

**Justification:** Fjall is a third-party LSM-tree crate
maintained under the `velvet-ballistics` workspace; the
internal locking model is the same as in
`journal/tests.rs:2598+` and `recovery/tests.rs`, which
use default-Rust threading without a Loom lane. The
`fail_next_persist_for_test` hook is the canonical
test-only seam for disk-full simulation.

## 3. Source-length exception ledger

The file `crates/vb_storage/src/edge_case_tests.rs` (637
lines) is on the project's source-length exception ledger
at `.config/source-length-exceptions.txt:150`:

```
crates/vb_storage/src/edge_case_tests.rs|lewis|vb-jpq7.47|split-or-retire-before-release|Pre-existing over-300-line Rust source baseline (637 lines); must be split by domain responsibility or retired before removing exception.
```

The trusted surfaces are:

- **Exception ownership**:
  The entry's owner is `lewis`; the removal plan is
  `vb-jpq7.47` (split-or-retire-before-release); the removal
  action is `split-or-retire-before-release`. The wire does
  not touch the exception entry; the entry is byte-identical
  pre/post wire.

- **Splitting plan**:
  Splitting the 637-line file into 7 topic buckets (disk
  full, concurrent, very large, open/close, record boundary,
  batch, queue) is tracked by bead `vb-jpq7.47`. The wire
  does not split the file; the file remains at 637 lines
  post-wire (CC-WIRE-006, CC-WIRE-007).

- **Project 300-line rule**:
  `scripts/check-source-length.sh` enforces the project's
  under-300-line source rule. Files on the exception
  ledger are exempted; the wire does not add a new file
  to the ledger.

**Justification:** The exception ledger is the project's
canonical mechanism for handling files that exceed the
300-line rule. The wire respects the exception; the
splitting plan is tracked separately by `vb-jpq7.47`.

## 4. Trusted surfaces — `tempfile::tempdir()` isolation

The 11 persistence tests and 3 open/close tests use
`tempfile::tempdir()` to create a per-test keyspace. The
trusted surfaces are:

- **`tempfile::tempdir()` semantics**:
  Each call creates a unique OS-managed temporary directory
  and returns a `tempfile::TempDir` guard. The directory
  is automatically removed on `Drop`. The tests keep the
  `TempDir` alive for the duration of the test (verified at
  `edge_case_tests.rs` lines 30, 77, 244, 311, 354, 397, 438).

- **Per-test isolation**:
  No two tests share a `TempDir`; each test creates its
  own keyspace in a fresh directory. The tests do not
  share Fjall state across tests.

- **CI disk sensitivity**:
  Each test creates and tears down its own keyspace; CI
  disk usage is bounded by the maximum test concurrency
  (default: number of CPU cores). Per-test keyspace size
  is small (single-digit MB at most).

**Justification:** `tempfile::tempdir()` is a well-tested
standard Rust pattern; the existing `journal/tests.rs`,
`recovery/tests.rs`, and `queue/tests.rs` use the same
pattern. CI disk sensitivity is low because each test
is self-contained.

## 5. Trusted surfaces — cargo test runner

The verification gate is `cargo test -p vb_storage --lib
edge_case 2>&1 | tail -30` and the tally is
`cargo test -p vb_storage --lib 2>&1 | tail -5`. The
trusted surfaces are:

- **Test harness**:
  cargo's built-in libtest harness, which is the same
  harness used by all 16 sibling `#[path = "..."] mod`
  declarations. The harness is a stable Rust feature.

- **Tally format**:
  The tally line `test result: ok. N passed; 0 failed;
  0 ignored; 0 measured; 0 filtered out; finished in
  <duration>s` is the standard cargo test output. The
  pre-wire baseline of N=1530 is established by direct
  execution at 2026-07-01 (`PROPTEST_CASES=1 cargo test
  -p vb_storage --lib 2>&1 | tail -3` from the isolated
  workdir); the historical May 2026 captures at
  `.beads/vb-2bok/qa-report.md:5` and
  `.beads/vb-core-atomic-admission/STATE.md:1349` (both 924)
  are the `historic_2026_05_baseline` and are NOT the
  current pre-wire value.

- **PROPTEST_CASES env var**:
  For proptest, `PROPTEST_CASES` controls the number of
  generated cases per `proptest!` block. The 26 tests
  in `edge_case_tests.rs` are **not** proptest strategies
  (they are concrete-value tests); `PROPTEST_CASES=1`
  in the obligation command is a no-op for these tests
  but satisfies the proof-planner verifier-specific check.

**Justification:** cargo test is the standard Rust test
runner; libtest is the standard test harness. The tally
format is stable across cargo versions.

## 6. Pre-wire baseline evidence

| Metric | Value | Source |
|---|---|---|
| Pre-wire `cargo test -p vb_storage --lib` tally (current) | 1530 | `PROPTEST_CASES=1 cargo test -p vb_storage --lib 2>&1 | tail -3` from isolated workdir on 2026-07-01 |
| Pre-wire `cargo test -p vb_storage --lib` tally (`historic_2026_05_baseline`) | 924 | `.beads/vb-2bok/qa-report.md:5` |
| Pre-wire `cargo test -p vb_storage --lib` tally (`historic_2026_05_baseline`, alt) | 924 | `.beads/vb-core-atomic-admission/STATE.md:1349` |
| `edge_case_tests.rs` line count | 637 | `rtk wc -l` (current) |
| `edge_case_tests.rs` test fn count | 26 | `rtk rg -n "^    fn \|^fn "` (current) |
| Sibling declarations in `lib.rs:118-181` | 16 | `rtk rg` (current) |
| Pre-wire `lib.rs` line count | 246 | `rtk wc -l crates/vb_storage/src/lib.rs` |
| Pre-wire `crates/vb_storage/Cargo.toml` line count | 32 | `rtk wc -l` |
| Dev-deps in `crates/vb_storage/Cargo.toml` | `tempfile`, `proptest` | `Cargo.toml:19-21` |

## 7. Diff-hygiene boundary

The wire's blast radius is bounded by:

- **1 file changed**: `crates/vb_storage/src/lib.rs`.
- **3 insertions**: the 3 lines
  `#[cfg(test)] #[path = "edge_case_tests.rs"] mod edge_case_tests;`
  at `lib.rs:182`.
- **0 deletions**: no lines removed from any file.
- **0 modifications to other lines**: every other line
  in `lib.rs` and every other file in the workspace is
  byte-identical pre/post wire.

The verification gate for this boundary is
`git diff --stat` (CC-WIRE-002). The trusted surfaces
are:

- **`git diff --stat` output format**:
  `git diff --stat` shows `<file> | <insertions> (+)
  <deletions> (-)`. The wire produces 1 file with +3/-0.

- **No accidental whitespace or trailing-newline changes**:
  The 3-line insertion is at `lib.rs:182` (after
  `snapshot_tests` at line 180-181 and before
  `pub mod queue;` at line 183). The empty line 182
  already exists in the pre-bead baseline; the 3
  inserted lines are placed at the empty line position,
  shifting subsequent lines down by 3.

**Justification:** `git diff --stat` is a stable git
feature; the wire's blast radius is verified by the
1-file / +3 / -0 invariant.

## 8. Cross-crate stability boundary

The wire does not touch any crate other than `vb_storage`.
The verification gate for this boundary is
`cargo check --workspace` (CC-WIRE-003). The trusted
surfaces are:

- **`cargo check --workspace`**:
  Compiles every crate in the workspace without running
  tests. The wire's 3-line insertion is in `vb_storage`;
  the other 8 crates (`vb_core`, `vb_runtime`, `vb_cli`,
  `vb_compile`, `vb_ipc`, `vb_queue_semantics`,
  `vb_validate`, `workspace_tests`) are unaffected.

- **Public API stability**:
  The wire is a `#[cfg(test)]` mod declaration; the
  `edge_case_tests` module is private to `vb_storage`
  and not exposed in any public API. The wire does not
  change the public API of `vb_storage` or any other
  crate.

**Justification:** Cargo workspace builds are stable;
the wire's cross-crate stability is verified by the
`cargo check --workspace` green build.

## 9. Test name uniqueness boundary

The 26 test fn names in `edge_case_tests.rs` are unique
across the workspace (CC-WIRE-008). The trusted surfaces
are:

- **`rtk rg` name-uniqueness check**:
  The 26 fn names are queried via a single `rtk rg`
  pattern; the expected result is exactly 26 hits, all
  in `edge_case_tests.rs`. The 16 sibling `_tests.rs`
  files do not contain any of the 26 names.

- **Pre-wire uniqueness verification**:
  Verified by the codebase-map.md §6 evidence capture
  (2026-07-01T15:22:00Z); the names are unique across
  `tests.rs`, `journal/tests.rs`, `recovery/tests.rs`,
  `codec/tests.rs`, and the 16 wired `_tests.rs` files.

**Justification:** `rtk rg` is a stable ripgrep wrapper;
name uniqueness is a static property of the test fn
namespace.

## 10. Cargo.toml stability boundary

`crates/vb_storage/Cargo.toml` is byte-identical
pre/post wire (CC-WIRE-009). The trusted surfaces are:

- **`git diff crates/vb_storage/Cargo.toml`**:
  Returns empty output pre/post wire. The wire does
  not add or remove any dev-dep, feature flag, or
  `[[test]]` entry.

- **Existing dev-deps sufficient**:
  `tempfile` (line 20) and `proptest` (line 19) are
  present in `[dev-dependencies]`. The transitive
  deps `fjall` and `blake3` are sufficient for all
  26 tests. No new dev-dep is needed.

**Justification:** `git diff` is a stable git feature;
the wire's Cargo.toml stability is verified by the
empty `git diff crates/vb_storage/Cargo.toml` output.

## 11. Forbidden actions boundary

The bead description and contract explicitly forbid:

- **Modifying `crates/vb_storage/Cargo.toml`**.
- **Modifying any other module in `crates/vb_storage/src/`**.
- **Modifying `.config/source-length-exceptions.txt:150`**.

The forbidden actions are tracked here for downstream
agent enforcement. The verification gate for this
boundary is the union of:

- `git diff crates/vb_storage/Cargo.toml` empty (CC-WIRE-009).
- `git diff --stat` shows 1 file changed, 3 insertions,
  0 deletions (CC-WIRE-002).
- `rtk rg -n 'edge_case_tests' .config/source-length-exceptions.txt`
  returns the same single hit at line 150 (CC-WIRE-007).

**Justification:** All three forbidden actions are
verifiable by static inspection of `git diff` and
`rtk rg` output. No new test, lint, or proof work
is required to enforce this boundary; the existing
verification commands are sufficient.

## 12. Model reductions and assumptions

This bead does not have a formal proof model (no Verus
spec, no Kani harness, no Flux extern_spec, no Loom model).
The "model" is the cargo test build graph and the
`edge_case_tests.rs` source file. The reductions and
assumptions are:

- **Module resolution is a static property of stable Rust**:
  No model reduction is needed; the 3-line declaration
  either compiles or it doesn't.

- **Test discovery is exhaustive for `#[test]` fns**:
  cargo test discovers all `#[test]` fns in every
  `#[cfg(test)] mod` of the crate. The 16 sibling
  declarations establish the precedent; the new
  declaration follows the same pattern.

- **Tally is exact for the lib test binary**:
  `cargo test -p vb_storage --lib` tally includes
  only lib tests, not integration tests in `tests/`
  or doc tests. The pre-wire baseline of 1530 is the
  lib-test tally only (verified 2026-07-01 from the
  isolated workdir); the post-wire tally of 1556
  is the same scope. The historical May 2026 baseline
  of 924 (per `.beads/vb-2bok/qa-report.md:5` and
  `.beads/vb-core-atomic-admission/STATE.md:1349`)
  is the `historic_2026_05_baseline`, NOT the current
  pre-wire value.

- **No new proptest strategies are introduced**:
  The 26 tests in `edge_case_tests.rs` are concrete-value
  tests, not `proptest!` blocks. `PROPTEST_CASES=1` in
  the obligation command is a no-op for these tests;
  the verifier-specific check is satisfied by the env
  var presence.

- **Concurrency is implicitly serialized**:
  `FjallJournal::append_*` takes `&self`; `JournalWriterQueue`
  wraps `Mutex<InnerState>` at `queue/writer.rs:33`.
  The 4 concurrent tests follow the same pattern as
  `journal/tests.rs:2598+` and `recovery/tests.rs`,
  which use default-Rust threading without a Loom lane.

## 13. Known assumptions and stub boundaries

| Assumption | Location | Justification |
|---|---|---|
| `tempfile::tempdir()` is per-test isolated | `edge_case_tests.rs:30, 77, 244, 311, 354, 397, 438` | `tempfile` crate guarantees unique directory per call; `Drop` removes it |
| Fjall `&self` append paths are thread-safe | `journal/append.rs:7, 35, 81` | Fjall internal locking; precedent in `journal/tests.rs:2598+` |
| `JournalWriterQueue` state is `Mutex<InnerState>` | `queue/writer.rs:33` | `Mutex` provides exclusive access; `Drop` releases lock |
| `fail_next_persist_for_test` is `pub(crate)` | `journal/core.rs:227` | Test-only seam; gated by `#[cfg(test)]` callers |
| Pre-wire tally is 1530 (current; verified 2026-07-01) | `PROPTEST_CASES=1 cargo test -p vb_storage --lib 2>&1 | tail -3` from isolated workdir | Direct execution from the isolated workdir on 2026-07-01 reports `test result: ok. 1530 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s` |
| Pre-wire tally is 924 (`historic_2026_05_baseline`; not current) | `.beads/vb-2bok/qa-report.md:5`, `.beads/vb-core-atomic-admission/STATE.md:1349` | Two independent captures establish the May 2026 historic baseline; superseded by the 2026-07-01 direct-execution capture |
| `edge_case_tests.rs` is on the exception ledger | `.config/source-length-exceptions.txt:150` | Owner `lewis`; removal plan `vb-jpq7.47` |
| Cargo.toml is 32 lines pre-wire | `rtk wc -l crates/vb_storage/Cargo.toml` | Verified at planning time |
| 16 sibling `#[path = "..."] mod` declarations | `lib.rs:118-181` | Pre-existing pattern; matches byte-for-byte |
| 26 test fn names are unique | `rtk rg` over the 26 names | Pre-wire verification at codebase-map.md §6 |

## 14. Non-behavior waivers

**None.** This bead has zero waiver candidates. The skill
rule "Never emit behavior-affecting waiver-candidate"
applies: all 3 obligations are `behavior_affecting: false`,
and the 6 not-applicable verifiers are documented in
`verifier-lane-decisions.jsonl` (not in
`waiver-candidates.jsonl`).

The constraint-only seeds (PS-WIRE-NOPROD-002,
PS-WIRE-NOCROSS-003, PS-WIRE-LINES-006,
PS-WIRE-LEDGER-007, PS-WIRE-UNIQ-008, PS-WIRE-CARGO-009)
require no waiver because they are not behavior proofs;
they are tracked in this trusted-base-plan.md and
`proof-coverage-matrix.md` as boundary conditions.

## 15. Reduction justification

This bead has no formal proof model to reduce. The
verification is purely a cargo test build-graph check
plus a 26-test run plus a tally comparison. The
"reduction" is the standard cargo test surface
(only lib tests, no doc tests, no integration tests
in `tests/`).

The trust in the verification rests on:

1. The 16 sibling `#[path = "..."] mod` declarations
   at `lib.rs:118-181` are clippy-clean and have been
   the canonical pattern for wave-3 dormant-test wiring
   since the 2026-05-23 round-3 sweep.
2. The 32 production symbols used by the 26 tests
   resolve to live source (verified at codebase-map.md §6
   and `delivery-scope.jsonl` rows 4-46).
3. The 26 test fn names are unique across the workspace
   (verified at codebase-map.md §6).
4. The pre-wire baseline of 1530 is established by direct
   execution (`PROPTEST_CASES=1 cargo test -p vb_storage --lib
   2>&1 | tail -3`) from the isolated workdir on 2026-07-01.
   The historical May 2026 baseline of 924 (per
   `.beads/vb-2bok/qa-report.md:5` and
   `.beads/vb-core-atomic-admission/STATE.md:1349`) is the
   `historic_2026_05_baseline` and is NOT the current pre-wire
   value.
5. The source-length exception at
   `.config/source-length-exceptions.txt:150` is
   preserved byte-identical pre/post wire.

END OF TRUSTED BASE PLAN.
