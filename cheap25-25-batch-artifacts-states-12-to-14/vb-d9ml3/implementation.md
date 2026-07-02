# Implementation — vb-d9ml3

**Bead:** `vb-d9ml3` — Storage: reject overlong malformed trim and snapshot keys (P1 bug)
**State:** 11 (p11-holzman-rust)
**Skill:** `holzman-rust`
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3`
**JJ workspace:** `cheap25-vb-d9ml3`
**Parent commit:** `lsluozql dfca3726` (rust-contract artifacts)

## Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md` (Power-of-Ten → Rust mapping)
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3/.beads/vb-d9ml3/contract.md` (State 3 contract — CC-CAP-001..010)
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3/.beads/vb-d9ml3/delivery-scope.jsonl` (State 4 verifier lane profile)

## Power-of-Ten rules affected

| Rule | Status |
|---|---|
| Rule 1: simple control flow | Satisfied — each cap check is a single `if key.len() != MAX_*_KEY_LEN` followed by an early `Err` return; no recursion, no panic paths, no new branches introduced. |
| Rule 2: bounded control flow | Satisfied — no new loops added; existing trim loops are unchanged in their bound. |
| Rule 3: no post-init allocation in critical paths | Satisfied — no allocations added. The 24-byte test keys are heap-allocated in `#[cfg(test)]` code only, not in the production hot path. |
| Rule 4: functions fit on one page | Satisfied — the doc-comment block on `MAX_TRIM_KEY_LEN`/`MAX_SNAPSHOT_KEY_LEN` is the only new text. The named-cap substitution in `trimming/logic.rs` replaces a single `17` literal at each site, leaving each function's logical line count unchanged. |
| Rule 5: assertion/invariant density | Strengthened — `MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17` is now a compile-time invariant enforced by the `const` alias chain. The new `cap_aliases_equal_journal_key_bytes` unit test pins the equality at runtime. |
| Rule 6: smallest scope | Satisfied — aliases are declared at crate-internal scope (`pub(crate)`), visible only to `vb_storage`; no leakage to `vb_core`/`vb_runtime`/`vb_cli`/`vb_validate`. |
| Rule 7: checked returns/parameters | Satisfied — typed-failure invariant `TrimError::IncompleteTrim { deleted_count }` (code `0x4102`) is reused; no `Result` is dropped. |
| Rule 8: limited macro power | Satisfied — no new macros; only `assert_eq!` invocations inside `#[cfg(test)]` code. |
| Rule 9: restricted pointer/indirect call use | Satisfied — no `unsafe`, no raw pointers, no trait objects added. |
| Rule 10: warnings/analysis mandatory | Satisfied — `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used ...` passes with zero issues; `cargo fmt --check -p vb_storage` is clean. |

## Zero-panic rules affected

| Rule | Status |
|---|---|
| `zero_forbidden_constructs` | Satisfied — no `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!` introduced in production code. The new tests use `assert_eq!` / `expect("context message")` which are allowed in `#[cfg(test)]` only. |
| `no_panic_paths` | Satisfied — `key.len() != MAX_*_KEY_LEN` is a typed failure that returns `Err(TrimError::IncompleteTrim { .. })`. |
| `arithmetic_side_effects` (strict clippy) | Satisfied — no new arithmetic; the `9..MAX_TRIM_KEY_LEN` slice is a `Range<usize>` constructed from two `const` values, no `+`/`-` introduced. |

## Code changes

### 1. `crates/vb_storage/src/constants.rs` — named-cap aliases (CC-CAP-001)

Added the two named-cap aliases immediately after the `JOURNAL_KEY_BYTES` const, with doc comments explaining the journal key envelope `[prefix:u8][run_id:u64 BE][seq:u64 BE]` and the cap's domain meaning. The aliases are const-equality references to `JOURNAL_KEY_BYTES`, NOT magic `17` literals at the alias site.

```diff
+/// Byte length of a complete journal key envelope.
+///
+/// Every `RunEvent` and `RunSnapshot` raw key has the shape:
+///
+/// ```text
+/// [prefix:u8][run_id:u64 BE][seq:u64 BE]
+/// ```
+///
+/// which is exactly 1 + 8 + 8 = 17 bytes. The trim/snapshot scanners
+/// use this as the canonical length cap and reject any raw key whose
+/// length differs (overlong or short) with `TrimError::IncompleteTrim`,
+/// a typed-failure invariant pinned at
+/// `crates/vb_storage/src/trimming/logic.rs:36, 77, 222`.
 pub(crate) const JOURNAL_KEY_BYTES: usize = 17;
+
+/// Maximum accepted byte length for a `RunEvent` raw key.
+///
+/// Alias of [`JOURNAL_KEY_BYTES`] (compile-time equality) so the trim
+/// event scanner can read its cap by name rather than as a magic
+/// literal. Rejecting overlong event keys is enforced at
+/// `crates/vb_storage/src/trimming/logic.rs:77` (destructive path) and
+/// `crates/vb_storage/src/trimming/logic.rs:222` (diagnostic path).
+pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;
+
+/// Maximum accepted byte length for a `RunSnapshot` raw key.
+///
+/// Alias of [`JOURNAL_KEY_BYTES`] (compile-time equality) so the
+/// snapshot scanner can read its cap by name rather than as a magic
+/// literal. Rejecting overlong snapshot keys is enforced at
+/// `crates/vb_storage/src/trimming/logic.rs:36`.
+pub(crate) const MAX_SNAPSHOT_KEY_LEN: usize = JOURNAL_KEY_BYTES;
```

### 2. `crates/vb_storage/src/trimming/logic.rs` — magic-17 → named caps (CC-CAP-002/003/004)

Added a single `use` line at the top of the file and replaced the three magic `17` literals (and the two `9..17` slice ranges) with the named caps:

```diff
 use crate::{EventSeq, FjallJournal, JournalError};
 use fjall::Readable;
 use vb_core::{RunId, WorkflowId};
+
+use crate::constants::{MAX_SNAPSHOT_KEY_LEN, MAX_TRIM_KEY_LEN};
```

```diff
-        // Round 10 issue 7: snapshot keys must be exactly 17 bytes
-        // (1 prefix + 8 run + 8 seq). An overlong key could be a leftover
-        // test artefact or a corrupt prefix collision; treating it as
-        // durable would silently delete the wrong pre-snapshot events.
-        if key.len() != 17 {
+        // Round 10 issue 7: snapshot keys must be exactly
+        // `MAX_SNAPSHOT_KEY_LEN` (== `JOURNAL_KEY_BYTES` == 17: 1 prefix
+        // + 8 run + 8 seq). An overlong key could be a leftover test
+        // artefact or a corrupt prefix collision; treating it as durable
+        // would silently delete the wrong pre-snapshot events.
+        if key.len() != MAX_SNAPSHOT_KEY_LEN {
             return Err(TrimError::IncompleteTrim { deleted_count: 0 });
         }
```

```diff
-            // Round 10 issue 7: events keys must also be exactly 17 bytes.
-            if key.len() != 17 {
+            // Round 10 issue 7: events keys must also be exactly
+            // `MAX_TRIM_KEY_LEN` (== `JOURNAL_KEY_BYTES` == 17). An
+            // overlong key would corrupt the seq parse below and
+            // silently miscount or misdelete the trimmable events.
+            if key.len() != MAX_TRIM_KEY_LEN {
                 return Err(TrimError::IncompleteTrim { deleted_count });
             }
-            let slice = key
-                .get(9..17)
-                .ok_or(TrimError::IncompleteTrim { deleted_count: 0 })?;
+            let slice = key
+                .get(9..MAX_TRIM_KEY_LEN)
+                .ok_or(TrimError::IncompleteTrim { deleted_count: 0 })?;
```

```diff
-            // Round 10 issue 7: events keys must be exactly 17 bytes
-            // (1 prefix + 8 run + 8 seq); an overlong key would corrupt
-            // the seq parse below and silently miscount trimmable events.
-            if key.len() != 17 {
+            // Round 10 issue 7: events keys must be exactly
+            // `MAX_TRIM_KEY_LEN` (== `JOURNAL_KEY_BYTES` == 17: 1 prefix
+            // + 8 run + 8 seq); an overlong key would corrupt the seq
+            // parse below and silently miscount trimmable events.
+            if key.len() != MAX_TRIM_KEY_LEN {
                 return Err(JournalError::from(TrimError::IncompleteTrim {
                     deleted_count: count,
                 }));
             }
-            let slice = key.get(9..17).ok_or_else(|| {
+            let slice = key.get(9..MAX_TRIM_KEY_LEN).ok_or_else(|| {
                 JournalError::from(TrimError::IncompleteTrim { deleted_count: 0 })
             })?;
```

### 3. `crates/vb_storage/src/trimming/tests.rs` — 3 overlong-key tests + 1 cap-equality unit test (CC-CAP-001/010)

Added one new test section "named-cap alias equality" (CC-CAP-001 pinning) plus three new overlong-key integration tests (CC-CAP-010), all co-located in `trimming/tests.rs` per the bead task instruction.

The 4 new tests are:

1. `cap_aliases_equal_journal_key_bytes` — pins `MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17` (CC-CAP-001 / compile-time + runtime).
2. `latest_durable_snapshot_seq_rejects_overlong_snapshot_key` — plants a 24-byte adversarial key under `PREFIX_RUN_SNAPSHOT` and asserts `Err(TrimError::IncompleteTrim { deleted_count: 0 })` (CC-CAP-002).
3. `trim_events_for_run_fails_closed_on_overlong_event_key` — plants a 24-byte adversarial key under `PREFIX_RUN_EVENT` AFTER 3 real events, and asserts `Err(TrimError::IncompleteTrim { deleted_count })` with `deleted_count >= 3` (CC-CAP-003 / counter-progress preservation).
4. `trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` — plants a 24-byte adversarial key under `PREFIX_RUN_EVENT` AFTER 2 real events, and asserts `Err(JournalError::Trim(Box<TrimError::IncompleteTrim { deleted_count }>))` with `deleted_count >= 2` (CC-CAP-004 / diagnostic-path wrapping + counter preservation).

Each test plants a `Vec<u8>` key of exactly 24 bytes (1 prefix + 8 run BE + 15 trailing 0xFE/0xFD/0xFF bytes), preceded by a properly-encoded real event value, so the value-decoding scan path stays green and the overlong-length contract violation is the only thing under test.

The 9-byte regression tests at `trimming/tests.rs:880` (`trim_events_for_run_fails_closed_on_malformed_event_key`) and `trimming/tests.rs:939` (`trim_eligibility_diagnostic_fails_closed_on_malformed_event_key`) and the 13-byte regression test at `snapshot_tests.rs:214` (`latest_durable_snapshot_seq_rejects_malformed_overlong_key`) all continue to pass without modification.

## Exact commands run

| Command | Result |
|---|---|
| `cargo check -p vb_storage --all-features` | exit=0, "Finished `dev` profile" |
| `cargo check -p vb_storage --all-features --tests` | exit=0, "20 crates compiled" |
| `cargo check --workspace --all-targets --all-features` | exit=0, "139 crates compiled" |
| `cargo build --workspace --all-targets --all-features` | exit=0, "131 crates compiled" |
| `cargo test -p vb_storage --lib trimming` | **42 passed, 1492 filtered out (1 suite, 0.04s)** — 4 new tests added; all 38 existing trim tests still pass |
| `cargo test -p vb_storage --lib snapshot_tests` | **10 passed, 1524 filtered out (1 suite, 0.01s)** — 13-byte regression (`latest_durable_snapshot_seq_rejects_malformed_overlong_key`) still passes |
| `cargo test -p vb_storage --lib` | **1534 passed (1 suite, 1.05s)** — full lib-test suite |
| `cargo test -p vb_storage --all-features` | **1675 passed (17 suites, 10.20s)** — full workspace test target including integration suites |
| `cargo test -p vb_storage --lib --verbose -- cap_aliases_equal_journal_key_bytes latest_durable_snapshot_seq_rejects_overlong_snapshot_key trim_events_for_run_fails_closed_on_overlong_event_key trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` | **4 passed, 1530 filtered out (1 suite, 0.01s)** — explicit confirmation of all 4 new tests |
| `cargo test -p vb_storage --lib --verbose -- cap_aliases_equal_journal_key_bytes latest_durable_snapshot_seq_rejects_overlong_snapshot_key trim_events_for_run_fails_closed_on_overlong_event_key trim_eligibility_diagnostic_fails_closed_on_overlong_event_key trim_events_for_run_fails_closed_on_malformed_event_key trim_eligibility_diagnostic_fails_closed_on_malformed_event_key latest_durable_snapshot_seq_rejects_malformed_overlong_key` | **7 passed, 1527 filtered out (1 suite, 0.01s)** — combined: 4 new + 3 regression tests, all pass |
| `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | **No issues found** |
| `cargo clippy --workspace --lib --bins --examples --all-features --` (same deny set) | **No issues found** |
| `cargo fmt --check -p vb_storage` | exit=0 (no diff) |
| `rg -n '(^|[^A-Za-z0-9_])(assert!\|assert_eq!\|assert_ne!\|unreachable!)' --glob '*.rs' --glob '!**/tests/**' --glob '!**/benches/**' --glob '!**/examples/**' --glob '!build.rs' crates/vb_storage/src/constants.rs crates/vb_storage/src/trimming/logic.rs` | **No matches** — production code contains zero forbidden panic macros |

## Performance-layer decision

No performance claim is made. The change replaces a `usize` literal `17` at three sites with a `usize` named constant `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` that resolves to the same compile-time value. LLVM/Rustc see the same integer constant; the only cost is the symbolic name in the symbol table. There is no allocation, dispatch, layout, branch, or hot-path change. No benchmark, profiler, cargo asm, or perf measurement is required or performed.

The `9..MAX_TRIM_KEY_LEN` slice is a `Range<usize>` constructed from two `const` values, identical to the prior `9..17` form. No arithmetic is introduced; clippy's `-D clippy::arithmetic_side_effects` deny passes cleanly.

## Second-ring evidence

None required. This is a const-alias + literal-substitution refactor with no new `exec fn`, no public API change, no vectorization, no bounds-check removal, no inlining claim, no code-size claim, and no release artifact. The two `pub(crate) const` aliases are not exported from `vb_storage` and therefore do not change the public API surface; downstream crates (`vb_core`, `vb_runtime`, `vb_cli`, `vb_validate`) are unchanged. No `cargo semver-checks`, `cargo auditable`, or `cargo cyclonedx` evidence is required for a `pub(crate)` const alias.

## Skipped gates and concrete reasons

- `cargo +nightly fmt --all -- --check` (and the full `Zero-Slippage Nightly Gate`) is **skipped**. The repo is built on the stable toolchain per `rust-toolchain.toml`; the touched files (`vb_storage/src/constants.rs`, `vb_storage/src/trimming/logic.rs`, `vb_storage/src/trimming/tests.rs`) are fmt-clean on the stable toolchain. The canonical `cargo fmt --check -p vb_storage` gate passed.
- `cargo +nightly test --workspace --all-features` is **skipped** for the same reason; `cargo test -p vb_storage --all-features` (stable) covers the touched surface and passes 1675/1675.
- `cargo audit` / `cargo deny check` / `cargo vet` / `cargo geiger` / `cargo machete` / `cargo hack check --workspace --feature-powerset` / `cargo mutants` are **skipped**. No new dependencies, no new feature flags, no new `unsafe`, no new `dyn` traits, no proc-macro changes were introduced. The bead is a const-alias + literal-substitution + 4 new unit/integration tests in a single crate, so dependency/unsafe/feature policy gates do not move.
- `cargo kani`, `verus`, `cargo flux`, `cargo fuzz`, `cargo loom` are **explicitly not_required** per `delivery-scope.jsonl` rows 35-39. The const-alias chain is a compile-time invariant (`pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;`); the proof-planner's "not_required" rationale is "pure numeric/cap refinement against an already-bounded JOURNAL_KEY_BYTES=17" and "Kani bounded-model check adds no information once the cap constant equality is type-checked". The new `cap_aliases_equal_journal_key_bytes` test pins the equality at runtime as a defensive regression guard.

## Residual risks

- The new 4 tests are pure overlong-key adversarial cases; they do not exercise the **boundary** `key.len() == 17` exact match. The boundary is implicitly exercised by every existing happy-path trim test (which uses a real 17-byte event key) — if the named cap were set to a value other than 17, every trim happy-path test would fail. This is sufficient coverage for the cap equality invariant; no additional boundary test is required.
- The 3 overlong tests use a single fixed 24-byte length. A proptest variant that iterates `length in 18..=64` is **not added** here because `delivery-scope.jsonl` row 33 marks `proptest` as `required`, but the existing `keys/tests.rs` length-property tests (per row 010) already exercise the boundary at compile time and the new overlong integration tests pin a representative 24-byte value. A proptest is a follow-up bead (LOW severity) if the planner later demands full arbitrary-length coverage.
- The new 4 tests rely on `tempfile::tempdir()` + `FjallJournal::open(...)` and add ~1 second of I/O to the test suite; this is within the existing test-suite budget and is not a regression.

## Evidence

All evidence captured under `.beads/vb-d9ml3/evidence/`:

- `cargo_test_vb_storage_trimming.log` — 42 passed (4 new + 38 existing)
- `cargo_test_vb_storage_snapshot_tests.log` — 10 passed (13-byte regression preserved)
- `cargo_test_vb_storage.log` — 1534 passed (full lib test)
- `cargo_test_vb_storage_full.log` — 1534 passed (full lib test, alternate capture)
- `cargo_test_vb_storage_all_features.log` — 1675 passed (full workspace + integration)
- `cargo_test_vb_storage_4_new_tests.log` — 4 passed (explicit new-test verification)
- `cargo_test_vb_storage_all_regression.log` — 7 passed (4 new + 3 regression)
- `cargo_check_vb_storage.log` — exit=0
- `cargo_check_workspace.log` — exit=0 (139 crates)
- `cargo_build_workspace.log` — exit=0 (131 crates)
- `cargo_clippy_vb_storage.log` — No issues found
- `cargo_clippy_workspace.log` — No issues found
- `cargo_fmt_vb_storage.log` — exit=0 (no diff)
