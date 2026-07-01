# Verifier Lane Matrix — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `proof-planner` (State 4)
> Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`

This matrix maps each `(requirement_id, contract_clause, proof_seed_id)` to its lane profile (required / not_applicable / blocked_tooling). The JSONL companion `verifier-lane-decisions.jsonl` carries the schema-bound rows; this matrix is the narrative form for the reviewer.

## 1. Lane Decision Counts

| Verifier | Total rows | Required | Not applicable | Blocked tooling |
|---|---:|---:|---:|---:|
| `proptest` | 5 | 5 | 0 | 0 |
| `verus` | 5 | 0 | 5 | 0 |
| `kani` | 5 | 0 | 5 | 0 |
| `flux-rs` | 5 | 0 | 5 | 0 |
| `loom` | 1 | 0 | 1 | 0 |
| `miri` | 1 | 0 | 1 | 0 |
| `cargo-fuzz` | 1 | 0 | 1 | 0 |
| **Total** | **23** | **5** | **18** | **0** |

The 5 `required` rows back the 3 actual obligations (PO-qol58-001 is mapped to 4 VLD rows since it cross-cites seed PS-X-001 + PS-D-001), and PO-qol58-002/-003 each map to a `proptest` requirement row paired with a parallel VLD row.

## 2. Per-Seed Lane Matrix

### 2.1 PS-qol58-A-001 — `IpcFrameHeader::encode` canonicalization

Requirement: `REQ-LINT-CANONICALIZE-IPC-HEADER-ENCODE`
Contract: `C-1`
Production source: `crates/vb_ipc/src/frame_types.rs:39-64`

| Verifier | Applicability | VLD ID | Reason |
|---|---|---|---|
| `proptest` | **required** | `VLD-qol58-A-001-proptest` | The verifier surface is `moon run :lint-src` and `cargo test -p vb_ipc --all-features`; proptest owns the cargo test analog in the SKILL enum. |
| `verus` | not_applicable | `VLD-qol58-A-001-verus` | surface_absent — no new Rust-local pure/core invariant; behavior_change=false. Evidence: `contract.md §C-1`, `domain-model.md §1`, `workflow-model.md §2.1`. |
| `kani` | not_applicable | `VLD-qol58-A-001-kani` | superseded_by_other_lane_with_evidence — `crates/vb_ipc/src/kani_ipc_header.rs` already covers encode panic-freedom; per AGENTS.md rule 5, scope trimmed to call-graph blast radius. Evidence: `codebase-map.md §3.3`, `contract.md §5`. |
| `flux-rs` | not_applicable | `VLD-qol58-A-001-flux` | surface_absent — no refinement-type claim; `type-contracts.md §6` confirms zero typestates. Evidence: `type-contracts.md §6`, `domain-model.md §6`. |

### 2.2 PS-qol58-B-001 — `SeededBytes::<N>::new` canonicalization

Requirement: `REQ-LINT-CANONICALIZE-SEEDED-BYTES-NEW`
Contract: `C-2`
Production source: `crates/workspace_tests/src/test_util/seed.rs:17-25`

| Verifier | Applicability | VLD ID | Reason |
|---|---|---|---|
| `proptest` | **required** | `VLD-qol58-B-001-proptest` | The verifier surface is `cargo test -p velvet-ballistics-workspace-tests --lib seed`; proptest owns the cargo test analog. |
| `verus` | not_applicable | `VLD-qol58-B-001-verus` | surface_absent — RNG fill surface is unchanged; existing unit tests at `seed.rs:33-50` cover determinism. Evidence: `contract.md §C-2`, `error-taxonomy.md §1.2`, `workflow-model.md §2.2`. |
| `kani` | not_applicable | `VLD-qol58-B-001-kani` | superseded_by_other_lane_with_evidence — bounded state covered by existing unit tests; per AGENTS.md rule 5. Evidence: `codebase-map.md §5`. |
| `flux-rs` | not_applicable | `VLD-qol58-B-001-flux` | surface_absent — zero typestates; Flux's decidable fragment adds nothing. Evidence: `type-contracts.md §6`, `error-taxonomy.md §1.2`. |

### 2.3 PS-qol58-C-001 — `FixtureBuilder::build_bytes` canonicalization

Requirement: `REQ-LINT-CANONICALIZE-FIXTURE-BUILDER-BUILD-BYTES`
Contract: `C-3`
Production source: `crates/workspace_tests/src/test_util/fixture.rs:52-60`

| Verifier | Applicability | VLD ID | Reason |
|---|---|---|---|
| `proptest` | **required** | `VLD-qol58-C-001-proptest` | The verifier surface is `cargo test -p velvet-ballistics-workspace-tests --lib fixture`; proptest owns the cargo test analog. |
| `verus` | not_applicable | `VLD-qol58-C-001-verus` | surface_absent — RNG fill surface is unchanged; existing capacity-boundary unit tests at `fixture.rs:67-90` cover the bounded state. Evidence: `contract.md §C-3`, `error-taxonomy.md §1.3`, `workflow-model.md §2.3`. |
| `kani` | not_applicable | `VLD-qol58-C-001-kani` | superseded_by_other_lane_with_evidence — bounded state covered by 4 existing unit tests at `fixture.rs:67-90`; per AGENTS.md rule 5. Evidence: `codebase-map.md §5`. |
| `flux-rs` | not_applicable | `VLD-qol58-C-001-flux` | surface_absent — capacity invariant enforced at constructor; Flux adds nothing. Evidence: `type-contracts.md §6`, `error-taxonomy.md §1.3`. |

### 2.4 PS-qol58-D-001 — `lint-src` denylist preservation

Requirement: `REQ-LINT-GATE-PRESERVED`
Contract: `C-4`
Production source: `.moon/tasks/all.yml:46-53`

| Verifier | Applicability | VLD ID | Reason |
|---|---|---|---|
| `proptest` | **required** | `VLD-qol58-D-001-proptest` | The verifier surface is `moon run :lint-src` and `git diff .moon/tasks/all.yml`; proptest is the closest cargo/lint analog. |
| `verus` | not_applicable | `VLD-qol58-D-001-verus` | surface_absent — no Rust function target; claim is YAML-based. Evidence: `contract.md §C-4`, `.moon/tasks/all.yml`. |
| `kani` | not_applicable | `VLD-qol58-D-001-kani` | surface_absent — no Rust MIR surface for a YAML deny-list. Evidence: `contract.md §C-4`. |
| `flux-rs` | not_applicable | `VLD-qol58-D-001-flux` | surface_absent — no Rust refinement predicate possible on YAML. Evidence: `contract.md §C-4`. |

### 2.5 PS-qol58-X-001 — Cross-site aggregate

Requirement: `REQ-LINT-CANONICALIZE-ALL-PROD-SITES`
Contract: `C-1+C-2+C-3+C-4`
Production source: 3 sites as above

| Verifier | Applicability | VLD ID | Reason |
|---|---|---|---|
| `proptest` | **required** | `VLD-qol58-X-001-proptest` | Cross-cite umbrella obligation. The verifier surface is the union of moon run :lint-src + cargo check + cargo test. |
| `verus` | not_applicable | `VLD-qol58-X-001-verus` | surface_absent — same as PS-qol58-{A,B,C,D}-001-verus. |
| `kani` | not_applicable | `VLD-qol58-X-001-kani` | superseded_by_other_lane_with_evidence — pre-existing kani harnesses continue to cover the IPC surface. |
| `flux-rs` | not_applicable | `VLD-qol58-X-001-flux` | surface_absent — zero typestates across all 3 sites. |
| `loom` | not_applicable | `VLD-qol58-X-001-loom` | surface_absent — all 3 sites are synchronous, single-threaded; no async/thread/atomic/channel/lock boundary. Evidence: `boundary-map.md §1.2`, `workflow-model.md §3`. |
| `miri` | not_applicable | `VLD-qol58-X-001-miri` | surface_absent — all sites in `#![forbid(unsafe_code)]` crates; no FFI. Evidence: `hazard-analysis.md §2.3`, `boundary-map.md §2`. |
| `cargo-fuzz` | not_applicable | `VLD-qol58-X-001-cargo-fuzz` | surface_absent — no parser/codec/untrusted-input boundary at the 3 sites. Evidence: `boundary-map.md §2`, `codebase-map.md §3.3`. |

## 3. Why `proptest` and Not `cargo test` Verifier Value

The SKILL enforces `verifier ∈ {verus, kani, flux-rs, loom, miri, cargo-fuzz, proptest}`. This bead's actual gates are `moon run :lint-src`, `cargo check -p vb_ipc --all-targets`, and `cargo test -p velvet-ballistics-workspace-tests --lib`. The closest semantic match in the enum is `proptest`:

- `proptest` in the SKILL owns "cargo test against the existing test surface" per `references/verifier-trigger-matrix.md`.
- The 3 obligations each run an existing unit-test or lint-gate command, not a property-based shrinking campaign.
- The `expected_evidence` for each row uses the actual moon/cargo exit markers (`EXIT=0`, `test result: ok`), not Kani/Verus markers.

This is documented in `proof-strategy.md §2.3` and the per-obligation `command` field carries the actual moon/cargo invocation.

## 4. Required Lane Summary and Obligation Pairing

The 5 `required` lane decisions pair with the 3 obligations:

| VLD ID | Paired Obligations |
|---|---|
| `VLD-qol58-A-001-proptest` | PO-qol58-002 (cargo check + cargo test for vb_ipc), PO-qol58-001 (cross-cite umbrella) |
| `VLD-qol58-B-001-proptest` | PO-qol58-003 (cargo test for workspace_tests), PO-qol58-001 |
| `VLD-qol58-C-001-proptest` | PO-qol58-003, PO-qol58-001 |
| `VLD-qol58-D-001-proptest` | PO-qol58-001 |
| `VLD-qol58-X-001-proptest` | PO-qol58-001, PO-qol58-002, PO-qol58-003 (cross-cite) |

Per `references/lane-decision-guide.md §"Pairing With Proof Obligations"`, every `required` lane has at least one matching obligation; every obligation references a `required` lane decision via the inverse index in `proof-to-implementation-input.md` (which is empty for this bead since all obligations are `behavior_affecting: false`, so no refinement obligations are required).

## 5. Cross-Reference

- `proof-strategy.md` §2, §7 (risk profile and lane selection rationale).
- `verifier-lane-decisions.jsonl` (schema-bound JSONL form).
- `proof-obligations.planned.jsonl` (3 obligations).
- `delivery-scope.jsonl` rows 1, 2, 3, 14, 15, 17, 18.
- `proof-seeds.jsonl` rows 1-5.
- `references/lane-decision-guide.md` (algorithm + self-audit checklist).
