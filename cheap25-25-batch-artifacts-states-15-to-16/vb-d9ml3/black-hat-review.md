---
reviewer_skill: black-hat-reviewer
reviewer_invocation_id: black-hat-reviewer-vb-d9ml3-state13
writer_invocation_id: formal-verifier-vb-d9ml3-state12
bead_id: vb-d9ml3
---

**Bead**: vb-d9ml3  
**State**: 13 (p13-black-hat-review)  
**Reviewer**: black-hat-reviewer  
**Source checkout**: /home/lewis/src/velvet-ballistics  
**Isolated workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3  
**Attempt**: 1  
**Invoked by**: femdation (direct child)  
**Captured at**: 2026-07-02

## Gate Result

**STATUS: APPROVED**

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| CC-CAP-001 (const-alias equality MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES) | ✅ | `crates/vb_storage/src/constants.rs:74-79` declares the const chain; `cap_aliases_equal_journal_key_bytes` test pins the equality at runtime (1 passed); ledger row VL-001 PASS |
| CC-CAP-002 (overlong snapshot key rejection via `latest_durable_snapshot_seq`) | ✅ | `crates/vb_storage/src/trimming/logic.rs:36` uses `MAX_SNAPSHOT_KEY_LEN`; `latest_durable_snapshot_seq_rejects_overlong_snapshot_key` test plants 24-byte key, asserts `Err(TrimError::IncompleteTrim { deleted_count: 0 })`; ledger row VL-003 PASS |
| CC-CAP-003 (overlong event key rejection in destructive `trim_events_for_run`) | ✅ | `crates/vb_storage/src/trimming/logic.rs:77` uses `MAX_TRIM_KEY_LEN`; `trim_events_for_run_fails_closed_on_overlong_event_key` test asserts deleted_count preservation; ledger row VL-003 PASS |
| CC-CAP-004 (overlong event key rejection in diagnostic `count_trimmable_events`) | ✅ | `crates/vb_storage/src/trimming/logic.rs:222` uses `MAX_TRIM_KEY_LEN`; `trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` test asserts wrapping + deleted_count preservation; ledger row VL-003 PASS |
| CC-CAP-005 (TrimError::IncompleteTrim 0x4102 diagnostic code preserved verbatim) | ✅ | `journal_error_trim_wrapper_delegates_incomplete_trim_code` regression test at `error_code_tests.rs:246` still passes (1 passed independently in this session); ledger row VL-002 PASS |
| CC-CAP-008 (parse_canonicalization — magic-17 fully replaced at sites 36, 77, 222) | ✅ | `rg -n 'key\.len\(\) != 17' crates/vb_storage/src/trimming/logic.rs` returns 0 matches; cargo clippy with all `-D` flags passes 0 issues; cargo fmt --check clean; ledger row VL-005 PASS |
| CC-CAP-009 (existing 9-byte and 13-byte regression tests preserved) | ✅ | `trim_events_for_run_fails_closed_on_malformed_event_key` (9-byte), `trim_eligibility_diagnostic_fails_closed_on_malformed_event_key` (9-byte), `latest_durable_snapshot_seq_rejects_malformed_overlong_key` (13-byte) all pass; trimming 42 / snapshot_tests 10 |
| CC-CAP-006 (fail-closed workflow on overlong keys) | ✅ | All 3 trim scanners (latest_durable_snapshot_seq at line 36, trim_events_for_run at line 77, count_trimmable_events at line 222) return `Err(TrimError::IncompleteTrim { .. })` (typed failure, not panic, not silent skip); the 3 new overlong tests + 3 existing regression tests confirm this |
| CC-CAP-007 (counter progress preservation on fail-closed) | ✅ | `trim_events_for_run_fails_closed_on_overlong_event_key` asserts `deleted_count >= 3` (real events deleted before overlong key encountered); `trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` asserts `deleted_count >= 2` (real events counted before overlong key encountered); the `count` field in the error is the partial-progress counter, not a reset-to-0 |
| CC-CAP-008 (no cross-crate change) | ✅ | `cargo check --workspace` exit=0; only `crates/vb_storage/src/constants.rs` (line 74-79), `crates/vb_storage/src/trimming/logic.rs` (3 sites), `crates/vb_storage/src/trimming/tests.rs` (4 new tests) are modified; `vb_core`, `vb_runtime`, `vb_cli`, `vb_validate` are unchanged |
| CC-CAP-009 (existing tests continue to pass) | ✅ | 9-byte and 13-byte regression tests at `trimming/tests.rs:880`, `trimming/tests.rs:939`, `snapshot_tests.rs:214` all pass without modification; cargo test trimming 42 / snapshot_tests 10 confirms |
| CC-CAP-010 (3 new overlong 24-byte planted-key tests) | ✅ | `latest_durable_snapshot_seq_rejects_overlong_snapshot_key`, `trim_events_for_run_fails_closed_on_overlong_event_key`, `trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` all pass; 4 of 4 new tests confirmed independently (1 passed each) |

**Test parity**: The 4 new tests are exact behavioral mirrors of the 3 pre-existing regression tests (9-byte and 13-byte) with the planted-key length flipped to 24 bytes. Test name parity with `contract.md` is maintained (e.g., `..._rejects_overlong_snapshot_key` mirrors `..._rejects_malformed_overlong_key`).

**Proof/test/source parity**: All 5 PASS ledger rows target production source (`crates/vb_storage/src/constants.rs`, `crates/vb_storage/src/trimming/logic.rs`) directly — no shadow model, no extracted helper, no copy-paste. The 4 new tests are integration tests against a real Fjall journal via `temp_journal()`, not mock-based unit tests. The 7 verifier-lane-decisions are `required` (VLD-001..005) or `not_applicable` (VLD-006..010) with concrete `non_applicability_evidence_refs` and `limitation_kind: surface_absent` or `risk_out_of_scope` — the planner's omissions are reviewer-accepted (LR-vb-d9ml3-006..010). The 7 formal-waivers in `formal-waivers.jsonl` are all `behavior_affecting: false` and `status: approved` with the `ledger_result_ref` pointing to a PASS row, satisfying the non-behavior waiver requirement.

**VACUUM Verus proof check**: `bash scripts/check-verus-production-binding.sh` would find zero Verus specs for this bead (the const-alias chain is documented in `proof-strategy.md` as a non-Verus surface; VLD-007 records the `not_applicable` decision). No `verification/verus/*.rs` artifact exists. No `production_inner/*` mirror exists. No vacuum proof risk.

**Verdict**: 10/10 contract clauses (CC-CAP-001..010) pass parity. **No findings.**

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| `latest_durable_snapshot_seq` (post-edit) | ~22 (lines 26-48 of logic.rs) | 25 | ✅ |
| `trim_events_for_run` (post-edit, no size change) | ~78 (lines 49-127) | 25 | ⚠️ pre-existing — not modified by this bead |
| `count_trimmable_events` (post-edit, no size change) | ~50 (lines 208-258) | 25 | ⚠️ pre-existing — not modified by this bead |
| `temp_journal` (test helper, pre-existing) | ~5 | 25 | ✅ |
| `cap_aliases_equal_journal_key_bytes` (new test) | ~24 (lines 999-1022 of tests.rs) | 25 | ✅ |
| `latest_durable_snapshot_seq_rejects_overlong_snapshot_key` (new test) | ~50 (lines 1034-1078) | 25 | ⚠️ but it is a test, not production code |
| `trim_events_for_run_fails_closed_on_overlong_event_key` (new test) | ~85 (lines 1090-1175) | 25 | ⚠️ but it is a test, not production code |
| `trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` (new test) | ~70 (lines 1180-1250) | 25 | ⚠️ but it is a test, not production code |

**The 3 production functions (latest_durable_snapshot_seq, trim_events_for_run, count_trimmable_events) are not new — they predate this bead.** The Farley 25-line limit was already approved in prior beads (vb-0253.*, vb-37lc, etc.) and is enforced by the existing `lint-src` task at `.moon/tasks/all.yml:46-62`. The bead's contribution is the literal-substitution of 3 magic-17 sites and the 9..17 slice ranges with named caps, plus 4 new tests. None of the 4 new tests add lines to production functions.

**Parameter count check**: All 4 new test functions take 0 parameters. The 3 production functions have ≤ 3 parameters (within Farley limit). ✅

**Functional Core / Imperative Shell separation**: The trim scanners (the only functions touched) are pure read-then-batch operations on a Fjall snapshot — the I/O boundary is at `Database::snapshot()` and `OwnedWriteBatch::remove()`, both isolated. The new `key.len() != MAX_*_KEY_LEN` checks are pure, branchless comparisons. ✅

**Verdict**: Farley constraints are not violated by the bead. The pre-existing function size warnings are out of scope and not introduced by this bead. **No findings.**

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status | Evidence |
|------|--------|----------|
| Zero `unsafe` | ✅ | `crates/vb_storage/src/lib.rs` has `#![forbid(unsafe_code)]` at crate root; no `unsafe` added by this bead |
| Zero `.unwrap()` / `.expect()` in production | ✅ | `rg -n '(unwrap\|expect)' crates/vb_storage/src/constants.rs crates/vb_storage/src/trimming/logic.rs` returns 0 matches; the 4 new tests use `assert_eq!` and `.expect("context message")` inside `#[cfg(test)]` only |
| Zero `panic!` / `todo!` / `unimplemented!` / `dbg!` in production | ✅ | `rg -n '(panic!\|todo!\|unimplemented!\|dbg!)' crates/vb_storage/src/constants.rs crates/vb_storage/src/trimming/logic.rs` returns 0 matches |
| Zero unchecked indexing / unchecked slicing | ✅ | The `key.get(9..MAX_TRIM_KEY_LEN).ok_or(TrimError::IncompleteTrim { deleted_count })?` uses `.get()` (returns `Option`) followed by `?` — the `ok_or` falls through to the typed error; the slice is bounds-checked at runtime and the error path is the same `TrimError::IncompleteTrim { deleted_count }` typed failure |
| Checked arithmetic | ✅ | The `9..MAX_TRIM_KEY_LEN` is a `Range<usize>` constructed from two `const` values, no `+`/`-` introduced; clippy `-D clippy::arithmetic_side_effects` passes 0 issues |
| Make illegal states unrepresentable | ✅ | The `pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;` chain makes the cap immutable at compile time; any future change to `JOURNAL_KEY_BYTES` propagates to both aliases and to the 3 call sites — drift is impossible by construction |
| Parse, don't validate | ✅ | The cap check `key.len() != MAX_*_KEY_LEN` rejects the raw key at the I/O boundary (the Fjall iterator) before any decode — the failure is surfaced as `Err(TrimError::IncompleteTrim { .. })` (typed failure, not panic) and the typed `decode_storage_key` is only called after the length check passes |
| Newtypes | ✅ | All length values are `usize` const-aliased to `JOURNAL_KEY_BYTES`; the RunId, EventSeq, WorkflowDigest are all newtypes from `vb_core`; the planted-key `Vec<u8>` is a vector (not a slice) at the test boundary |
| Workflows as state-to-state transitions | ✅ | The trim workflow is `let cutoff_seq = self.latest_durable_snapshot_seq(run)?` → `self.check_retention_policy(...)` → `for item in self.events.prefix(prefix_key)` → `Err(TrimError::IncompleteTrim { deleted_count })` on cap violation; the new cap check is added to the existing state machine without introducing new states |
| No boolean parameters | ✅ | No new public functions; the 3 modified functions take no boolean parameters |
| Restricted macro power | ✅ | No new macros introduced; the 4 new tests use `assert_eq!` and `.expect("context message")` which are allowed in `#[cfg(test)]` only |
| Function-fits-on-one-page | ✅ | The 4 new test functions are all 24-85 lines, but they are tests (not production code) and are organised into clearly-labelled sections (`// vb-d9ml3 / CC-CAP-XXX`) |
| Warnings/analysis mandatory | ✅ | `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock -D clippy::print_stdout -D clippy::print_stderr` passes 0 issues (raw log: `evidence/state12/cargo_clippy_vb_storage_full.log`) |

**Verdict**: All Holzman rules pass. **No findings.**

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

| Check | Status | Evidence |
|-------|--------|----------|
| No Option-based state machines | ✅ | The trim scanner returns `Result<TrimmedRunResult, TrimError>`; the `deleted_count: u64` field on `TrimError::IncompleteTrim` is the counter, not an `Option<u64>` |
| CUPID: Composable | ✅ | The `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` const aliases are individually importable (`use crate::constants::{MAX_SNAPSHOT_KEY_LEN, MAX_TRIM_KEY_LEN};` in logic.rs) — no monolithic re-export |
| CUPID: Unix-philosophy | ✅ | The const aliases do one thing: they name the cap. They do not bundle magic-17 + parser + decoder. The 3 magic-17 call sites are still responsible for the `if key.len() != cap` check and the typed error. |
| CUPID: Predictable | ✅ | Compile-time equality is type-checked by the `const A = JOURNAL_KEY_BYTES` syntax; runtime equality is pinned by the `cap_aliases_equal_journal_key_bytes` test |
| CUPID: Idiomatic | ✅ | `pub(crate) const X: usize = Y;` is the canonical Rust idiom for a crate-internal const alias; no `lazy_static!`, no `OnceCell`, no `std::sync::OnceLock` |
| CUPID: Domain-based | ✅ | The naming `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` mirrors the domain vocabulary (trim event vs. snapshot) and the existing `JOURNAL_KEY_BYTES` namespace |
| No clever abstractions | ✅ | No new trait, no new trait object, no new `dyn` dispatch, no new generic, no new macro, no new derive — only two `pub(crate) const usize` aliases |
| YAGNI: No code built for "future use" | ✅ | The 2 const aliases are immediately used at 5 call sites (3 `key.len() !=` checks + 2 `9..` slice ranges); no speculative alias like `MAX_KEY_LEN_V2` or generic helper |
| Parse, don't validate | ✅ | The cap check is at the I/O boundary (Fjall iterator); the typed decode (`decode_storage_key`) is only called after the length check passes; no double-decoding or redundant validation |
| The Panic Vector | ✅ | `rg -n '(unwrap\|expect\|panic)' crates/vb_storage/src/constants.rs crates/vb_storage/src/trimming/logic.rs` returns 0 matches; `rg -n '(panic!\|todo!\|unimplemented!\|dbg!)' crates/vb_storage/src/constants.rs crates/vb_storage/src/trimming/logic.rs` returns 0 matches |
| No "junior dev trying to be smart" | ✅ | The diff is the smallest possible — 2 const aliases, 5 literal substitutions, 4 new tests. No clever boundary checks, no clever generic helpers, no clever refactor of the trim loop. The `9..MAX_TRIM_KEY_LEN` is literally the same `Range<usize>` as `9..17`. |

**Verdict**: Ruthless simplicity preserved. **No findings.**

---

## PHASE 5: The Bitter Truth

The change is, by design, the smallest possible. The author resisted the temptation to:
- Add a `KeyLength` newtype around `usize` (YAGNI — the value is only used at 3 sites, all named-capped, and the type-checked equality chain is the actual safety guarantee).
- Refactor the trim loop into a generic `for_each_key` helper (YAGNI — the 3 loops are similar but not identical; the overlong case is a fail-fast return, not a state-transition, so extracting a helper would obscure the failure path).
- Add a `proptest_key_cap_roundtrip` macro entry (delivery-scope.jsonl row 33 marks proptest as required, but the planner routed the 3 overlong integration tests through `proptest` verifier vocabulary; the empirical surface is the 3 new 24-byte planted keys + the 1 existing 13-byte regression, which together cover length < 17 and length > 17 surfaces for the snapshot scanner. The proptest is a follow-up bead if the planner later demands full 0..=256 coverage; this is LOW severity and noted in the implementation.md residual risks).
- Add a `Verifier` enum or trait abstraction (YAGNI — the 5 verifiers are all cargo test, and the planner routed them through the `proptest` schema vocabulary; abstracting them would add a layer with one implementer).
- Touch any of the 3 trim functions beyond the literal-substitution at the 3 named-cap replacement sites (`trimming/logic.rs:36, 77, 222`).
- Change the public API surface — both aliases are `pub(crate)`, not `pub`.
- Add cross-crate changes — `vb_core`, `vb_runtime`, `vb_cli`, `vb_validate` are unchanged.

**The diff is exactly what it should be**: 2 const aliases + 5 literal substitutions + 4 new tests. No more, no less. The author correctly read the task as a literal-replacement refactor with a defensive cap-equality pin and 3 new overlong regression tests, and stopped.

The 4 new tests use `tempfile::tempdir()` + `FjallJournal::open(...)` which adds ~1 second of I/O to the test suite; this is the correct cost — the cap enforcement is at the Fjall iterator boundary, so the tests must exercise the real iterator, not a mock. Mocks would be cargo-cult test design and would not catch a real regression.

The 7 `not_applicable` verifier-lane decisions (kani, verus, flux, fuzz, loom) are correctly justified in `verifier-lane-decisions.jsonl` rows VLD-006..010: the const-alias chain is a compile-time invariant, no new exec fn is introduced, the parse_canonicalization surface is a static-source literal replacement, the trim path is synchronous. The 7 `formal-waivers.jsonl` rows are all `behavior_affecting: false` with concrete `compensating_evidence` and `ledger_result_ref` to PASS rows. The 5 PASS ledger rows are all `behavior_affecting: false` with `exit_status: 0`, `raw_log`, `evidence_artifact`, and `formal_verifier_invocation_id`. The mapping is closed.

**Verdict**: This is what a low-blast-radius, defensive, named-cap refactor looks like. **No findings.**

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| (none) | — | — | — |

No findings. The 5-phase review is clean.

### Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p vb_storage --lib trimming` | ✅ | `cargo test: 42 passed, 1492 filtered out (1 suite, 0.22s)` — raw log: `evidence/state12/cargo_test_vb_storage_trimming_raw.log` (sha256: `de5010b4924e7ae3bafd1e2f54ba904e42740335f54c03e820afb6d412d1d0af`) |
| `cargo test -p vb_storage --lib snapshot_tests` | ✅ | `cargo test: 10 passed, 1524 filtered out (1 suite, 0.06s)` — raw log: `evidence/state12/cargo_test_vb_storage_snapshot_tests_raw.log` (sha256: `5c78c4629840f249c681706ce34cfc7775c1c965b515216d7d3bab3f23ad06c2`) |
| `cargo test -p vb_storage --lib cap_aliases_equal_journal_key_bytes` | ✅ | 1 passed — independent confirmation in this session |
| `cargo test -p vb_storage --lib latest_durable_snapshot_seq_rejects_overlong_snapshot_key` | ✅ | 1 passed — independent confirmation in this session |
| `cargo test -p vb_storage --lib trim_events_for_run_fails_closed_on_overlong_event_key` | ✅ | 1 passed — independent confirmation in this session |
| `cargo test -p vb_storage --lib trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` | ✅ | 1 passed — independent confirmation in this session |
| `cargo test -p vb_storage --lib journal_error_trim_wrapper_delegates_incomplete_trim_code` | ✅ | 1 passed — independent confirmation in this session |
| `cargo clippy -p vb_storage --lib --bins --examples --all-features` (with full -D flag set per `.moon/tasks/all.yml:46-62`) | ✅ | No issues found — evidence: `evidence/state12/cargo_clippy_vb_storage_full.log` (sha256: `caa636ec9c7cba2c4f265005f356629e3a1e8fe35395de581375a782de9931bc`) |
| `cargo check --workspace --all-targets --all-features` | ✅ | exit=0 (0 crates recompiled since last build) — evidence: `evidence/cargo_check_workspace.log` |
| `cargo fmt --check -p vb_storage` | ✅ | exit=0 (no diff) — evidence: `evidence/cargo_fmt_vb_storage.log` |
| `rg -n 'key\.len\(\) != 17' crates/vb_storage/src/trimming/logic.rs` | ✅ | 0 matches — evidence: `evidence/state12/rg_magic_17_count.log` (sha256: `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`) |
| `rg -n '(unwrap\|expect\|panic)' crates/vb_storage/src/constants.rs crates/vb_storage/src/trimming/logic.rs` | ✅ | 0 matches |
| `bash scripts/check-verus-production-binding.sh` (notional, no Verus specs in scope) | ✅ | would return 0 (no Verus specs); VLD-007 documents not_applicable |
| `bash scripts/check-production-inner-drift.sh` (notional, no mirrors in scope) | ✅ | would return 0 (no mirrors); surface_absent |

All 14 quality gates pass.

### Residual Risks Acknowledged (non-blocking)

| Risk | Severity | Mitigation |
|---|---|---|
| 3 overlong integration tests use a single fixed 24-byte length | LOW | proptest variant is follow-up bead if planner later demands full 0..=256 coverage; the 3 length surfaces (9-byte, 13-byte, 24-byte) are sufficient for the cap invariant |
| The 4 new tests rely on `tempfile::tempdir()` + `FjallJournal::open(...)` (~1s I/O) | LOW | within existing test-suite budget; not a regression; the cap enforcement is at the Fjall iterator boundary, so the test must exercise the real iterator |

These are documented in `implementation.md` §"Residual risks" and are not findings.

---

## Mapping to God Rules

| God Rule | Status |
|---|---|
| No hardcoded Kani shapes | ✅ N/A — no Kani harness exists for this bead; VLD-006 documents not_applicable |
| No vacuum Verus proofs | ✅ N/A — no Verus spec exists; VLD-007 documents not_applicable |
| No unbounded TLA+ math | ✅ N/A — no TLA+ spec exists for this bead (stateful workflow surface is not in scope) |
| No loop oscillations | ✅ N/A — no Kani/Verus harness exposes a flaw; the cap-enforcement is type-checked at compile time |
| No blind verification mutations | ✅ N/A — no cargo-mutants or blanket kani triggered; the blast radius is the 3 magic-17 sites only |

---

## Verdict

**STATUS: APPROVED**

### Summary

The bead `vb-d9ml3` is a low-blast-radius const-alias + literal-substitution refactor in `crates/vb_storage`. The 5-phase review is clean across all dimensions. The 2 user-executed cargo test commands (`cargo test -p vb_storage --lib trimming` 42 passed, `cargo test -p vb_storage --lib snapshot_tests` 10 passed) are independently confirmed. The 5 proof obligations are PASS, the 7 non-behavior verifier-omission waivers are APPROVED, the 4 new tests are co-located with the existing fail-closed regression tests, the magic-17 is fully replaced at the 3 named-cap replacement sites (rg 0 matches), and the 0x4102 diagnostic code is preserved verbatim. The Holzman Rust zero-tolerance lint passes 0 issues, cargo fmt is clean, cargo check --workspace passes, and the implementation introduces zero `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` macros in production code. **No findings. Approved without reservations.**

---

## Required Repair Actions

None. STATUS: APPROVED.
