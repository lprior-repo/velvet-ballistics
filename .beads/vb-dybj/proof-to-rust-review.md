# Proof-to-Rust Bridge Review — vb-dybj State 7

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-dybj-state7-bridge-001
bead_id: vb-dybj
state: 7
sublane: bridge-review
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-dybj
source_checkout: /home/lewis/src/velvet-ballistics
reviewed_bridge_invocation_id: proof-to-implementation-vb-dybj-state7-001
reviewed_artifacts:
  - .beads/vb-dybj/proof-to-rust-map.md
  - .beads/vb-dybj/rust-refinement-obligations.jsonl
  - .beads/vb-dybj/contract.md
  - .beads/vb-dybj/proof-review.md
  - .beads/vb-dybj/proof-findings.jsonl
  - .beads/vb-dybj/trusted-base-ledger.jsonl
host_session_id: velvet-ballistics-vb-dybj-femdation-2026-05-27
started_at: 2026-05-27T22:00:00.000000+00:00

## Provenance

- **Bridge artifact**: `proof-to-implementation-vb-dybj-state7-001` (ledger sequence 18, `.beads/vb-dybj/agent-invocation-ledger.jsonl`).
- **Parent invocation**: `proof-reviewer-vb-dybj-state6-005` (ledger sequence 17, State 6 approval with test-first trust boundaries).
- **This review**: `proof-reviewer-vb-dybj-state7-bridge-001` — distinct skill invocation from proof-to-implementation, no self-approval.
- **Bridge writer**: `proof-to-implementation` skill (not `proof-reviewer`). Provenance is clean.

## Bridge Mapping Analysis

### Completeness

All 18 proof obligations from `proof-obligations.planned.jsonl` are mapped in `proof-to-rust-map.md` and `rust-refinement-obligations.jsonl`. The bridge coverage summary table (lines 335-354 of `proof-to-rust-map.md`) confirms each obligation has source refs, behavior test refs, and refinement harness refs.

- **12 Owner State 6 obligations**: All mapped to concrete `crates/*/src/` source lines, planned behavior tests in `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`, and refinement harnesses.
- **6 Owner State 8 obligations**: All mapped with source refs to production types and planned test sub-modules.
- **No missing obligations** from the planned obligation set.

### Source Ref Presence

Each obligation cites exact file:line-range references to production code:

| Obligation | Source Refs Present | Verified |
|---|---|---|
| PO-VB-DYBJ-001 | `crates/vb_core/src/ids/mod.rs:229-244`, `:9-30`, `:278-283` | YES — RunId constructor/accessor/ZERO |
| PO-VB-DYBJ-002 | `crates/vb_core/src/ids/mod.rs:12-16`, `:65`, `:229-231` | YES — RunId serde derive + ZERO |
| PO-VB-DYBJ-004 | `crates/vb_core/src/ids/mod.rs:340-356` | YES — WorkflowDigest struct + methods |
| PO-VB-DYBJ-005 | `crates/vb_core/src/ids/mod.rs:340-342` | YES — WorkflowDigest [u8; 32] struct |
| PO-VB-DYBJ-007 | `crates/vb_storage/src/records.rs:136-190`, `:192-224` | YES — RecordKind enum + id() |
| PO-VB-DYBJ-008 | `crates/vb_storage/src/records.rs:139-148`, `:195-222`, `:136` | YES — Selected RecordKind variants |
| PO-VB-DYBJ-010 | `crates/vb_storage/src/codec/header.rs:26-58`, `payload.rs:56-82`, `error/mod.rs:123-125` | YES — Codec header/payload/error |
| PO-VB-DYBJ-012 | Same as PO-VB-DYBJ-010 + `error/mod.rs:117-125` | YES — Codec validation chain |
| PO-VB-DYBJ-013 | `ids/mod.rs:340-342`, `codec/mod.rs:35-44`, `payload.rs:56-82` | YES — WorkflowDigest + codec decode |
| PO-VB-DYBJ-014 | `ids/mod.rs:340-342`, `codec/mod.rs:35-44`, `error/mod.rs:127-128` | YES — WorkflowDigest + decode error |
| PO-VB-DYBJ-015 | `ids/mod.rs:340-342`, `codec/mod.rs:35-44` | YES — WorkflowDigest + codec |
| PO-VB-DYBJ-016 | `restate_postcard_newtype_compat_tests.rs` (golden constants) | YES — Fixture lifecycle mapped to Rust test constants |
| PO-VB-DYBJ-003,006,009,011,017,018 | Per-contract source refs | YES — Owner State 8 obligations with concrete source mapping |

**Verdict**: No missing source refs. All obligations reference real, grep-able file:line targets in the source checkout.

### Behavior Test Ref Presence

Every behavior-affecting obligation (PO-VB-DYBJ-001 through PO-VB-DYBJ-017) maps to a planned test in `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`. The test sub-modules are:

| Test Sub-Module | Obligations Covered |
|---|---|
| `run_id` | PO-VB-DYBJ-001, 002, 003 |
| `workflow_digest` | PO-VB-DYBJ-004, 005, 006 |
| `record_kind` | PO-VB-DYBJ-007, 008, 009 |
| `missing_bytes` | PO-VB-DYBJ-010, 011, 012 |
| `trailing_bytes` | PO-VB-DYBJ-013, 014, 015 |
| `migration_required` | PO-VB-DYBJ-016, 017 |

PO-VB-DYBJ-018 is non-behavior-affecting (source scan policy check) — properly listed without behavior test refs.

Existing tests in production crates are correctly cited as reference points:
- `crates/vb_core/src/ids/mod.rs:507-516` — RunId ZERO/MAX unit tests
- `crates/vb_core/src/ids/mod.rs:603-615` — WorkflowDigest roundtrip unit tests

**Verdict**: No missing behavior test refs. Every behavior-affecting obligation has planned independent behavior tests in the bead target file. The planned tests do not duplicate existing coverage but extend it with Postcard golden-byte fixtures and compat-specific assertions.

### Harness/Test Overlap Check

The bridge correctly distinguishes refinement harnesses from behavior tests:

- **Refinement harnesses** (Verus/Kani/Flux/fuzz/TLA+): Owned by State 6 proof reviewer, provide formal/semi-formal evidence.
- **Behavior tests** (proptest + BDD): Owned by State 8 implementation, provide executable specification assertions.

No obligation uses a harness as its sole behavior test. Each obligation that has a harness also has a planned independent behavior test. The responsibilities are clearly separated:
- Harnesses prove internal soundness / bounded properties.
- Behavior tests prove the contract from the outside via public APIs.

**Verdict**: No harness/test overlap confusion. Clean separation.

### TLA+ Rust Event/State Mapping

PO-VB-DYBJ-016 (migration lifecycle) provides explicit temporal-to-Rust mappings (lines 216-230 of `proof-to-rust-map.md`):

| TLA+ State/Transition | Rust Realization |
|---|---|
| `FixtureFrozen` | Frozen byte constants in `restate_postcard_newtype_compat_tests.rs` |
| `EncodedCompared` | Postcard encode + `assert_eq!` on fixture bytes |
| `MigrationRequired` | Migration-required assertion messages (PO-VB-DYBJ-017) |
| `Accepted` | Golden fixture assertions pass |
| `bytesChanged ∧ ¬migrationNamePresent → MigrationRequired` | Fixture byte drift without migration docs → test failure |

This mapping is complete and honest. The TLA+ model is correctly characterized as temporal design evidence, not Rust implementation proof. The actual Rust realization is the `migration_required` behavior test (PO-VB-DYBJ-017).

**Verdict**: TLA+ claims are properly bridged to Rust event/state realization via golden fixture constants and migration-required assertions.

### Delivery Scope Compliance

The bridge mapping respects the delivery scope:
- **NO** production code source refs outside the test-first bead scope.
- **NO** claims that Verus standalone models constitute production-bound proof.
- **NO** claims that blocked Kani harnesses have been verified.
- The test target file `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` is correctly identified as the primary deliverable.

## Unresolved Bridge Gaps (Honest Documentation)

The bridge documents 4 unresolved gaps at lines 358-364 of `proof-to-rust-map.md`:

1. **Verus production binding (PO-VB-DYBJ-001/004/007)**: Standalone `*Model` types not mechanically bound to production `exec fn`. Deferred to State 12. **ACCEPTED**: This is an honest trust boundary already accepted at State 6 under test-first bead rules.
2. **Flux toolchain gap (PO-VB-DYBJ-005)**: `flux_rs` crate unresolved. Deferred to State 12. **ACCEPTED**: Documented as tool-integration gap in TB-VB-DYBJ-003.
3. **vb_storage Kani compile blockers (PO-VB-DYBJ-008/010)**: Unrelated `kani_recovery_hydrate.rs` blocks selected harness compilation. Deferred to State 12. **ACCEPTED**: Documented in TB-VB-DYBJ-002.
4. **TLA+ not Rust implementation proof (PO-VB-DYBJ-016)**: Temporal model provides design evidence only. **ACCEPTED**: Correctly bridged to migration-required behavior tests with explicit Rust event/state mapping.

These gaps are fully consistent with the State 6 proof review approval, which accepted the 6 ACCEPTED_TRUST_BOUNDARY dispositions for these exact obligations.

## Trust Marker Alignment

The bridge references 7 trust markers (TB-VB-DYBJ-001 through TB-VB-DYBJ-007) from `trusted-base-ledger.jsonl`:

| Trust Marker | Status in Bridge | Status in Ledger |
|---|---|---|
| TB-VB-DYBJ-001 | Referenced for PO-VB-DYBJ-001/004/007 | `pending-proof-reviewer`, `active` |
| TB-VB-DYBJ-002 | Referenced for PO-VB-DYBJ-002/010/013 | `pending-proof-reviewer`, `active` |
| TB-VB-DYBJ-003 | Referenced for PO-VB-DYBJ-005 | `pending-proof-reviewer`, `active` |
| TB-VB-DYBJ-004 | Referenced for PO-VB-DYBJ-007/008 | `pending-proof-reviewer`, `active` |
| TB-VB-DYBJ-005 | Referenced for PO-VB-DYBJ-012/015 | `pending-proof-reviewer`, `active` |
| TB-VB-DYBJ-006 | Referenced for PO-VB-DYBJ-016 | `pending-proof-reviewer`, `active` |
| TB-VB-DYBJ-007 | Referenced for PO-VB-DYBJ-018 | `pending-proof-reviewer`, `active` |

All trust markers maintain `pending-proof-reviewer` status — correct for State 7 bridge review. State 12 will re-evaluate.

**Verdict**: Bridge trust marker references are accurate and consistent with the trusted-base-ledger.

## rust-refinement-obligations.jsonl Validation

All 18 entries in `rust-refinement-obligations.jsonl` conform to the `rust-refinement-obligation/v1` schema:

- Each entry has `schema_version`, `id`, `proof_id`, `requirement_id`, `contract_clause`.
- `rust_target` is specified for each obligation.
- `source_refs` are populated with concrete file:line references.
- `behavior_test_refs` are populated (except RRO-VB-DYBJ-018 which correctly has empty array as non-behavior-affecting).
- `refinement_harness_refs` match the harness artifact paths in `proof-to-rust-map.md`.
- `refinement_claim` describes the claim in prose.
- `verifier` field matches the tool: `verus`, `kani`, `flux-rs`, `proptest`, `cargo-fuzz`, `tla-plus`, `source-scan`.
- `evidence_command` is specified with exact command for each obligation.
- `mapping_status` is `planned` (12) or `mapped_existing` (6).
- `required` is `true` for all 18.
- `owner_state` is `6` (12) or `8` (6).
- Cross-reference consistency: `proof_id`, `requirement_id`, and `contract_clause` values are internally consistent.

No schema violations detected.

## Verdict

**STATUS: APPROVED**

The bridge mapping is complete, honest, and actionable:

1. **18/18 obligations mapped** with concrete source refs, behavior test refs, and refinement harness refs.
2. **Source refs are real and grep-able** — all reference production code in `crates/` with exact line ranges.
3. **Behavior tests are planned and distinct** from refinement harnesses — clean separation of proof evidence from executable specification.
4. **TLA+ model is bridged** to Rust event/state realization via golden fixture constants and migration-required assertions.
5. **Trust boundaries are documented** and consistent with State 6 approval.
6. **No self-approval** — bridge writer was `proof-to-implementation`, reviewer is `proof-reviewer`.
7. **No missing source refs, behavior test refs, or TLA+ mappings**.

The 4 unresolved bridge gaps are honestly documented and match the ACCEPTED_TRUST_BOUNDARY dispositions from the State 6 proof review. These gaps do not block test planning (State 8) or test writing (State 9), because the test-only deliverable (`restate_postcard_newtype_compat_tests.rs`) does not require production code changes.

The bridge is ready to proceed to State 8 (test planning).

---
Bridge review completed. No findings.
