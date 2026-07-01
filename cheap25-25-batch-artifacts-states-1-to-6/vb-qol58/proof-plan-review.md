---
reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: proof-plan-reviewer-vb-qol58-state4b
planner_invocation_id: go-skill-vb-qol58-state1
review_state: 4
reviewed_at: 2026-07-01T17:00:00Z
status: APPROVED
---

# Proof Plan Review: vb-qol58

## Review Metadata

- **Bead**: vb-qol58 — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
- **Reviewer Skill**: proof-plan-reviewer
- **Reviewer Invocation**: `proof-plan-reviewer-vb-qol58-state4b`
- **Planner Invocation** (state1 controller): `go-skill-vb-qol58-state1`
- **Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`
- **State**: 4 (proof-plan-reviewer, sub-state 4b)
- **Outcome**: APPROVED

## Reviewed Artifacts

| Artifact | Hash (sha256) | Status |
|----------|---------------|--------|
| `proof-strategy.md` | `518c6cb959b604bf3e1faf36e8e9c64e04e5d3319887b8d3b6fb14cf54f17029` | reviewed |
| `verifier-lane-decisions.jsonl` | (23 rows, all `verifier-lane-decision/v1`) | reviewed |
| `verifier-lane-matrix.md` | `45f4f4fbe08d3bbb9eb7418c6b6b45823297368f3d65a05b189ce90f80a0dc6d` | reviewed |
| `proof-coverage-matrix.md` | `3aa4b5598ec43ddebdf2542c167983d09d524d9192b35e8c87a80650adf59169` | reviewed |
| `proof-obligations.planned.jsonl` | `63f333fc2cedcf87bbcf7f1fe63bc8c64571d441bcab3482b81aa065e6b54a38` (3 rows) | reviewed |
| `trusted-base-plan.md` | `11f955d90585ee9882582b1713d693ab89c775a8ce2289f033ceb9249f355eed` | reviewed |
| `waiver-candidates.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (empty, SHA-256 of zero bytes) | reviewed |
| `proof-seeds.jsonl` | `f37104350bddf1469644709cf784529d98a4765228fec7609844829967393b15` | reviewed |
| `traceability-matrix.jsonl` | `13fa5bbf629968811e38c0cb0e115ba12babcec901621dd940c97842d9fc3d37` | reviewed |
| `contract.md` | `b4203a2c689baf9f14f6354ffe462b65f4c033dae611777e2eb7b286a169e0b5` | reviewed |
| `domain-model.md` | `eb81a184944544f033a6cb4367933da5fde6aa864af5296a97d32db8ecdf8652` | reviewed |
| `error-taxonomy.md` | `209c949f9347c6e9e9847d51b89bd03276fe97408bf2596a14706d924e3b0f957` | reviewed |
| `type-contracts.md` | `5f9e4c65fa2d8f24118a610304f99800050f79827296382a642f61c576b63fd4` | reviewed |
| `workflow-model.md` | `bd545f15fbaceed2e9f2cdc4ca520bd9a1ac44834e24f9bed0d8276361fc9a15` | reviewed |
| `boundary-map.md` | `91689dce1afbe33f4be2dadfa637bdd36984613991d4dfecae805c0034e2fe69` | reviewed |
| `hazard-analysis.md` | `31310f40b09d4e9514161ae0fb7a23119cb2d2470ff192ce588d779917a760e0` | reviewed |
| `codebase-map.md` | `4a98816294492fcb77c90a6e416fcfec0480fff95586f771aebda722f2008228` | reviewed |
| `delivery-scope.jsonl` | `4821edab7b125f871289989fc492d6c4401e70223d200aeff64897fd9ada8806` | reviewed |
| `.moon/tasks/all.yml` | `423e84fa22c28ad863a089a7e4ae2c6dfce4ae827f5db0d2cea991fca1f6134d` | reviewed (cross-cited) |

All evidence-ref SHA-256 hashes in `verifier-lane-decisions.jsonl` were cross-verified to match the artifacts in `.beads/vb-qol58/`. Production-line citations (`frame_types.rs:41`, `seed.rs:23`, `fixture.rs:58`) were read live from the isolated workspace and confirmed to contain the asserted patterns.

## User Brief Alignment

The user brief explicitly stated:
> "Verify: 3 production-line edits, source-lint + cargo-check + cargo-test lanes only, no formal Verus/Kani/Flux required."

The proof plan satisfies this brief precisely:

| User Requirement | Plan Mapping |
|---|---|
| 3 production-line edits | `delivery-scope.jsonl` rows 1, 2, 3; verified live at `frame_types.rs:41`, `seed.rs:23`, `fixture.rs:58` |
| source-lint lane | `moon run :lint-src` at `.moon/tasks/all.yml:46-53` → PO-qol58-001 (verifier: proptest) |
| cargo-check lane | `rustup run nightly-2026-04-28 cargo check --quiet -p vb_ipc --all-targets --all-features` → PO-qol58-002 (verifier: proptest) |
| cargo-test lane | `rustup run nightly-2026-04-28 cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features` → PO-qol58-003 (verifier: proptest) |
| No formal Verus/Kani/Flux | All 15 verus/kani/flux-rs lane decisions are `not_applicable` with concrete evidence refs |

## Review Summary

### Lane Decision Coverage: PASS

- 23 lane decisions (VLD-qol58-{A,B,C,D,X}-001-{verus,kani,flux-rs,proptest,loom,miri,cargo-fuzz}) covering all 5 proof seeds.
- 5 `required` lanes (all `proptest`): one per proof seed, matching the user brief.
- 18 `not_applicable` lanes: 5 verus + 5 kani + 5 flux-rs (per-site) + 1 loom + 1 miri + 1 cargo-fuzz (cross-site only).
- Every `not_applicable` row cites concrete SHA-256 evidence refs (artifact hashes verified).
- `limitation_kind` is set correctly: `surface_absent` for verus/flux/loom/miri/cargo-fuzz rows; `superseded_by_other_lane_with_evidence` for kani rows (the pre-existing kani harnesses in `vb_ipc/src/kani_*.rs` already cover the panic-freedom surface).
- Per `verification-lane-policy.md`, the conditional lanes (loom, miri, cargo-fuzz) are correctly scoped to the cross-site aggregate seed (PS-X-001) only — the per-site seeds do not introduce concurrency, unsafe, or parser concerns at the single-site level.

### Non-Applicability Evidence: PASS

All 18 `not_applicable` decisions cite concrete evidence refs:

- **Verus × 5**: `surface_absent` — no new Rust-local pure/core invariant introduced; the 3 edits are pure spelling changes. Evidence refs cite `contract.md §C-1..§C-4`, `domain-model.md §1`, `workflow-model.md §2.{1,2,3}`, `error-taxonomy.md §1.{1,2,3}`, `hazard-analysis.md §2` (all hashes verified).
- **Kani × 5**: `superseded_by_other_lane_with_evidence` — pre-existing kani harnesses at `crates/vb_ipc/src/kani_ipc_header.rs`, `kani_ipc_header_rejects_oversize.rs`, `kani_ipc_decode_order.rs` already cover the IPC encode/decode panic-freedom surface. Per AGENTS.md rule 5 (No Blind Verification Mutations), verification scope is trimmed to the call-graph blast radius of 3 production lines.
- **Flux-rs × 5**: `surface_absent` — `type-contracts.md §6` confirms zero typestates; the `FixtureCapacity::new` constructor already enforces the `≤ MAX_CAPACITY` invariant at the construction site. Flux's decidable fragment adds nothing to a spelling change.
- **Loom × 1 (cross-site only)**: `surface_absent` — all 3 sites are synchronous, single-threaded; no async/thread/atomic/channel/lock boundary per `boundary-map.md §1.2` and `workflow-model.md §3`.
- **Miri × 1 (cross-site only)**: `surface_absent` — all sites live in `#![forbid(unsafe_code)]` crates; no FFI, no raw pointers, no MaybeUninit. Miri's UB-detection role is inapplicable by construction.
- **Cargo-fuzz × 1 (cross-site only)**: `surface_absent` — the 3 sites are not parser/codec/untrusted-input boundaries; the seed changes only writer-target borrow expressions, not decode paths. Pre-existing kani harnesses cover the fuzzable surface symbolically.

### Proof Obligation Schema: PASS

- All 3 obligations use schema `proof-obligation/v1` with all required fields present (verified via `jq` against the schema in `proof-schemas.md`).
- Required fields populated: `schema_version`, `id`, `requirement_id`, `contract_clause`, `domain_claim`, `risk`, `risk_tags`, `verifier`, `artifact`, `target`, `command`, `workdir`, `expected_evidence`, `assumptions`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, `required`, `behavior_affecting`, `mode`, `owner_state`, `rerun_from`, `status`.
- No legacy alias fields (`layer`, `checker`, `claim`) detected.
- `target` is canonical in all obligations: `crates/vb_ipc::frame_types::IpcFrameHeader::encode`, `crates::workspace_tests::test_util::seed::SeededBytes::new`, `crates::workspace_tests::test_util::fixture::FixtureBuilder::build_bytes`.
- `command`, `workdir`, `expected_evidence`, `assumptions`, `model_bounds`, `tool_metadata` are populated.
- `behavior_affecting: false` and `required: true` and `mode: verify-proof` and `owner_state: 4` and `rerun_from: 4` consistent across all 3 obligations.

### Verus Production Binding: N/A (auto-satisfied)

The plan emits zero Verus obligations; the production-binding discipline is automatically satisfied by lane omission. The `production_binding` validation gate (per the skill instructions) does not apply because no `proof-obligation/v1` row has `verifier: verus`. There is no vacuum-Verus-proof risk.

### TLA+ Compliance: N/A

TLA+ is removed per upstream mandate. No TLA+ obligations, lane decisions, or waived lanes appear in this plan.

### Waiver Candidates: PASS

- `waiver-candidates.jsonl` is empty (0 bytes; SHA-256 `e3b0c4429...`).
- No behavior-affecting waivers. No waivers at all.
- `behavior_affecting: false` on all 3 obligations is consistent with no waivers needed.

### Trusted Base Plan: PASS

- 3 trust notes (TB-qol58-lint-denylist-preserved, TB-qol58-encode-byte-layout-preserved, TB-qol58-testutil-rng-determinism).
- All 3 are `behavior_affecting: false` (these are assumptions, not trust markers).
- All 3 have concrete compensating evidence (existing unit tests, `.moon/tasks/all.yml` byte-identity check).
- `assumptions` arrays in the 3 obligations are 1-to-1 with the 3 trust notes.
- Zero `assume`, `axiom`, `admit`, `sorry`, `external_body`, `#[trusted]`, `#[ignore]`, `opaque` markers introduced by this bead.

### Non-Vacuity: PASS

- All 3 obligations use existing unit-test or lint-gate commands (no property-based shrinking campaign).
- The `expected_evidence` field for each obligation cites concrete tool markers (`EXIT=0`, `test result: ok`) and concrete test names (`seeded_bytes_determinism`, `zero_capacity_rejected`, etc.).
- No `cover!`-only, no `assert(true, ...)`, no vacuum models.
- The 3 obligations are `behavior_affecting: false`; no `E_BEHAVIOR_WAIVER` is possible and none are present.

### Bridge Planning: N/A

Per `proof-strategy.md §10` and `proof-coverage-matrix.md §3`, all 3 obligations are `behavior_affecting: false`, so no `rust-refinement-obligation/v1` rows are required. The proof-to-implementation bridge produces zero rows. This is correct because:
- The 3 production-line edits are spelling changes with byte-equivalent machine code at the borrow level.
- The 7 `cursor.write_uXX<LittleEndian>` calls in `frame_types.rs:42-62` are byte-identical pre/post refactor.
- The `StdRng::seed_from_u64(seed)` constructor is unchanged in both `seed.rs:21` and `fixture.rs:56`.
- The `if N == 0 { return None }` guard at `seed.rs:18-20` and the `FixtureCapacity::MAX_CAPACITY = 1 MiB` bound at `fixture.rs:11` are preserved verbatim.

### Review Provenance: PASS

- Reviewer invocation: `proof-plan-reviewer-vb-qol58-state4b`
- Planner invocation (state1 controller): `go-skill-vb-qol58-state1`
- Independent, non-self-approved. Reviewer invocation_id is distinct from all planner invocation_ids in the agent-invocation-ledger.jsonl.
- No reviewer fields (e.g., `reviewer_disposition`, `reviewer_invocation_id`, `reviewer_note`) are stamped into any planner artifact (`verifier-lane-decisions.jsonl`, `proof-strategy.md`, `proof-obligations.planned.jsonl`, `trusted-base-plan.md`, `proof-coverage-matrix.md`, `verifier-lane-matrix.md`).
- Workspace ledger validated: 2 prior entries (state1 controller, state2 explore); this review appends the state4 review row to maintain the hash chain.

### Verifier-Lane-Review JSONL: PASS

- 23 review rows (VLR-qol58-{A,B,C,D,X}-001-{proptest,verus,kani,flux-rs,loom,miri,cargo-fuzz}) written with `verifier-lane-review/v1` schema.
- All 23 lanes have `reviewer_disposition: accepted`.
- Planner and reviewer invocation IDs populated on every row.
- `owner_state: 4`, `status: reviewed`.
- Schema validated via `jq -c .`; no parse errors; all required fields present.

### Source-Citation Anti-Hallucination: PASS

The 3 production-line citations were read live from the isolated workspace and confirmed:

| Citation | Line Content (pre-refactor) | Status |
|---|---|---|
| `crates/vb_ipc/src/frame_types.rs:41` | `let mut cursor = std::io::Cursor::new(&mut bytes[..]);` | verified |
| `crates/workspace_tests/src/test_util/seed.rs:23` | `rng.fill(&mut bytes[..]);` | verified |
| `crates/workspace_tests/src/test_util/fixture.rs:58` | `rng.fill(&mut vec[..]);` | verified |

The `.moon/tasks/all.yml:51` deny-list was also verified to contain the cited `-D clippy::indexing_slicing`, `-D clippy::get_unwrap`, `-D clippy::unwrap_used`, `-D clippy::expect_used`, `-D clippy::panic`, `-D clippy::string_slice`, `-D clippy::arithmetic_side_effects`, `-D clippy::as_conversions`, and other flags.

## Findings

| ID | Code | Severity | Disposition | Description |
|----|------|----------|-------------|-------------|
| FIND-001 | E_LANE_VERIFIER_ENUM_MAPPING | low | `owner_approved_no_action` | The SKILL mandates `verifier ∈ {verus, kani, flux-rs, loom, miri, cargo-fuzz, proptest}`. This bead's actual gates are `moon run :lint-src`, `cargo check`, and `cargo test`. The plan maps all 3 obligations to `proptest` (the closest enum match per the planner's narrative) and documents the mapping rationale in `proof-strategy.md §2.3` and `verifier-lane-matrix.md §3`. This is a known schema-vs-actual mismatch in the upstream go-skill enum, not a planner-side fault. The plan is honest about the mapping. No action required. |

No blocker findings. No high-severity findings. No medium-severity findings.

## Pre-Existing Plan Strengths

The following aspects of the plan are sound and do not require changes:

1. **Schema integrity**: Every `proof-obligation/v1` row and every `verifier-lane-decision/v1` row has all required fields per the canonical schema in `proof-schemas.md`. No legacy alias fields.
2. **Production-binding discipline**: All 3 obligations name production symbols (not file-only refs) in the `target` field.
3. **Trusted-base ledger**: 3 trust notes, all `behavior_affecting: false`, all with concrete compensating evidence. Zero `assume` / `axiom` / `admit` / `#[trusted]` markers.
4. **Non-applicability evidence**: 18 `not_applicable` rows with concrete SHA-256 evidence refs (all verified). No weak reasoning like "not needed" or "too hard".
5. **Command reproducibility**: All 3 obligation commands are absolute-pathed, use `rustup run nightly-2026-04-28` (matches `.moon/tasks/all.yml:51`'s toolchain), and use `--all-targets`/`--lib --all-features` flags as appropriate.
6. **Behavior-preservation argument**: The plan correctly argues that `bytes.as_mut_slice()` is byte-equivalent to `&mut bytes[..]` at the byte-stream level, that the `cursor.write_uXX<LittleEndian>` call sequence is unchanged, that the RNG seed and fill window are unchanged, and that the `if N == 0` guard and `MAX_CAPACITY` bound are preserved verbatim.
7. **Cross-cite discipline**: PO-qol58-001 (cross-site aggregate) correctly cross-cites all 5 VLD proptest rows; PO-qol58-002 cross-cites VLD-qol58-A-001-proptest + VLD-qol58-X-001-proptest; PO-qol58-003 cross-cites VLD-qol58-B/C-001-proptest + VLD-qol58-X-001-proptest. Every `required` lane has at least one matching obligation; every obligation references a `required` lane decision.
8. **No self-approval**: No `reviewer_disposition`, `reviewer_invocation_id`, or `reviewer_note` fields appear in any planner artifact.
9. **Test-side patterns explicitly out of scope**: `delivery-scope.jsonl` rows 4-13 enumerate test-side `clippy::indexing_slicing`-class patterns in `crates/vb_ipc/src/{tests,frame/tests,frame_types/tests,client/tests,server/impl_tests}.rs`, `crates/vb_cli/tests/*`, and `crates/workspace_tests/tests/*` as out of default scope per `contract.md §7`. The 3 obligations are scoped narrowly to the 3 production sites per `delivery-scope.jsonl:14` (RECOMMENDED DEFAULT SCOPE). Follow-up beads can pick up the test-side cleanups; this bead does not need to.

## Verdict

The proof plan is complete, precise, and implementation-bound. All 23 lane decisions are justified with concrete evidence and aligned with the user brief ("3 production-line edits, source-lint + cargo-check + cargo-test lanes only, no formal Verus/Kani/Flux required"). All 3 obligations have explicit commands, bounds, assumptions, and expected evidence. The trusted base is planned with compensating evidence. No behavior-affecting waivers exist. No bridge planning is required (all obligations are `behavior_affecting: false`). The 3 production-line citations are verified to exist as cited. The plan is ready for proof-writer (State 5).

**STATUS: APPROVED**

## Next Steps

1. **State 5 (proof-writer)**: Execute the 3 planned obligations using exact commands. The `moon run :lint-src` log, `cargo check` log, and `cargo test` log are captured at `.evidence/vb-qol58/`. (No `proof-obligations.written.jsonl` rows are required because no formal-verifier artifacts (Verus/Kani/Flux/Loom/Miri/cargo-fuzz) are written; the 3 obligations are pure cargo/moon gates.)
2. **State 6 (proof-reviewer)**: Validate the captured `moon run :lint-src`, `cargo check`, `cargo test` exit-0 logs against the `expected_evidence` field in each obligation.
3. **State 7 (proof-to-implementation)**: Materialize zero `rust-refinement-obligation/v1` rows (all 3 obligations are `behavior_affecting: false`).
4. **State 11 (holzman-rust)**: Apply the 3 production-line edits:
   - `crates/vb_ipc/src/frame_types.rs:41`: `Cursor::new(&mut bytes[..])` → `Cursor::new(bytes.as_mut_slice())`
   - `crates/workspace_tests/src/test_util/seed.rs:23`: `rng.fill(&mut bytes[..])` → `rng.fill(bytes.as_mut_slice())`
   - `crates/workspace_tests/src/test_util/fixture.rs:58`: `rng.fill(&mut vec[..])` → `rng.fill(vec.as_mut_slice())`
5. **State 12 (formal-verifier)**: Run the 3 commands and emit `verification-ledger/v1` rows with `result: PASS` for PO-qol58-001, PO-qol58-002, PO-qol58-003.

---

**Reviewer**: proof-plan-reviewer
**Invocation ID**: proof-plan-reviewer-vb-qol58-state4b
**Timestamp**: 2026-07-01T17:00:00Z

## STATUS: APPROVED
