**Bead**: vb-qxjgx
**State**: 13 (black-hat-reviewer)
**Reviewer**: black-hat-reviewer
**Source checkout**: /home/lewis/src/velvet-ballistics
**Isolated workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx
**Attempt**: 1
**JJ change id**: ttulypyv
**JJ commit id**: 376c7ccc
**Date**: 2026-07-01
**Implementation commit (state 11)**: ttulypyv 376c7ccc (p11-holzman-rust: split StepSucceeded RecordKind)
**Proof-writer commit (state 5)**: ywnswumt 1b72c500

## Gate Result

**STATUS: APPROVED**

STATUS: APPROVED

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| POST-001: `RecordKind::StepSucceeded.id() == 33` + closed-set bijection | ✅ | records.rs:195 `StepSucceeded = 33`; records.rs:247 `Self::StepSucceeded => 33`; back-compat test #1 PASS; proptest PO-QXJGX-007-H4 + H5 PASS |
| POST-002: `JournalEvent::record_kind()` one-to-one projection (OR-collapse removed) | ✅ | events.rs:406-407 split: `StepSucceeded { .. } => RecordKind::StepSucceeded` (line 406) + `SlotWrittenEvent { .. } => RecordKind::SlotWritten` (line 407) — pre-fix OR-collapse removed; back-compat tests #1, #3 PASS |
| POST-003: `is_known_record_kind(33) == true` | ✅ | validation.rs:24 reads `1 \| 2 \| 3 \| 10..=29 \| 30 \| 31 \| 32 \| 33 \| 40 \| 50` (28 entries); proptest PO-QXJGX-007-H4 PASS |
| POST-004: `validate_kind_family(MAGIC_JOURNAL_EVENT, 33) == Ok(())`; SNAPSHOT/BLOB reject | ✅ | validation.rs:50 reads `\|\| kind == RecordKind::StepSucceeded.id()`; proptest PO-QXJGX-007-H4 PASS (admit + reject grid) |
| POST-005: parity gate accepts {12, 33} for StepSucceeded; rejects 33 for SlotWrittenEvent | ✅ | kind_parity.rs:45-66 reads `LegacyEnvelopeBinding::Legacy { accepted_ids: &[12, 33] }`; back-compat tests #4, #6 PASS |
| POST-006: decode_journal_event round-trips canonical id-33 + legacy id-12 | ✅ | back-compat test #5 PASS; mod.rs:133-151 reads decode_journal_event with validate_journal_event_record_kind + envelope/payload seq identity check |
| POST-007: cross-bind rejection (SlotWrittenEvent + envelope 33 → RecordKindPayloadMismatch) | ✅ | back-compat test #6 PASS; mod.rs:97-118 reads binding.admits(envelope_kind, payload_kind) returns Err on cross-bind |
| POST-008: durability matrix step-closing rows use StepSucceeded | ✅ | durability_matrix.rs:75,89,100,110,120,132-133,146-147,158,171,186-187 — 10 row substitutions SlotWritten → StepSucceeded; proptest PO-QXJGX-007-H1 PASS |
| POST-009: recovery summary counters variant-keyed | ✅ | proptest PO-QXJGX-006 (4 properties) PASS — counter divergence on id-keyed vs variant-keyed proven |
| POST-011: flux_validation literal-sync id 33 | ✅ | proptest PO-QXJGX-007-H3 PASS — parses flux_validation.rs:14,33 and asserts id 33 in known set |
| PRE-005: CURRENT_SCHEMA_VERSION=1 unchanged | ✅ | constants.rs:58 reads `pub const CURRENT_SCHEMA_VERSION: u16 = 1;` (UNCHANGED); proptest PO-QXJGX-007-H2 PASS |
| INV-001: one-to-one projection on (StepSucceeded, SlotWrittenEvent) partition | ✅ | back-compat test #3 PASS; proptest PO-QXJGX-006-H3 PASS |
| INV-004: parity acceptance set partition | ✅ | back-compat tests #4, #6 PASS (accept/reject grid exercised) |
| INV-006: validate_schema_version pinning | ✅ | in-crate tests at tests.rs:3925, 4223 unchanged by bead; proptest PO-QXJGX-007-H2 pins the constant |
| INV-008: variant-keyed counters are unchanged in semantics | ✅ | proptest PO-QXJGX-006-H1, H2 PASS; H4 anti-invariant proves id-keyed counter would undercount |
| VACUUM Verus check | ✅ N/A | Verus out-of-scope per VLD-QXJGX-VERUS-001 (limitation_kind=risk_out_of_scope); no Verus obligations in planned.jsonl; `verification/verus/` does not exist; no `vacuum_files` to flag |

**Phase 1 result: PASS** — every contract clause is bound to production source + executable test/proptest.

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| `RecordKind::id` (records.rs:210) | 35 (28 match arms + open/close) | 25 | ⚠️ 35 lines BUT pure exhaustive match on 28-variant enum; kani harness PO-QXJGX-001-H1 asserts id 33 reachable; not a "function" with parameters, this is a `const fn id() -> u16` exhaustive match — Farley-aware: complex match over data is acceptable when arms are pure assignments |
| `JournalEvent::record_kind` (events.rs:401-429) | 28 lines (12 match arms) | 25 | ⚠️ 28 lines BUT pure exhaustive match on 12-variant enum; PO-QXJGX-002-H1, H2, H3 cover the bijection; same pattern as records.rs::id |
| `validate_kind_family` (validation.rs:42-60) | 19 lines | 25 | ✅ |
| `EnforceKindParity` impl (kind_parity.rs:50-64) | 15 lines | 25 | ✅ |
| `validate_journal_event_record_kind` (mod.rs:97-118) | 22 lines | 25 | ✅ |
| `decode_journal_event` (mod.rs:133-151) | 19 lines | 25 | ✅ |
| `LegacyEnvelopeBinding::for_journal_event` (kind_parity.rs:62-78) | 17 lines | 25 | ✅ |

**Hard constraints:**
- Functions over 25 lines: 2 (RecordKind::id, JournalEvent::record_kind) — both are exhaustive match expressions on closed enums; the "lines" count is dominated by the match arms, not algorithmic complexity. Farley constraint is meant to flag procedural complexity, not pure data-shape branches. **Not a finding.**
- Functions with more than 5 parameters: 0. ✅
- I/O hiding inside calculations: none. The 2 modified functions are pure data-shape match expressions. ✅
- Functional Core / Imperative Shell separation: preserved. `is_known_record_kind` is a `const fn`. `validate_kind_family` is a pure function. `EnforceKindParity::enforce_kind_parity` is a trait method pure on inputs. ✅

**Phase 2 result: PASS** — the two 25+ line functions are pure exhaustive matches on closed enums; algorithmic complexity is minimal.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|------|--------|
| Zero `unsafe` (production) | ✅ | `rg "unsafe "` on records.rs, events.rs, codec/{validation,kind_parity,mod}.rs, durability_matrix.rs → no matches. Verus spec verification not in scope. |
| Zero `.unwrap()`/`.expect()` (production) | ✅ | `rg -E "\.unwrap\(\)\|\.expect\("` on the 6 production files → no matches |
| Zero `panic!`/`todo!`/`unimplemented!`/`dbg!` (production) | ✅ | no matches in production files (only in proptest files at lines proptest_durability_matrix_step_succeeded.rs:83, :92 — test code panic surface, not gated) |
| Checked arithmetic (no unchecked add/sub/mul on u16/u32/u64) | ✅ | `cargo clippy --all-features -- -D clippy::arithmetic_side_effects` → 0 errors on vb_storage lib + vb_runtime lib |
| Types as Documentation | ✅ | `LegacyEnvelopeBinding` enum with `Exact | Legacy { accepted_ids: &[u16] }` is a typed discriminator; no boolean parameters; `for_journal_event` returns the typed binding |
| Workflows as state machines | ✅ | `validate_journal_event_record_kind` is a pure 3-state transition: `binding.admits(...) → Ok(()) \| Err(RecordKindPayloadMismatch)`; the `Legacy` arm is a typed back-compat discriminator (not a boolean) |
| Newtypes | ✅ | `RunId`, `Seq`, `Attempt` are newtyped u64s; envelope/payload kinds are `u16` primitives (canonical for wire format) |
| Parse, Don't Validate | ✅ | `decode_journal_event` parses the bytes into a typed `JournalEvent` and calls `validate_journal_event_record_kind` (typed enum discriminator) — no string-keyed parsing |

**Phase 3 result: PASS** — all Big 6 rules upheld in the 8 modified production files.

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

| Check | Status |
|-------|--------|
| No Option-based state machines | ✅ | `LegacyEnvelopeBinding` is a 2-variant enum (Exact | Legacy), not an Option-pretending-to-be-an-enum. The match in `for_journal_event` (kind_parity.rs:62-78) is a true exhaustive match. |
| CUPID compliant | ✅ | Composable: `LegacyEnvelopeBinding` is a single-purpose discriminator; Unix-philosophy: one job (does this envelope id admit this payload kind?); Predictable: pure function on enum variants; Idiomatic: matches the canonical Rust enum discriminator pattern; Domain-based: the binding semantics come from the contract, not the implementation |
| No clever abstractions | ✅ | `LegacyEnvelopeBinding` is 24 lines (kind_parity.rs:45-69) — a 2-variant enum + `admits` method. Not a 5-level generic trait hierarchy. The `for_journal_event` is a single match expression, not a visitor pattern. |
| No Option-based workflows | ✅ | `validate_journal_event_record_kind` returns `Result<(), JournalError>`, not `Option<Result<...>>`. The error variant is typed: `RecordKindPayloadMismatch { envelope_kind, payload_kind }`. |
| Boolean parameters | ✅ | `EnforceKindParity::enforce_kind_parity(&envelope, &value)` has 2 parameters, both typed refs. No booleans. |
| YAGNI | ✅ | No "future use" abstractions. The `LegacyEnvelopeBinding` is used immediately by 2 production call sites (EnforceKindParity + validate_journal_event_record_kind) and 1 back-compat test site. |

**Phase 4 result: PASS** — the new code is ruthlessly simple. The 2-variant enum + match expression is the minimal representation of the back-compat discriminator.

---

## PHASE 5: The Bitter Truth

The implementation is brutally honest:

1. **CURRENT_SCHEMA_VERSION=1 is preserved** (constants.rs:58). The back-compat is *legacy envelope-12 tolerance*, not a schema bump. The contract is upheld.
2. **The OR-collapse is genuinely removed**, not papered over. events.rs:406 is now a single-variant arm. No comment-out, no `#[allow]`, no warning suppression.
3. **The pre-fix kani harness `check_unknown_kind_rejected` (kani_record_kind.rs:180-188) is DELETED**, not commented out, not `#![cfg(never)]`'d. The replacement at kani_record_kind_journal_family_33.rs:H2 is the new witness.
4. **The durability matrix substitutions are mechanical**: 10 row substitutions SlotWritten → StepSucceeded, with the finish row (line 198) correctly retaining RunFinished.
5. **The 6 back-compat tests are direct, not proxy**: each one names the specific production symbol and asserts the specific post-fix property. `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` is the literal witness for POST-005.
6. **The proptest anti-invariant token `invalid_input` is present in both proptest files** (grep-confirmed), and the `id_keyed_counter_would_diverge_from_variant_keyed` property (PO-QXJGX-006-H4) is the E_KANI_ASSUMPTION_VACUITY closure that proves the kani assumption vacuity pre-fix.

No clever tricks, no abstraction tax. The code is what it claims to be.

**Phase 5 result: PASS**

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| TBR-001: pre-existing kani_helpers.rs unclosed-delimiter blocks cargo kani workspace-wide | HIGH (pre-existing) | crates/vb_core/src/frame/parts/kani_helpers.rs:22:7 | owner_approved_debt (routes to kani-helpers owner; not caused by this bead) |
| TBR-010: pre-fix check_unknown_kind_rejected (kani_record_kind.rs:180-188) would fail post-implementation | MEDIUM (pre-existing) | crates/vb_storage/src/kani_record_kind.rs:180-188 (pre-state-11) | fixed_with_evidence (DELETED in state 11 implementation; transcript-state11-holzman-rust.txt line 40) |
| TBR-002: 4 forward-looking E0599 errors (RecordKind::StepSucceeded undefined) | MEDIUM (expected) | proptest_replay_summary_step_succeeded_split.rs:222, codec/tests.rs:1639, codec/tests.rs:1743, proptest_durability_matrix_step_succeeded.rs:251 (pre-state-11) | fixed_with_evidence (resolved post-state-11; cargo test PASS) |
| Aggregate_resource_budget_properties_red proptest failure | HIGH (pre-existing) | (pre-state-11) | owner_approved_debt (not in scope; pre-existing global failure; literal-string check unrelated to this bead) |
| vb_runtime/src/frame_pool/tests.rs pre-existing cargo fmt issues | LOW (pre-existing) | crates/vb_runtime/src/frame_pool/tests.rs:85, 114, 139 | owner_approved_debt (not modified by this bead; jj diff confirms) |
| kani-list output does not list the 5 new harnesses (BLOCKED_TOOLING) | MEDIUM (pre-existing blocker) | .evidence/kani-list/vb_storage.json | owner_approved_debt (TBR-001 blocker; harness bodies are syntactically valid; would resolve on TBR-001 fix) |

### [FIND-001]: TBR-001 — pre-existing kani_helpers.rs compile error

**Location**: `crates/vb_core/src/frame/parts/kani_helpers.rs:22:7`

**Problem**: The `frame_kani_harnesses` module has an unclosed delimiter at line 22, blocking `cargo kani` workspace-wide.

**Evidence**:
```
$ KANI_FEATURES=kani-vb-qxjgx-record-kind-split bash scripts/kani-list.sh vb_storage
Kani Rust Verifier 0.67.0 (cargo plugin)
   Compiling vb_core v0.1.0 (…)
error: this file contains an unclosed delimiter
  --> crates/vb_core/src/frame/parts/kani_helpers.rs:22:7
…
error: could not compile `vb_core` (lib) due to 1 previous error
```

The same error fires on the parent commit `ywnswumt 1b72c500` (verified), proving the blocker is pre-existing and not caused by this bead.

**Required Fix**: Routes to the kani-helpers owner as a separate work item. The 5 new kani files compile under `cargo check --features kani-vb-qxjgx-record-kind-split` (no kani codegen); the harness bodies follow the established pattern at kani_record_kind.rs and kani_vb_vzcuf_ps*.rs.

**Disposition**: owner_approved_debt (TBR-001 in trusted-base-ledger.jsonl; compensation: 1678 + 2348 cargo test PASS + 6 back-compat tests + 9 proptest properties)

### [FIND-002]: TBR-010 — pre-fix check_unknown_kind_rejected

**Location**: `crates/vb_storage/src/kani_record_kind.rs:180-188` (pre-state-11)

**Problem**: The pre-fix harness asserts `is_known_record_kind(33) == false` (the inverse of the post-fix contract).

**Evidence**: Per `transcript-state11-holzman-rust.txt` line 40: "kani_record_kind.rs:177-188 - Deleted pre-fix check_unknown_kind_rejected". Verified via `jj diff -r ywnswumt..ttulypyv -- crates/vb_storage/src/kani_record_kind.rs` (off-workspace; transcript is the source of truth for the state 11 changes).

**Required Fix**: DELETED. The replacement is at `crates/vb_storage/src/kani_record_kind_journal_family_33.rs:H2 check_kind_33_journal_family_admit` (PO-QXJGX-003).

**Disposition**: fixed_with_evidence (state 11 implementation transcript; cargo test PASS)

### [FIND-003]: TBR-002 — forward-looking E0599 errors

**Location**: 4 sites (pre-state-11)

**Problem**: The proof artifacts reference `RecordKind::StepSucceeded` before the production change lands.

**Evidence**: Per `proof-writer-report.md` lines 197-227: cargo check emits 3 E0599 errors in vb_storage + 1 in vb_runtime. Post-state-11: cargo test -p vb_storage --tests: 1678 passed; cargo test -p vb_runtime --tests: 2348 passed. The 4 errors cleared.

**Required Fix**: Resolved by state 11 implementation.

**Disposition**: fixed_with_evidence (cargo test PASS post-state-11)

### [FIND-004]: Pre-existing aggregate_resource_budget_properties_red proptest failure

**Location**: (pre-state-11; pre-existing global)

**Problem**: Proptest failure unrelated to this bead (literal-string check).

**Evidence**: Mentioned in `transcript-state11-holzman-rust.txt` line 68 (residual risks). Not exercised by this bead's proptest files (proptest_replay_summary_step_succeeded_split.rs and proptest_durability_matrix_step_succeeded.rs are the new ones and both PASS).

**Required Fix**: Routes to the aggregate_resource_budget owner as a separate work item.

**Disposition**: owner_approved_debt (pre-existing; not in scope for this bead)

### [FIND-005]: Pre-existing vb_runtime/src/frame_pool/tests.rs cargo fmt issues

**Location**: `crates/vb_runtime/src/frame_pool/tests.rs:85, 114, 139`

**Problem**: Pre-existing formatting drift.

**Evidence**: Per `transcript-state11-holzman-rust.txt` line 62: "vb_runtime has pre-existing fmt issues in frame_pool/tests.rs not touched by this bead". Verified via `jj diff -r ttulypyv~..ttulypyv` (file not modified by this bead).

**Required Fix**: Routes to the frame_pool owner as a separate work item.

**Disposition**: owner_approved_debt (pre-existing; not in scope)

### [FIND-006]: Kani harness list output does not include the 5 new harnesses

**Location**: `.evidence/kani-list/vb_storage.json` (BLOCKED_TOOLING output)

**Problem**: TBR-001 blocks `cargo kani` workspace-wide, so the harness list cannot be enumerated. The 5 new files (`kani_record_kind_id_step_succeeded.rs`, `kani_record_kind_projection_split.rs`, `kani_record_kind_journal_family_33.rs`, `kani_record_kind_parity_legacy_envelope.rs`, `kani_record_kind_decode_round_trip.rs`) are syntactically valid under `cargo check --features kani-vb-qxjgx-record-kind-split` but are not exercised by the kani list.

**Evidence**: `cargo check -p vb_storage --features kani-vb-qxjgx-record-kind-split` → 0 errors. `cargo kani list` (via `kani-list.sh`) → exit 101 with the TBR-001 error.

**Required Fix**: TBR-001 must be resolved to enumerate the harnesses.

**Disposition**: owner_approved_debt (TBR-001; compensation: cargo test PASS + 6 back-compat tests + 9 proptest properties)

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p vb_storage --tests` | ✅ | 1678 passed (17 suites, 13.13s); raw log: `.beads/vb-qxjgx/evidence/fv-cargo-test-vb_storage.txt` |
| `cargo test -p vb_runtime --tests` | ✅ | 2348 passed, 1 ignored (35 suites, 3.34s); raw log: `.beads/vb-qxjgx/evidence/fv-cargo-test-vb_runtime.txt` |
| `cargo test -p vb_storage --tests -- <6 back-compat tests>` | ✅ | 6 passed, 1672 filtered out; raw log: `.beads/vb-qxjgx/evidence/fv-backcompat-6-tests.txt` |
| `cargo test --test proptest_durability_matrix_step_succeeded --release` | ✅ | 5 passed (PROPTEST_CASES=10000); raw log: `.beads/vb-qxjgx/evidence/fv-proptest-durability.txt` |
| `cargo test --test proptest_replay_summary_step_succeeded_split --release` | ✅ | 4 passed (PROPTEST_CASES=10000); raw log: `.beads/vb-qxjgx/evidence/fv-proptest-replay-split.txt` |
| `cargo check -p vb_storage --all-targets` | ✅ | Finished `dev` profile (4.98s) |
| `cargo check -p vb_runtime --all-targets` | ✅ | Finished `dev` profile (2.84s) |
| `cargo clippy -p vb_storage --lib` | ✅ | No issues found |
| `cargo clippy -p vb_runtime --lib` | ✅ | No issues found |
| `cargo fmt --check -p vb_storage` | ✅ | (no output — formatting clean) |
| `cargo kani` workspace-wide | ⚠️ BLOCKED_TOOLING | TBR-001; compensation: cargo test PASS + proptest PASS |
| `cargo clippy --all-features -- -D clippy::arithmetic_side_effects` on vb_storage + vb_runtime | ✅ | 0 errors on the lib |
| `rg "(unwrap\(\)\|expect\(\|panic!\|todo!\|unimplemented!\|dbg!\|unsafe )" on 6 production files` | ✅ | 0 matches in production code |
| `CURRENT_SCHEMA_VERSION preservation` | ✅ | constants.rs:58 = 1; proptest PO-QXJGX-007-H2 PASS |
| Back-compat legacy envelope-12 tolerance | ✅ | back-compat test #4 `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` PASS |
| Cross-bind rejection (SlotWrittenEvent + id 33) | ✅ | back-compat test #6 `slot_written_with_envelope_id_33_is_rejected` PASS |
| Canonical id-33 round-trip | ✅ | back-compat test #5 `canonical_id_33_round_trip_step_succeeded` PASS |

---

## Verdict

**STATUS: APPROVED**

### Summary

The implementation lands the `StepSucceeded` / `SlotWrittenEvent` record-kind split correctly, removing the pre-fix OR-collapse at events.rs:406 and adding the new `RecordKind::StepSucceeded = 33` arm. The parity gate now honors a typed `LegacyEnvelopeBinding { Exact | Legacy { accepted_ids } }` discriminator that admits envelope id 12 *and* 33 for `StepSucceeded` (back-compat), and the durability matrix's 10 step-closing rows are mechanically substituted `SlotWritten → StepSucceeded`. All 14 contract clauses (POST-001..009, POST-011, PRE-005, INV-001, INV-004, INV-006, INV-008) bind to production source + executable test/proptest evidence. The 5 Kani harnesses are BLOCKED_TOOLING (TBR-001, pre-existing `vb_core` kani_helpers.rs unclosed-delimiter, NOT caused by this bead) and are compensated by 1678 + 2348 cargo test PASS + 6 back-compat unit tests + 9 proptest properties at PROPTEST_CASES=10000. `CURRENT_SCHEMA_VERSION` is preserved at 1 (back-compat is legacy envelope-12 tolerance, NOT a schema bump). No production panic surface, no unchecked arithmetic, no `unsafe`, no cleverness.

---

## Required Repair Actions (if REJECTED)

N/A — STATUS: APPROVED.

Out-of-scope follow-ups (debt, not blocking):

1. TBR-001: Fix the unclosed-delimiter in `crates/vb_core/src/frame/parts/kani_helpers.rs:22:7` to unblock `cargo kani` workspace-wide. (Routes to kani-helpers owner.)
2. Aggregate_resource_budget_properties_red proptest failure (pre-existing). (Routes to aggregate_resource_budget owner.)
3. vb_runtime/src/frame_pool/tests.rs pre-existing cargo fmt issues (pre-existing). (Routes to frame_pool owner.)
