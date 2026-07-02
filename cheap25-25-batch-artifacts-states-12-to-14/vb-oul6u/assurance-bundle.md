# Assurance Bundle — vb-oul6u

bead_id: vb-oul6u
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u
commit_or_change: cheap25-vb-oul6u @ xyxuylsy 14f6eea3 (parent rsvywymk 1d6c017f)
phase: 5 (state 14 closure)

STATUS: APPROVED

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| PRE-001 (`TraceRing::capacity() >= 1`) | `contract.md:15` | `trace.rs::TraceRing::new` enforces `capacity.max(1)` (referenced by INV-004) | `proof-strategy.md` §3.1 — `cargo test trace_ring_fill_pct` 3/3 pass | PASS |
| PRE-002 (`pending_len() <= capacity()`) | `contract.md:16` | `rtrb::RingBuffer` semantics; pinned by RA-003 boundary test (len=cap is allowed) | `proof-review.md` STATUS: APPROVED | PASS |
| PRE-003 (`trace_ring_fill_pct: f32`) | `contract.md:17` | `runtime.rs:112` `ShardsMetricsSnapshotBuilder::build` returns `f32`; field type frozen at `vb_runtime/src/counters.rs:113` | `proof-writer-report.md` and `proof-to-rust-review.md` | PASS |
| PRE-004 (no new crate dep) | `contract.md:18` | Diff at `.beads/vb-oul6u/evidence/diff.patch` modifies only `runtime.rs:21-46` + `608-627`; no `Cargo.toml` changes | black-hat-review PHASE 5 | PASS |
| INV-001 (`trace_ring_fill_pct: f32` field) | `contract.md:31` | Field type frozen; `vb_runtime/src/counters.rs:113` unmodified | black-hat-review PHASE 2 | PASS |
| INV-002 (`collect_metrics` pure `&self`) | `contract.md:32` | Diff at `evidence/diff.patch` adds only a pure helper + arithmetic; no `&mut self`, no async, no I/O | black-hat-review PHASE 2 | PASS |
| INV-003 (`trace_ring_fill_pct ∈ [0.0, 100.0]`) | `contract.md:33` | `trace_ring_fill_pct_boundary_values_are_bit_exact` (RA-003) | `evidence/cargo-test-trace-ring-verifier-rerun.log` line 5 ("... ok") | PASS |
| INV-004 (`f32::from(u32)` replacement, parent-approved deviation) | `contract.md:34` | `u32_to_f32_exact` helper at `runtime.rs:32-46`; bit-equivalence to `(n as f32)` for `n ∈ [0, 2^24)` proven in `evidence/ieee-754-bit-equivalence.log` (2,097,172 cases bit-exact; 2^24 sanity; 1024 boundary tests) | `formal-verification-report.md` §"Parent-Approved Deviation"; black-hat-review PHASE 1 | PASS (parent deviation accepted) |
| INV-005 (no stale `// SAFETY:` block) | `contract.md:35` | Diff at `evidence/diff.patch` removes the 2-line stale `// SAFETY:` comment | black-hat-review PHASE 1 | PASS |
| INV-006 (workspace `as_conversions = "deny"` preserved) | `contract.md:36` | `rg -n "allow\(clippy::as_conversions" crates/vb_runtime/src/runtime.rs` → 0 matches | black-hat-review PHASE 1; `evidence/verifier-runtime-rg-as-conversions.log` | PASS |
| POST-001 (post-conditions on result range) | `contract.md:22` | RA-003 corpus boundary test | `formal-verification-report.md` PASS row PO-OUL6U-RA003-002 | PASS |
| POST-002 (zero `as`-casts + zero `#[allow(...)]` at `runtime.rs:578-588`) | `contract.md:23` | `cargo clippy ... -D clippy::as_conversions` exits 0 | `evidence/clippy-as-conversions-verifier-rerun.log` | PASS |
| POST-003 (bit-identical to `(trace_len as f32)/(trace_capacity as f32)*100.0`) | `contract.md:24` | `evidence/ieee-754-bit-equivalence.log`: 2,097,172 cases bit-exact, 2^24 sanity check, 1024 boundary tests | `formal-verification-report.md` Equivalence Proof §1-5 | PASS |
| POST-004 (clippy command exits 0) | `contract.md:25` | `cargo clippy -p vb_runtime --lib --bins --all-features -- -D clippy::as_conversions` exit 0 | `evidence/clippy-as-conversions-verifier-rerun.log` (active-execution context) | PASS |
| POST-006 (3 RA-003 tests pass) | `contract.md:27` | 3/3 tests: `trace_ring_fill_pct_boundary_values_are_bit_exact`, `trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two`, `trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps` | `evidence/cargo-test-trace-ring-verifier-rerun.log` | PASS |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-OUL6U-LINT-001 | cargo-clippy | `cargo clippy -p vb_runtime --lib --bins --all-features -- -D clippy::as_conversions` | `evidence/clippy-as-conversions-verifier-rerun.log` | PASS (exit 0, zero diagnostics) | none |
| PO-OUL6U-RA003-002 | cargo-test | `cargo test -p vb_runtime --lib trace_ring_fill_pct` | `evidence/cargo-test-trace-ring-verifier-rerun.log` | PASS (3 passed, 0 failed) | none (parent deviation documented) |
| PO-OUL6U-CALLSITE-003 | cargo-check + cargo-test | `cargo check -p vb_runtime --all-targets --all-features` (+ RA-003 corpus triangulation) | `evidence/cargo-check-verifier-rerun.log` + `evidence/cargo-test-trace-ring-verifier-rerun.log` | PASS (exit 0, call-site boundaries 0/50/100 transitively pinned by RA-003 corpus) | none (parent deviation documented) |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| `cargo check -p vb_runtime --all-targets --all-features` | `cargo check -p vb_runtime --all-targets --all-features` | `evidence/cargo-check-verifier-rerun.log` | exit 0 (1 crate compiled, 0.04s) |
| `cargo clippy -p vb_runtime --lib --bins --all-features -- -D clippy::as_conversions` | (same as written in obligation) | `evidence/clippy-as-conversions-verifier-rerun.log` | exit 0 (zero diagnostics) |
| `cargo test -p vb_runtime --lib trace_ring_fill_pct` | (same as written in obligation) | `evidence/cargo-test-trace-ring-verifier-rerun.log` | 3 passed, 0 failed, 0 ignored, 1804 filtered out |
| `rg -n "allow\(clippy::as_conversions" crates/vb_runtime/src/runtime.rs` | (AST scan) | `evidence/verifier-runtime-rg-as-conversions.log` | 0 matches |
| `rg -n " as f32" crates/vb_runtime/src/runtime.rs` | (AST scan) | `evidence/verifier-runtime-rg-as-f32.log` | 2 matches (line 28 + line 617 — documentation comments only, no production-code `as f32`) |
| 2,097,172 power-of-two equivalence test | (offline harness, captured at State 11) | `evidence/ieee-754-bit-equivalence.log` | "Total cases (powers of two): 2097172; Bit-exact: YES" |
| 2^24 sanity check | (offline harness) | `evidence/ieee-754-bit-equivalence.log` | "All 2^24 values match (n as f32): YES" |
| 1024 boundary-value check (empty/full ring) | (offline harness) | `evidence/ieee-754-bit-equivalence.log` | "All 1024 empty-ring values: 0.0; All 1024 full-ring values: 100.0" |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| State 4 proof-plan-review | `.beads/vb-oul6u/proof-plan-review.md` | (approved) | 3 observations, 0 findings (per State 4 dispatch) |
| State 5 proof-writer | `.beads/vb-oul6u/proof-writer-report.md` | (NO_FORMAL_PROOF_WORK_REQUIRED, accepted) | 0 |
| State 6 proof-review | `.beads/vb-oul6u/proof-review.md` | **STATUS: APPROVED** | 0 |
| State 7 proof-to-implementation | `.beads/vb-oul6u/proof-to-rust-map.md` | (produced) | 0 |
| State 7 proof-to-rust-review | `.beads/vb-oul6u/proof-to-rust-review.md` | **STATUS: APPROVED** | 3 observations, 0 findings |
| State 11 holzman-rust implementation | `.beads/vb-oul6u/implementation.md` | COMPLETED_WITH_RESIDUAL_BLOCKER (parent-resolved) | 0 in-scope findings; 1 residual blocker (parent deviation — option (a) accepted) |
| State 12 formal-verification | `.beads/vb-oul6u/formal-verification-report.md` | **STATUS: APPROVED** | 0 |
| State 13 black-hat-review | `.beads/vb-oul6u/black-hat-review.md` | **STATUS: APPROVED** | 0 |

## Findings Disposition

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| (no findings) | n/a | n/a | n/a | n/a |

Three observations exist at the State-7 bridge review
(PF-VB-OUL6U-PTBR-001/002/003) confirming bridge accuracy. Per
`proof-to-rust-review.md` these are non-blocking
confirmations that the proof-to-rust map is correct. They are not
action findings.

The 1 residual blocker from State 11 holzman-rust was
`CONTRACT/CANONICAL_FORM_DEVIATION` (contract INV-004 specifies
`f32::from(u32)`, which is NOT implemented in Rust). The femdation
parent reviewed and selected option (a) per STATE.md:
accept `u32_to_f32_exact` helper as the canonical form. This is
documented in `formal-verification-report.md` §"Parent-Approved
Deviation" and in the in-file annotation at `runtime.rs:614-619`.

Per canonical `finding/v1.disposition` taxonomy, this is not a
finding because the parent explicitly accepted the deviation
before formal-verifier close — it is a contract amendment in
flight, owner-approved by femdation.

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| 7 formal-verifier lanes not_applicable (Verus/Kani/Flux/Loom/Miri/proptest/cargo-fuzz) | `proof-strategy.md` §3.1 + `proof-plan-review.md` §48-57 explicitly classify all 7 as `not_applicable` for a single-file lint remediation. None of these lanes can produce evidence that this bead is required to produce. | n/a (waiver is upstream at State 4) | n/a (closed at proof-strategy) | `proof-waiver-candidates.jsonl` rows W-OUL6U-VERUS-001 through W-OUL6U-FUZZ-007 (all `not_applicable`, `behavior_affecting: false`, approved at State 4) |
| Contract INV-004 amendment (f32::from(u32) → u32_to_f32_exact) | Future contract-maintenance: type-contracts.md:33-34 still references `f32::from(u32)`. | contract-maintenance owner (future bead) | follow-up bead (out of scope for vb-oul6u) | `formal-verification-report.md` §"Parent-Approved Deviation" provides the mathematical equivalence + bit-equivalence log + 3/3 test pass as the in-scope evidence. The contract text is amended in spirit; the literal contract amendment is filed as out-of-scope future debt. |

## Truth Serum Audit

- report: `.beads/vb-oul6u/truth-serum-report.md`
- status: APPROVED
- final-decision: `.beads/vb-oul6u/final-evidence-decision.md`
- final-status: APPROVED
