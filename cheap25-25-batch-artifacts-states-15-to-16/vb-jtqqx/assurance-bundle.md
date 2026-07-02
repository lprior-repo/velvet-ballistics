# Assurance Bundle

bead_id: vb-jtqqx
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx
commit_or_change: rqywwymq f2aadf8a (state11 commit;
                  +217/-26 in
                  crates/workspace_tests/tests/journal_side_index_contracts.rs)
bead_title: Tests: make side-index malformed-key tests decode malformed keys (P1)
controller: femdation
host_session: femdation-cheap25-batch
bundle_state: 14
status_at_state_11: COMPLETE (holzman-rust, transcript-state11.txt)
status_at_state_12: PASS (formal-verifier, formal-verification-report.md)
status_at_state_13: APPROVED (black-hat-reviewer, black-hat-review.md)
status_at_state_14: APPROVED (evidence-packaging, this bundle)

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| Each PO-008 proptest body calls decode_storage_key against a malformed payload and asserts on a typed KeyDecodeError variant | SIDEX-MAL-001 | PO-MAL-001 (proptest, 11 passed in journal_side_index_contracts, 3 named proptests with 128 cases each, 12 distinct decoder-call surfaces) | verifier-lane-decisions.jsonl VLD-jtqqx-001 (proptest, required, accepted); formal-verification-report.md; black-hat-review.md PHASE 1 | PASS |
| All proptest strategies drive at least one payload shape (no `_`-discard) | SIDEX-MAL-002 | PO-MAL-001 (truncate_len in 1u8..=12u8 consumed at line 242; _extra_bytes in 1u8..=10u8 consumed at line 321, 399); PO-MAL-002 (clippy on the in-scope file is clean: 0 lints) | verifier-lane-decisions.jsonl VLD-jtqqx-008, 009, 015; formal-verification-report.md; black-hat-review.md PHASE 3 | PASS |
| Per-variant decoder branch coverage | SIDEX-MAL-003 | PO-MAL-001: KeyLengthMismatch (3 prefixes), InvalidRunId (3 variants), EmptyKey (1), UnknownPrefix (1); ReservedSeqSentinel not asserted (correct) | verifier-lane-decisions.jsonl VLD-jtqqx-010, 019, 020, 021, 022; formal-verification-report.md PO-MAL-001 | PASS |
| JOURNAL_KEY_PROPTEST_CASES = 128 preserved | SIDEX-MAL-004 | PO-MAL-002: line 37 unchanged; test runs 128 cases per proptest | verifier-lane-decisions.jsonl VLD-jtqqx-023; formal-verification-report.md | PASS |
| #![forbid(unsafe_code)] preserved | SIDEX-MAL-005 | PO-MAL-002: line 27 unchanged; file-level forbid lint | verifier-lane-decisions.jsonl VLD-jtqqx-024; formal-verification-report.md | PASS |
| No unwrap/expect/panic/todo/unimplemented/dbg! in PO-008 block | SIDEX-MAL-006 | PO-MAL-002: 0 forbidden constructs in PO-008 block (awk scan confirms) | verifier-lane-decisions.jsonl VLD-jtqqx-013; black-hat-review.md PHASE 3 | PASS |
| PO-008 block does not call FjallJournal::has_*_index_entry / temp_journal / KeyspaceScanPolicy::* | SIDEX-MAL-007 | PO-MAL-001: PO-008 block imports only `vb_storage::keys::{decode_storage_key, index_*_key}` and `vb_storage::KeyDecodeError`; no Fjall, no I/O | verifier-lane-decisions.jsonl VLD-jtqqx-014; black-hat-review.md PHASE 1 | PASS |
| Bounded to one test file | SIDEX-MAL-008 | `jj diff --stat`: 1 file changed (journal_side_index_contracts.rs); no Cargo.toml, no Cargo.lock, no vb_storage/**, no other test file | state-11 transcript; black-hat-review.md PHASE 1 | PASS |
| KeyLengthMismatch { prefix, expected, actual } field surfacing correct | SIDEX-MAL-009 | PO-MAL-001: 7 distinct surface forms checked (3 prefixes × multiple actual lengths); prefix field is the byte actually present | verifier-lane-decisions.jsonl VLD-jtqqx-017; formal-verification-report.md | PASS |
| Per-variant run==0 payload | SIDEX-MAL-010 | PO-MAL-001: each of 3 proptests has run==0 shape; decoder branches at keys.rs:400-402, 412-414, 423-425 exercised | verifier-lane-decisions.jsonl VLD-jtqqx-010, 019; black-hat-review.md PHASE 1 | PASS |
| Per-variant within-family prefix mismatch | SIDEX-MAL-011 | PO-MAL-001: action (vec![0x30;13]), status (vec![0x32;18]), workflow (covered via UnknownPrefix shape) | verifier-lane-decisions.jsonl VLD-jtqqx-020; black-hat-review.md PHASE 1 | PASS |
| EmptyKey exercised at least once | SIDEX-MAL-012 | PO-MAL-001: action test (d) calls decode_storage_key(&[]) → Err(KeyDecodeError::EmptyKey) | verifier-lane-decisions.jsonl VLD-jtqqx-021; black-hat-review.md PHASE 1 | PASS |
| UnknownPrefix exercised at least once | SIDEX-MAL-013 | PO-MAL-001: workflow test (d) calls decode_storage_key(&vec![0xFF;13]) → Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF }) | verifier-lane-decisions.jsonl VLD-jtqqx-022; black-hat-review.md PHASE 1 | PASS |
| Truncated length in [1, expected) | SIDEX-MAL-014 | PO-MAL-001: truncate_len in 1u8..=12u8 keeps truncated length in [1, 13); workflow literal 11-byte is in [1, 13) | verifier-lane-decisions.jsonl VLD-jtqqx-018; black-hat-review.md PHASE 1 | PASS |
| _extra_bytes wired into payload | SIDEX-MAL-015 | PO-MAL-001 + PO-MAL-002: status (line 323) and workflow (line 401) use overlong.resize(N + extra, 0u8) | verifier-lane-decisions.jsonl VLD-jtqqx-009; black-hat-review.md PHASE 1 | PASS |
| ReservedSeqSentinel not asserted | SIDEX-MAL-016 | PO-MAL-001: 0 matches for ReservedSeqSentinel in PO-008 block | verifier-lane-decisions.jsonl VLD-jtqqx-012; black-hat-review.md PHASE 1 | PASS |
| JournalError::KeyCapacity not asserted | SIDEX-MAL-017 | PO-MAL-001: 0 matches for KeyCapacity in PO-008 block | verifier-lane-decisions.jsonl VLD-jtqqx-011; black-hat-review.md PHASE 1 | PASS |
| KeyDecodeError imported via public re-export | SIDEX-MAL-018 | PO-MAL-001 + PO-MAL-002: `use vb_storage::{..., KeyDecodeError}` at line 33; re-export at vb_storage/src/lib.rs:202 | verifier-lane-decisions.jsonl VLD-jtqqx-016; black-hat-review.md PHASE 1 | PASS |
| Verification-ledger.jsonl has 2 rows (PO-MAL-001, PO-MAL-002) | state-12 | verification-ledger.jsonl: 2 rows, both PASS, with raw evidence, exit codes, sha256 | formal-verification-report.md; black-hat-review.md | PASS |
| Formal-waivers.jsonl: 6 non-behavior waivers, 0 behavior-affecting | state-12 | formal-waivers.jsonl: 6 rows, all behavior_affecting=false, all status=approved, all with matching ledger_result_ref | formal-verification-report.md; black-hat-review.md | PASS |
| Black-hat review: STATUS: APPROVED with 0 findings | state-13 | black-hat-review.md: STATUS: APPROVED; defects.md: empty | (this bundle) | PASS |
| Pre-existing global failures (vb_compile, vb_core, workspace_tests round-9) out of scope | state-12, 13 | formal-verification-report.md#pre-existing-global-failures; identical on parent commit rsvywymk | state-11 transcript; black-hat-review.md PHASE 5 | DOCUMENTED (out of scope) |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-MAL-001 (decoder-rejection) | proptest | `cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` (and PROPTEST_CASES=128, --release, named 3-test filter) | `verification-ledger.jsonl:vl-jtqqx-001`; `evidence/state12_*.log` | PASS (11 tests, 3 named proptests, 12 distinct surface forms, 1536 decoder invocations at 128 cases × 3 proptests) | none |
| PO-MAL-002 (structural preservation) | clippy | `cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` (after cargo clean) | `verification-ledger.jsonl:vl-jtqqx-002`; `evidence/state12_clippy_clean.log` | PASS (in-scope file: 0 lints; 152 package-scope pre-existing in OTHER test files, identical on parent rsvywymk) | none |
| WC-jtqqx-001 (verus not_applicable) | n/a (surface_absent) | n/a (no Verus spec exists for this P1) | `formal-waivers.jsonl:FW-jtqqx-001` | WAIVED (non-behavior) | FW-jtqqx-001 |
| WC-jtqqx-002 (kani not_applicable) | n/a (surface_absent) | n/a (no Kani harness for this P1) | `formal-waivers.jsonl:FW-jtqqx-002` | WAIVED (non-behavior) | FW-jtqqx-002 |
| WC-jtqqx-003 (flux-rs not_applicable) | n/a (risk_out_of_scope) | n/a (no refinement types in scope) | `formal-waivers.jsonl:FW-jtqqx-003` | WAIVED (non-behavior) | FW-jtqqx-003 |
| WC-jtqqx-004 (loom not_applicable) | n/a (surface_absent) | n/a (no concurrent surface in scope) | `formal-waivers.jsonl:FW-jtqqx-004` | WAIVED (non-behavior) | FW-jtqqx-004 |
| WC-jtqqx-005 (miri not_applicable) | n/a (surface_absent) | n/a (no unsafe in scope) | `formal-waivers.jsonl:FW-jtqqx-005` | WAIVED (non-behavior) | FW-jtqqx-005 |
| WC-jtqqx-006 (cargo-fuzz not_applicable) | n/a (superseded_by_other_lane_with_evidence) | n/a (proptest + canonical fixture pattern) | `formal-waivers.jsonl:FW-jtqqx-006` | WAIVED (non-behavior) | FW-jtqqx-006 |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| journal_side_index_contracts (default budget) | `cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` | `evidence/state12_journal_side_index_contracts.log` | 11 passed (0.42s) |
| journal_side_index_contracts (named 3 proptests) | `cargo test ... -- index_action_key_decode_error_on_short_input index_status_key_decode_error_on_wrong_length index_workflow_key_decode_error_on_wrong_length` | `evidence/state12_three_po008.log` | 3 passed, 8 filtered out (0.00s) |
| journal_side_index_contracts (full budget 128 cases) | `PROPTEST_CASES=128 cargo test ...` | `evidence/state12_journal_side_index_contracts_128cases.log` | 11 passed (0.79s) |
| journal_side_index_contracts (release profile) | `cargo test ... --release` | `evidence/state12_journal_side_index_contracts_release.log` | 11 passed (0.11s) |
| cargo check (in-scope compile) | `cargo check -p velvet-ballistics-workspace-tests --all-targets` | `evidence/state12_cargo_check_workspace_tests.log` | Finished (0.07s) |
| cargo check (all features) | `cargo check -p velvet-ballistics-workspace-tests --all-targets --all-features` | `evidence/state12_cargo_check_workspace_tests_all.log` | Finished (0.08s) |
| cargo check (decoder compile) | `cargo check -p vb_storage` | `evidence/state12_cargo_check_vb_storage.log` | Finished (0.03s) |
| cargo fmt | `cargo fmt -p velvet-ballistics-workspace-tests --check` | (n/a) | exit 0 |
| check-panic-surface | `bash scripts/check-panic-surface.sh` | (n/a) | NoViolationFound, exit 0 |
| forbidden-scan | `bash scripts/forbidden-scan.sh` | (n/a) | PASS — no forbidden patterns |
| check-test-integrity | `bash scripts/check-test-integrity.sh` | (n/a) | PASS base=@- |
| check-source-length | `bash scripts/check-source-length.sh` | (n/a) | test_top_level scanned=348 warn=6 over_limit=0 |
| check-workspace-assertions | `bash scripts/check-workspace-assertions.sh` | (n/a) | exit 0 |
| check-ignored-fallible-results | `bash scripts/check-ignored-fallible-results.sh` | (n/a) | exit 0 (2 pre-existing DISCARD-006 in vb_runtime out of scope) |
| check-stepstate-matrix | `bash scripts/check-stepstate-matrix.sh` | (n/a) | PASS |
| check-error-exhaustiveness | `bash scripts/check-error-exhaustiveness.sh` | (n/a) | exit 0 (4 fuzz-target warnings pre-existing in fuzz/** out of scope) |
| check-hot-cold-forbidden-apis | `bash scripts/check-hot-cold-forbidden-apis.sh` | (n/a) | ScanSummary: violations=0, justified=0 |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Explore (state 2) | `.beads/vb-jtqqx/codebase-map.md`, `delivery-scope.jsonl` | completed | (none) |
| proof-planner (state 4) | `.beads/vb-jtqqx/proof-strategy.md` (implied via plan-review) | completed | (none) |
| proof-plan-reviewer (state 4b) | `.beads/vb-jtqqx/proof-plan-review.md` | STATUS: APPROVED | (none) |
| holzman-rust (state 11) | `.beads/vb-jtqqx/implementation.md`, `transcript-state11.txt` | completed | (none) |
| formal-verifier (state 12) | `.beads/vb-jtqqx/formal-verification-report.md`, `verification-ledger.jsonl`, `formal-waivers.jsonl`, `transcript-state12.txt` | STATUS: PASS for both PO-MAL-001 and PO-MAL-002 in the in-scope test file | (none) |
| black-hat-reviewer (state 13) | `.beads/vb-jtqqx/black-hat-review.md`, `defects.md`, `transcript-state13.txt` | STATUS: APPROVED | 0 findings |
| evidence-packaging (state 14) | `.beads/vb-jtqqx/assurance-bundle.md` (this), `truth-serum-report.md`, `final-evidence-decision.md` | (this bundle) | (none) |

## Findings Disposition

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| (none) | — | — | — | — |

The black-hat review at state 13 found 0 findings. The formal-verifier at
state 12 found 0 failures (PASS for both PO-MAL-001 and PO-MAL-002 in
the in-scope test file). The proof-plan-reviewer at state 4b found
0 findings.

## Waivers And Deferred Work

Waivers and deferred work are not finding dispositions. Findings
must use only canonical `finding/v1.disposition` values:
`fixed_with_evidence`, `owner_approved_debt`,
`owner_approved_no_action`, or `blocker`.

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| FW-jtqqx-001 (verus) | No production source change in scope; the decoder at `keys.rs:346-434` is read-only; the contract is already bound at the type level via `KeyDecodeError` (a `#[non_exhaustive]` enum at `error/key_decode.rs:8-31`) | proof-planner | 2026-12-31 | ledger_result_ref vl-jtqqx-001 (PO-MAL-001 PASS); `crates/vb_storage/src/keys.rs:1` `#![forbid(unsafe_code)]`; `delivery-scope.jsonl:2` status: decoder_unchanged_read_only |
| FW-jtqqx-002 (kani) | Decoder is a pure match-based function with no loops, recursion, or unsafe indexing (`keys.rs:281-295, 346-434`); Kani would add zero coverage; future Kani harness captured as PS-MAL-019 (proof-seeds.jsonl:19) | proof-planner | 2026-12-31 | ledger_result_ref vl-jtqqx-001 (PO-MAL-001 PASS); `proof-seeds.jsonl:19` PS-MAL-019 future Kani scope-up |
| FW-jtqqx-003 (flux-rs) | No refinement types in scope; the contract uses `KeyDecodeError` directly (a sum type), not `flux_rs::refined_by::*`; Flux would add zero coverage | proof-planner | 2026-12-31 | ledger_result_ref vl-jtqqx-001 (PO-MAL-001 PASS); `crates/vb_storage/src/error/key_decode.rs:8-31` no `#[refined_by]` annotations |
| FW-jtqqx-004 (loom) | PO-008 proptest bodies are single-threaded; no `Arc`, `Mutex`, channels, or `Send`/`Sync` markers; proptest seeds run sequentially | proof-planner | 2026-12-31 | ledger_result_ref vl-jtqqx-001 (PO-MAL-001 PASS); `boundary-map.md` no concurrency surface in scope |
| FW-jtqqx-005 (miri) | Both the test file and the decoder carry `#![forbid(unsafe_code)]`; zero unsafe, FFI, raw pointers, or `MaybeUninit` in scope; Miri is a no-op on safe-only code | proof-planner | 2026-12-31 | ledger_result_ref vl-jtqqx-002 (PO-MAL-002 PASS); `crates/workspace_tests/tests/journal_side_index_contracts.rs:27` `#![forbid(unsafe_code)]` |
| FW-jtqqx-006 (cargo-fuzz) | Proptest's `JOURNAL_KEY_PROPTEST_CASES = 128` budget already provides randomized malformed-payload coverage; future cargo-fuzz target captured as PS-MAL-020 (proof-seeds.jsonl:20); canonical fixture at `crates/vb_storage/src/preview/tests.rs:111-180` already exercises the parser through a real `KeyspaceScanPolicy` path | proof-planner | 2026-12-31 | ledger_result_ref vl-jtqqx-001 (PO-MAL-001 PASS); `proof-seeds.jsonl:20` PS-MAL-020 future fuzz scope-up; `crates/vb_storage/src/preview/tests.rs:111-180` canonical fixture |
| PS-MAL-019 (future Kani) | Out of scope for this P1 test-only repair; would add bounded-exhaustiveness coverage over 2^L input space for `decode_storage_key` | future bead | follow-up | `proof-seeds.jsonl:19` PS-MAL-019; `traceability-matrix.jsonl:19` status: out_of_scope_followup |
| PS-MAL-020 (future cargo-fuzz) | Out of scope for this P1 test-only repair; would add hostile-input coverage for `decode_storage_key` | future bead | follow-up | `proof-seeds.jsonl:20` PS-MAL-020; `traceability-matrix.jsonl:20` status: out_of_scope_followup |
| Pre-existing global failures (vb_compile compile errors, vb_core admission proptest, workspace_tests strict-admission test, edge_frame_pool / resource_frame_pool round-9, moon ci unrelated lanes) | Pre-existing on parent commit `rsvywymk`; identical pattern on baseline verification; out of scope for this P1 test-only repair | other beads (P0/P1 carryover) | follow-up | `state12_cargo_test_workspace.log`, `state12_cargo_test_workspace_excl_vb_compile.log`, `state12_moon_ci.log`, `state12_clippy_parent.log` (parent baseline) |

## Truth Serum Audit

- report: `.beads/vb-jtqqx/truth-serum-report.md`
- status: APPROVED

## Hash Anchors

For tamper-evidence cross-referencing:

| Artifact | sha256 |
|---|---|
| `.beads/vb-jtqqx/formal-verification-report.md` | 00d0c864c5dd975c0f06e8768485bb082baa4a6bc2b7dc337aae3cca8e7ffe44 |
| `.beads/vb-jtqqx/verification-ledger.jsonl` | 0ad733f07f2569d44ea29a2529bac8b0d4948d35c35b3d103c96c39cd9417cb8 |
| `.beads/vb-jtqqx/formal-waivers.jsonl` | 2ad03aca84d7617e25787cb1be1cb7ecdcbdbf866379b20dd2ec24a4e630e134 |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs` | d5964cb789ce98aaf297e6df63ea9ba614f777deabeb2cc234b528c7c2e1b663 |
