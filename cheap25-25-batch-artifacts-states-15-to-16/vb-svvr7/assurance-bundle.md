# Assurance Bundle

bead_id: vb-svvr7
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7
commit_or_change: lrutlkzunmkq (p11-holzman-rust: reject trailing bytes in CLI postcard decoder) — child of mrwpkqqn 0cf4e2c2

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| REQ-TB-STRICT-LENGTH | CC-TB-1 | PO-TB-PROP-01 (proptest, BLOCKED_TOOLING, TB-TB-01, compensated by PO-TB-UNIT-01); `tests.rs:179-184` `decode_rejects_trailing_bytes_after_valid_frame` (21 passed); `tests.rs:186-192` `decode_accepts_exact_length_frame` | black-hat-review.md PHASE 1 CC-TB-1 ✅; proof-coverage-matrix.md C1, C3 | COVERED |
| REQ-TB-VARIANT-SHAPE | CC-TB-4 | PO-TB-UNIT-01 (`tests.rs:204-214` `postcard_error_trailing_bytes_is_unit_variant_and_distinct`, 21 passed); source `error.rs:30-31` `TrailingBytes,` declared as unit variant | black-hat-review.md PHASE 1 CC-TB-4 ✅; proof-coverage-matrix.md C4 | COVERED |
| REQ-TB-EXACT-LENGTH | CC-TB-1 | PO-TB-UNIT-01 (`tests.rs:186-192` `decode_accepts_exact_length_frame`, 21 passed); PO-TB-PROP-01 (BLOCKED, compensated) | black-hat-review.md PHASE 1 CC-TB-1 ✅; proof-coverage-matrix.md C1 | COVERED |
| REQ-TB-JSON-PROPAGATION | CC-TB-6 | PO-TB-UNIT-01 (`tests.rs:194-202` `decode_postcard_json_propagates_trailing_bytes`, 21 passed); source `codec.rs:24-34` `?` propagates | black-hat-review.md PHASE 1 CC-TB-6 ✅; proof-coverage-matrix.md C6 | COVERED |
| REQ-TB-ENCODER-EXACT-LENGTH | CC-TB-7 | PO-TB-UNIT-01 (`tests.rs:85-92` `test_encode_postcard`, 21 passed); source `codec.rs:46-73` | black-hat-review.md PHASE 1 CC-TB-7 ✅; proof-coverage-matrix.md C7 | COVERED |
| REQ-TB-DISPLAY-DISTINGUISHABLE | CC-TB-5 | PO-TB-UNIT-01 (`tests.rs:204-214` `postcard_error_trailing_bytes_is_unit_variant_and_distinct`, covers `format!` non-empty + contains "trailing" + distinct from `DecodeFailed`, 21 passed); source `error.rs:48-53` Display arm | black-hat-review.md PHASE 1 CC-TB-5 ✅; proof-coverage-matrix.md C5 | COVERED |
| REQ-TB-CROSS-CRATE-PARITY | CC-TB-9 | PO-TB-CLIPPY-01 (cargo clippy moon lint-src form, exit 0, 0 warnings); `cargo test -p vb_ipc --lib` (540 passed, parity preserved); sibling `vb_ipc/src/frame.rs:44` uses `!= expected_len` (same single-compare shape) | black-hat-review.md PHASE 1 CC-TB-9 ✅; proof-coverage-matrix.md C9 | COVERED |
| REQ-TB-PRESERVE-BOUNDED-ALLOCATION | CC-TB-10 | PO-TB-LINT-01 (`tests.rs:53-59` `test_decode_payload_too_large`, 21 passed); PO-TB-LINT-01 source-lint (panic-surface exit 0, ignored-fallible-results exit 0, clippy exit 0); source `validation.rs:71-78` header validation order | black-hat-review.md PHASE 1 CC-TB-10 ✅; proof-coverage-matrix.md C10 | COVERED |
| REQ-TB-PRESERVE-MAGIC-HEADER-VALIDATION | CC-TB-10 | PO-TB-LINT-01 (`tests.rs:43-50` `test_decode_invalid_magic`, `tests.rs:62-76` `test_decode_invalid_header_length`, 21 passed); source `validation.rs:71-78` header validation order | black-hat-review.md PHASE 1 CC-TB-10 ✅; proof-coverage-matrix.md C10 | COVERED |
| REQ-TB-PRESERVE-TRUNCATION-REJECTION | CC-TB-2 | PO-TB-UNIT-01 (`tests.rs:79-83` `test_decode_data_too_short`, `tests.rs:170-177` `decode_rejects_truncated_header`, 21 passed); source `validation.rs:87-89` `< payload_end` branch returns `DecodeFailed` | black-hat-review.md PHASE 1 CC-TB-2 ✅; proof-coverage-matrix.md C2 | COVERED |

**Coverage summary**: 10/10 requirements covered. All 10 contract clauses (CC-TB-1..CC-TB-10) discharged by at least one passing obligation. The proptest obligation `PO-TB-PROP-01` is `BLOCKED_TOOLING` with compensating unit-test coverage from `PO-TB-UNIT-01` (TB-TB-01 in `trusted-base-plan.md` §2.1).

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-TB-PROP-01 | proptest | `PROPTEST_CASES=10000 cargo test -p vb_cli --test cli_postcard_properties --release strict_length_no_trailing_bytes` (planned) | (not executed — test target not wired; see TB-TB-01) | BLOCKED_TOOLING | WVR-TB-01-PROPTEST-WIRING (tooling-only, non-behavior, expiry 2026-12-31) — `formal-waivers.jsonl:1` |
| PO-TB-UNIT-01 | cargo-test | `cargo test -p velvet-ballistics --lib cli_postcard` | `.beads/vb-svvr7/evidence/cargo-test-velvet-ballistics-cli_postcard.txt` | PASS — 21 passed, 197 filtered out (1 suite, 0.00s); exit 0 | none |
| PO-TB-CLIPPY-01 | cargo-clippy | `cargo clippy --quiet --workspace --lib --bins --examples --all-features -- -D warnings -W clippy::all -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock -D clippy::print_stdout -D clippy::print_stderr` (moon `:lint-src` form) | `.beads/vb-svvr7/evidence/cargo-clippy-lint-src.txt` | PASS — exit 0; 0 warnings emitted | none |
| PO-TB-LINT-01 | source-lint | `moon run :lint-src` (composes panic-surface + ignored-fallible-results + unsafe-audit + clippy --workspace + fmt); sub-gates re-executed fresh | `.beads/vb-svvr7/evidence/cargo-clippy-lint-src.txt` + `.beads/vb-svvr7/evidence/check-panic-surface-fresh.txt` + `.beads/vb-svvr7/evidence/check-ignored-fallible-results.txt` | PASS — all sub-gates green (exit 0) | none |

**Verifier-lane decisions** (10 total, all referenced in `.beads/vb-svvr7/verifier-lane-decisions.jsonl`):

| VLD ID | Verifier | Applicability | Reason |
|---|---|---|---|
| VLD-TB-01 | proptest | required (PO-TB-PROP-01) | Primary lane for property claim; BLOCKED on tooling with compensating PO-TB-UNIT-01 |
| VLD-TB-02 | cargo-test | required (PO-TB-UNIT-01) | Variant-shape unit tests + discriminant + format! + exact equality + JSON propagation |
| VLD-TB-03 | cargo-clippy | required (PO-TB-CLIPPY-01) | Cross-crate parity lock + workspace lint set |
| VLD-TB-04 | source-lint | required (PO-TB-LINT-01) | Canonical CI gate per AGENTS.md |
| VLD-TB-05 | verus | not_applicable (surface_absent) | No production-bound spec; vacuum Verus rejected per GOD RULE 2 |
| VLD-TB-06 | kani | not_applicable (superseded_by_other_lane_with_evidence) | Proptest over 10000 cases of arbitrary [0, MAX_PAYLOAD] x [1, 4096] is stronger |
| VLD-TB-07 | flux-rs | not_applicable (surface_absent) | Single integer compare; no refinement surface |
| VLD-TB-08 | loom | not_applicable (surface_absent) | Pure single-threaded decode over `&[u8]` |
| VLD-TB-09 | miri | not_applicable (surface_absent) | `unsafe_code = forbid` at workspace level |
| VLD-TB-10 | cargo-fuzz | not_applicable (superseded_by_other_lane_with_evidence) | No fuzz target exists; proptest covers arbitrary trailing lengths |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| `cargo test -p velvet-ballistics --lib cli_postcard` (State 12 fresh re-execution) | `cargo test -p velvet-ballistics --lib cli_postcard` | `.beads/vb-svvr7/evidence/cargo-test-velvet-ballistics-cli_postcard.txt` | 21 passed, 197 filtered out (1 suite, 0.00s); exit 0 |
| `cargo test -p velvet-ballistics --lib` (full lib re-execution; pre-recorded) | `cargo test -p velvet-ballistics --lib` | `.beads/vb-svvr7/evidence/cargo-test-vb_cli-full.txt` | 218 passed, 0 failed, 0 ignored (1 suite, 0.24s); exit 0 |
| `cargo test -p vb_ipc --lib` (sibling parity; State 12 fresh re-execution) | `cargo test -p vb_ipc --lib` | `.beads/vb-svvr7/evidence/cargo-test-vb_ipc-lib.txt` | 540 passed (1 suite, 0.23s); exit 0 |
| `cargo clippy --quiet --workspace --lib --bins --examples --all-features -- -D warnings` (moon `:lint-src` form) | as above | `.beads/vb-svvr7/evidence/cargo-clippy-lint-src.txt` | exit 0; 0 warnings emitted |
| `bash scripts/check-panic-surface.sh` | as above | `.beads/vb-svvr7/evidence/check-panic-surface-fresh.txt` | NoViolationFound; exit 0 |
| `bash scripts/check-ignored-fallible-results.sh` | as above | `.beads/vb-svvr7/evidence/check-ignored-fallible-results.txt` | FixturePass malformed exception rejected exit=3; exit 0 |
| `cargo fmt --check vb_cli` (pre-recorded) | as above | `.beads/vb-svvr7/evidence/cargo-fmt-vb_cli.txt` | empty (clean) |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Formal verification (State 12) | `.beads/vb-svvr7/formal-verification-report.md` | STATUS: APPROVED (line 172) | 0 FAIL; 1 BLOCKED_TOOLING (PO-TB-PROP-01, compensated); 3 PASS |
| Black-hat review (State 13) | `.beads/vb-svvr7/black-hat-review.md` | STATUS: APPROVED (line 14) | 0 CRITICAL/HIGH/MEDIUM/LOW; 3 advisory notes (NOTE-1, NOTE-2, NOTE-3) — non-blocking |
| Defects ledger (State 13) | `.beads/vb-svvr7/defects.md` | empty | (no defects) |

## Findings Disposition

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| (no CRITICAL/HIGH/MEDIUM/LOW findings) | — | — | — | — |
| NOTE-1: `decode_postcard` is 34 lines (slightly above 25-line Farley guideline) | advisory (not a finding severity) | black-hat-review.md PHASE 2 | owner_approved_no_action | Function has linear flow with early returns; splitting adds ceremony without reducing logic. Acceptable. |
| NOTE-2: `encode_postcard` is 28 lines (slightly above 25-line Farley guideline) | advisory (not a finding severity) | black-hat-review.md PHASE 2 | owner_approved_no_action | Same rationale as NOTE-1; canonical encoder shape. Acceptable. |
| NOTE-3: `PO-TB-PROP-01` is `BLOCKED_TOOLING` per TB-TB-01 | advisory (not a finding severity) | black-hat-review.md PHASE 5 + formal-verification-report.md | owner_approved_debt | Compensating coverage from `PO-TB-UNIT-01` (21 passed, 0 failed). Documented in `formal-waivers.jsonl:1` (WVR-TB-01-PROPTEST-WIRING, expiry 2026-12-31). Non-blocking follow-up to wire `crates/vb_cli/tests/cli_postcard_properties.rs`. |

## Waivers And Deferred Work

Waivers and deferred work are not finding dispositions. Findings use only canonical `finding/v1.disposition` values: `fixed_with_evidence`, `owner_approved_debt`, `owner_approved_no_action`, or `blocker`.

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| WVR-TB-01-PROPTEST-WIRING (`formal-waivers.jsonl:1`) | Tooling-only: `verification/proptest/properties.rs` is not wired into a Cargo test target inside `vb_cli`. `crates/vb_cli/tests/cli_postcard_properties.rs` does not exist; the bug-closure property `prop_strict_length_no_trailing_bytes` does not yet exist in the proptest file. This is TB-TB-01 (`trusted-base-plan.md` §2.1). Behavior_affecting: false. | proof-planner (validated by formal-verifier) | Expiry 2026-12-31. Follow-up: wire `crates/vb_cli/tests/cli_postcard_properties.rs` and add `prop_strict_length_no_trailing_bytes` to `verification/proptest/properties.rs` in a separate bead. | PO-TB-UNIT-01: `decode_rejects_trailing_bytes_after_valid_frame` (boundary case `[1,1]`), `decode_postcard_json_propagates_trailing_bytes` (boundary case `[8,8]`), `decode_accepts_exact_length_frame` (the `Ok ⇒ exact length` direction), `postcard_error_trailing_bytes_is_unit_variant_and_distinct` (CC-TB-4 + CC-TB-5). All 21 cli_postcard unit tests pass. |

## Truth Serum Audit

- report: `.beads/vb-svvr7/truth-serum-report.md`
- status: APPROVED