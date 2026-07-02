# Proof-Test-Source Alignment: vb-qol58

## Alignment Matrix

| Proof ID | Source Ref | Behavior Test Ref | Ledger Row | Result | Status |
|----------|------------|-------------------|------------|--------|--------|
| PO-qol58-001 | `crates/vb_ipc/src/frame_types.rs:41`, `crates/workspace_tests/src/test_util/seed.rs:23`, `crates/workspace_tests/src/test_util/fixture.rs:58`, `.moon/tasks/all.yml:51` (deny-list) | unit-test inventory at seed/fixture tests; lint-src deny-list at `.moon/tasks/all.yml:51` | `PO-qol58-001-LEAD` | PASS | aligned |
| PO-qol58-002 | `crates/vb_ipc/src/frame_types.rs:41::IpcFrameHeader::encode` | `crates/vb_ipc/src/frame_types.rs::tests::{roundtrip_encode_decode, reject_bad_magic, reject_bad_version}` | `PO-qol58-002-LEAD` | PASS | aligned |
| PO-qol58-003 | `crates/workspace_tests/src/test_util/seed.rs:23::SeededBytes::<N>::new`, `crates/workspace_tests/src/test_util/fixture.rs:58::FixtureBuilder::build_bytes` | `seed.rs::tests::{seeded_bytes_determinism, seeded_bytes_different_seeds, seeded_bytes_zero_capacity}`, `fixture.rs::tests::{zero_capacity_rejected, valid_capacity_accepted, max_capacity_boundary, over_max_capacity_rejected}` | `PO-qol58-003-LEAD` | PASS | aligned |

## Requirement Alignment

| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Command | Ledger Result | Status |
|-------------|----------|---------------|-------------|--------------------|-------------------------|---------|---------------|--------|
| REQ-LINT-CANONICALIZE-ALL-PROD-SITES | PO-qol58-001 | RO-qol58-001 | frame_types.rs:41, seed.rs:23, fixture.rs:58, .moon/tasks/all.yml:51 | 7 named unit tests + .moon deny-list | — (no Kani/Verus/Flux/Loom in scope) | `moon run :lint-src` | PASS | aligned |
| REQ-LINT-CANONICALIZE-IPC-HEADER-ENCODE | PO-qol58-002 | RO-qol58-002 | frame_types.rs:41::IpcFrameHeader::encode | frame_types.rs::tests::{roundtrip_encode_decode, reject_bad_magic, reject_bad_version} | `crates/vb_ipc/src/kani_ipc_header.rs`, `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs`, `crates/vb_ipc/src/kani_ipc_decode_order.rs` (pre-existing, `superseded_by_other_lane_with_evidence`) | `rustup run nightly-2026-04-28 cargo check --quiet -p vb_ipc --all-targets --all-features` | PASS | aligned |
| REQ-LINT-CANONICALIZE-SEEDED-BYTES-NEW+REQ-LINT-CANONICALIZE-FIXTURE-BUILDER-BUILD-BYTES | PO-qol58-003 | RO-qol58-003 | seed.rs:23::SeededBytes::<N>::new, fixture.rs:58::FixtureBuilder::build_bytes | seed.rs::tests::{seeded_bytes_determinism, seeded_bytes_different_seeds, seeded_bytes_zero_capacity}; fixture.rs::tests::{zero_capacity_rejected, valid_capacity_accepted, max_capacity_boundary, over_max_capacity_rejected} | — (no Kani/Verus/Flux/Loom in scope) | `rustup run nightly-2026-04-28 cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features` | PASS | aligned |

## Test Inventory (cargo test -p velvet-ballistics-workspace-tests --lib --all-features)

| Test | Source | Phase |
|------|--------|-------|
| `seeded_bytes_determinism` | `crates/workspace_tests/src/test_util/seed.rs::tests` | determinism regression |
| `seeded_bytes_different_seeds` | `crates/workspace_tests/src/test_util/seed.rs::tests` | collision resistance |
| `seeded_bytes_zero_capacity` | `crates/workspace_tests/src/test_util/seed.rs::tests` | N == 0 short-circuit |
| `zero_capacity_rejected` | `crates/workspace_tests/src/test_util/fixture.rs::tests` | FixtureCapacity::new(0) → Err |
| `valid_capacity_accepted` | `crates/workspace_tests/src/test_util/fixture.rs::tests` | FixtureCapacity::new(100) → Ok |
| `max_capacity_boundary` | `crates/workspace_tests/src/test_util/fixture.rs::tests` | FixtureCapacity::new(MAX) → Ok |
| `over_max_capacity_rejected` | `crates/workspace_tests/src/test_util/fixture.rs::tests` | FixtureCapacity::new(MAX+1) → Err |
| (11 sibling tests) | `crates/workspace_tests/src/test_util/*::tests` | test target siblings |

**Summary:** 18 tests run; 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out.

## Verus Production-Binding Audit

| Verifier | Production-Binding Status |
|----------|---------------------------|
| verus | N/A — zero `verifier: verus` obligations in `proof-obligations.planned.jsonl` (all 5 verus lane decisions are `not_applicable` in `verifier-lane-decisions.jsonl`); `scripts/check-verus-production-binding.sh` exit 2 from "verification/verus directory does not exist" is the canonical N/A signal per `formal-verifier` skill workflow step 2 (no Verus spec to bind → no VACUUM risk by construction) |
| kani | N/A — pre-existing Kani harnesses continue to cover the IPC panic-freedom surface; no new harness was created; production-binding N/A by lane omission |
| flux-rs | N/A — no refinement-type predicate introduced; `type-contracts.md §6` confirms zero typestates |
| loom | N/A — all 3 sites are synchronous, single-threaded; no concurrency boundary per `boundary-map.md §1.2` |
| miri | N/A — all sites live in `#![forbid(unsafe_code)]` crates |
| cargo-fuzz | N/A — no parser/codec/untrusted-input boundary at the 3 sites |

## Refinement-Harness Inventory (pre-existing, not introduced)

| Harness | Purpose | Relation to Bead |
|---------|---------|------------------|
| `crates/vb_ipc/src/kani_ipc_header.rs` | panic-freedom of `IpcFrameHeader::encode` / `decode` | unrelated; site spelling is invisible to harness |
| `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` | oversize payload rejection | unrelated; unchanged post-refactor |
| `crates/vb_ipc/src/kani_ipc_decode_order.rs` | decode-order invariants | unrelated; unchanged post-refactor |

These are verifier harnesses (non-`#[cfg(test)]` model checks), **not** behavior tests, per `proof-to-rust-review.md §"Criterion 3"`. They are catalogued for traceability; they are not in scope for behavior-test parity because this bead is `behavior_affecting: false` per `proof-plan-review.md`.

## Status Summary

- **3/3 alignment rows `aligned`.**
- **3/3 ledger rows `PASS`.**
- **0 waivers** (formal-waivers.jsonl is 0 bytes; canonical-empty SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`).
- **0 trust markers** introduced (trusted-base-ledger.jsonl 0 bytes).
- **0 RRO rows** (zero-RRO is the honest disposition for a `behavior_affecting: false` set per `proof-to-rust-review.md STATUS: APPROVED`).
