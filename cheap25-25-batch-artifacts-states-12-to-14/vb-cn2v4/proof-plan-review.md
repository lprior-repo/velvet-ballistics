# Proof Plan Review: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Review Provenance

- **reviewer_skill:** `proof-plan-reviewer`
- **reviewer_invocation_id:** `femdation:vb-cn2v4:p4b:reviewer:v1`
- **planner_invocation_id:** `femdation:vb-cn2v4:p4:planner:v1` (from `proof-strategy.md` L9)
- **review_state:** `4b` (proof-plan-reviewer stage; sits between state 4 planner and state 5 writer)
- **host_session_id:** `femdation-cheap25-batch` (matches state-1 ledger entry)
- **workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4`
- **controller:** `femdation` (parent dispatcher; this is a direct child)

Reviewer invocation ID and planner invocation ID differ (independent
provenance). The reviewer is a direct child of `femdation`, not a sub-agent.

## Reviewed Artifacts (with SHA-256 content hashes)

| Artifact | Path | sha256 |
|---|---|---|
| `proof-strategy.md` | `.beads/vb-cn2v4/proof-strategy.md` | `851d35ba614c3211c77d6d3e1239007872d2083e265545702570f58bab5bf2df` |
| `verifier-lane-decisions.jsonl` | `.beads/vb-cn2v4/verifier-lane-decisions.jsonl` | `110576fd816d26ec0ea2ec33e26bd2fe55f2b992e41be8060b30b168201e9c94` |
| `proof-obligations.planned.jsonl` | `.beads/vb-cn2v4/proof-obligations.planned.jsonl` | `704eb787ac5958a3fcd78dcb76cde89589811c8f28748c630cc06914b1f5c169` |
| `trusted-base-plan.md` | `.beads/vb-cn2v4/trusted-base-plan.md` | `822754ef18c832ee0c87c491a6a93eadcc89438037acfdaf96ee8c6d3e94f7b0` |
| `waiver-candidates.jsonl` | `.beads/vb-cn2v4/waiver-candidates.jsonl` | `c82c977d8f2ffe56e4d9c432490525c94a9df25d7d493fbcf3d99899270f31f1` |
| `proof-seeds.jsonl` | `.beads/vb-cn2v4/proof-seeds.jsonl` | `f63b17b8768b910ae571cae4ebdceb3cab34d6b7359d2d451f9b48626fd54f0e` |
| `contract.md` | `.beads/vb-cn2v4/contract.md` | `8c16f3453e406eec81b632bb3ec0e2a300aadcb67d87a80047256413e6c72670` |
| `traceability-matrix.jsonl` | `.beads/vb-cn2v4/traceability-matrix.jsonl` | `40510dc23d2ed82335aa545e3815c7269d228870fff1be3694f832bf8de6d53d` |

## Reviewer-Owned Outputs

| Artifact | Path | sha256 |
|---|---|---|
| `verifier-lane-review.jsonl` | `.beads/vb-cn2v4/verifier-lane-review.jsonl` | `40fd73b42520114bbe53214314fd3c70f5e17f5db9160dc709cb1d32d4b46ef2` |
| `proof-plan-findings.jsonl` | `.beads/vb-cn2v4/proof-plan-findings.jsonl` | `a620f1b5a08b842d8b10af324ca305fce1620c1439b6c068e577248cafd5a70c` |
| `proof-plan-review.md` | `.beads/vb-cn2v4/proof-plan-review.md` | (this file; hash will be in the appended ledger row) |

## Independent Provenance Check

- planner_invocation_id `femdation:vb-cn2v4:p4:planner:v1` differs from
  reviewer_invocation_id `femdation:vb-cn2v4:p4b:reviewer:v1`. Not a self-review.
- All 13 verifier-lane-decision rows are matched to a verifier-lane-review row in
  `verifier-lane-review.jsonl` (VLD-* 1:1 to VLR-*).
- The reviewed artifacts existed before the reviewer started
  (state-1 row 1; state-2 row 2 in `agent-invocation-ledger.jsonl`),
  so `reviewed_artifacts_existed_before_start: true` on the appended
  state-4 row.

## Lane Profile Coverage (default Rust profile)

| Verifier | Lane decisions | Required rows | Not applicable rows | Reviewer disposition |
|---|---|---|---|---|
| `verus` | 3 | 3 | 0 | accepted (PO-001 + PO-002 WEAK_EXTERN well-formed) |
| `kani` | 3 | 3 | 0 | accepted (Kani split-harness shape documented) |
| `proptest` | 3 | 3 | 0 | accepted (per-prefix + mutation-resistance covers all 5 call sites) |
| `flux-rs` | 1 | 0 | 1 | accepted (surface_absent; non_applicability_evidence_refs cite type-contracts + domain-model sha256) |
| `loom` | 1 | 0 | 1 | accepted (surface_absent; synchronous encoders, no Send/Sync boundary) |
| `miri` | 1 | 0 | 1 | accepted (surface_absent; #![forbid(unsafe_code)] at crate root) |
| `cargo-fuzz` | 1 | 0 | 1 | accepted (risk_out_of_scope; typed-input encoders; contract.md#C8 explicit "no fuzz required") |
| **Total** | **13** | **9** | **4** | **all 13 accepted** |

## Obligations (Six Total)

| ID | Verifier | Risk | Mechanism | Acceptance |
|---|---|---|---|---|
| `PO-001-VERUS-MIRROR` | verus | rejection | WEAK_EXTERN (mirror_path=verification/verus/extern_vb_storage_keys.rs, production_path=crates/vb_storage/src/keys.rs:76-209) | accepted |
| `PO-002-VERUS-DECODER-SYMMETRY` | verus | equality | WEAK_EXTERN (mirror_path=same, production_path=crates/vb_storage/src/keys.rs:162-209) | accepted |
| `PO-003-KANI-SPLIT-HARNESS` | kani | rejection | bounded_symbolic (SymbolicKeyInputs; kani::Arbitrary; Err(_)=>assert!(false) replaced with split) | accepted |
| `PO-004-KANI-ORDER-OF-CHECKS` | kani | rejection | bounded_symbolic (require_non_zero_run fires before to_u8_checked on index_status_key) | accepted |
| `PO-005-PROPTEST-PER-PREFIX` | proptest | rejection | property (PROPTEST_CASES=10000; matches!(result, Err(JournalError::InvalidRunId {run})) per prefix) | accepted |
| `PO-006-PROPTEST-MUTATION` | proptest | rejection | property (mutation_resistance_require_non_zero_run; flag-controlled guard simulation) | accepted |

## Production-Binding Plan Validation (Mandatory)

Every `proof-obligation/v1` row with `verifier: verus` MUST carry a
`production_binding` field. Verified below for PO-001 and PO-002:

### PO-001-VERUS-MIRROR (WEAK_EXTERN)

- `mechanism`: `WEAK_EXTERN` — in {STRONG, WEAK_MIRROR, WEAK_EXTERN} ✓
- `production_path`: `crates/vb_storage/src/keys.rs` — exists on disk ✓
- `production_lines`: `76-209` — non-empty, covers all 6 public encoders (76-78, 81-83, 86-91, 101-122, 125-137, 140-155) ✓
- `assume_specification_targets`: non-empty array
  `["crate::keys::run_event_key", "crate::keys::journal_key", "crate::keys::encode_key"]` ✓
- `mirror_path`: `verification/verus/extern_vb_storage_keys.rs` — exists on disk ✓
  (The proof-plan-reviewer rubric names this field `extern_path`;
  field-name divergence logged as F-CN2V4-001 / E_SCHEMA_REFERENCE_FORK
  with disposition `owner_approved_no_action`. Semantically equivalent.)

### PO-002-VERUS-DECODER-SYMMETRY (WEAK_EXTERN)

- `mechanism`: `WEAK_EXTERN` — in {STRONG, WEAK_MIRROR, WEAK_EXTERN} ✓
- `production_path`: `crates/vb_storage/src/keys.rs` — exists on disk ✓
- `production_lines`: `162-209` — non-empty, covers encode_key_into (162-198) and encode_key (205-209) ✓
- `assume_specification_targets`: non-empty array
  `["crate::keys::encode_key", "crate::keys::encode_key_into"]` ✓
- `mirror_path`: `verification/verus/extern_vb_storage_keys.rs` — exists on disk ✓

### Rejection criteria audit (per skill rubric)

1. ✓ No missing `production_binding` field on any Verus row.
2. ✓ All `production_binding.mechanism` values are in {STRONG, WEAK_MIRROR, WEAK_EXTERN}.
3. ✓ All `production_path` values exist on disk.
4. ✓ All `production_lines` are non-empty.
5. ✓ All `assume_specification_targets` are non-empty arrays.
6. ✓ All `mirror_path`/`extern_path` values exist on disk.
7. ✓ No `EXPLICITLY_ALLOWED` / `ALLOWED_EXCEPTIONS` / `OFFLOAD` mechanisms used.

## 6-Obligation Verification (Task Brief)

Confirmed via `proof-obligations.planned.jsonl` enumeration:

1. **PO-001-VERUS-MIRROR** — `verus` — `verifier/verus/extern_vb_storage_keys.rs`
   extended with `SpecKeyEncodeError::InvalidRunId { run: u64 }` and
   `assume_specification` contracts on the run-bearing mirror fns. ✓
2. **PO-002-VERUS-DECODER-SYMMETRY** — `verus` — mirror fns return
   `Err(SpecKeyEncodeError::InvalidRunId { run })` iff `run == 0`; decoder
   mirror unchanged. ✓
3. **PO-003-KANI-SPLIT-HARNESS** — `kani` — `assert_key_contracts` is
   reorganised so the `run_value == 0` rejection arm distinguishes from the
   `run_value != 0` happy arm; `kani::cover` reachability for both arms. ✓
4. **PO-004-KANI-ORDER-OF-CHECKS** — `kani` — `require_non_zero_run` fires
   before `state.to_u8_checked` in `index_status_key`; `Other(0..2)`
   collision path with `RunId(0)` is unreachable. ✓
5. **PO-005-PROPTEST-PER-PREFIX** — `proptest` —
   `encoder_rejects_zero_run_id_for_every_prefix` covers all 6 public
   encoder entry points with `RunId(0)`. ✓
6. **PO-006-PROPTEST-MUTATION** — `proptest` — `mutation_resistance_require_non_zero_run`
   verifies the shared `require_non_zero_run` guard cannot be removed
   without the proptest detecting it. ✓

## Kani Split-Harness Verification

- File: `crates/vb_storage/src/kani_typed_partitioned_ids.rs`
- Existing match-arms at lines 61, 69, 77, 85 use `Err(_) => assert!(false)` — this is the
  C6 contract-defect that the plan rewrites with split `if/else` on
  `run_value == 0`.
- `SymbolicKeyInputs` at lines 15-24 uses `kani::Arbitrary` (GOD RULE 1
  compliant — no hardcoded structural inputs).
- `run_raw(inputs)` at line 35 maps `run_hi: u16 | run_lo: u16` to
  `run_value: u64`; the full domain `[0, 2^32-1]` is reachable (no overflow
  concerns). Plan says `kani::cover` reachability proves both arms are
  reachable.

## Shared `require_non_zero_run` Helper Verification

- PO-005 covers the 5 call sites: `run_only_key`, `sequenced_run_key`,
  `index_status_key`, `index_workflow_key`, `index_action_key`.
- PO-006 explicitly tests the mutation-resistance: removing the guard
  causes the proptest to fail (flag-controlled guard simulation).
- The shared helper is the C2 contract clause; `proof-seed-002`
  documents it; `trusted-base-plan.md` lists it as part of TB-002.

## 18 Test Flips Verification

- Per `contract.md#C5`: `crates/vb_storage/src/keys/tests.rs` (11 tests,
  specific line ranges in C5.1-C5.11) +
  `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` (3 tests) +
  `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` (4 tests) = 18.
- The flips are the test-writer's scope (proof-planner Non-Goals states
  explicitly: "No tests authored in this plan").
- The proof-plan surfaces the flips via: (a) contract.md#C5 enumeration,
  (b) seed-007 (`REQ-test-flip-suite`), (c) VLD-PROPTEST-CN2V4-003 (proptest
  companion), (d) PO-005 (per-prefix property test complements them).
- Verification: 11 + 3 + 4 = **18** ✓.

## Risk Tags and Profile

The default Rust profile is honored (no missing lanes, no unjustified
`not_applicable` decisions). The `loom`/`miri`/`cargo-fuzz` rows are
non-applicable for surface-absent or risk-out-of-scope reasons with
concrete evidence refs (sha256-prefixed citations to type-contracts.md,
domain-model.md, boundary-map.md, workflow-model.md, contract.md).

## Anti-Laundering Discipline

Verified per `proof-strategy.md#Anti-Laundering-Discipline`:

- No `assume(`, `axiom`, `admit`, `sorry`, `#[verifier::external_body]`
  in the planner's `expected_evidence` or `command` of any Verus
  obligation. The `#[verifier::external]` markers on the mirror fn
  bodies are the project-established pattern.
- No `kani::cover!` as sole property evidence. The `expected_evidence`
  cites `kani::assert` (function contract postcondition or direct
  assertion).
- No hardcoded structural Kani inputs. `SymbolicKeyInputs` uses
  `kani::Arbitrary` (GOD RULE 1).
- No `is_ok()`-only proptests. Every proptest asserts
  `matches!(result, Err(JournalError::InvalidRunId { .. }))` exactly.

## Trusted-Base Plan

Two trust markers, both non-behavior (structural/harness-pattern
necessity):

- **TB-001-verus-mirror-extern-pattern** — WEAK_EXTERN production
  binding; mirror body opaque to Verus, `assume_specification` contracts
  are the verified surface. Required for PO-001 + PO-002.
- **TB-002-kani-harness-split-shape** — Split-harness shape (in-place
  if/else) is required because the current `Err(_)=>assert!(false)`
  arms treat legitimate rejection as a counterexample. Required for
  PO-003 + PO-004.

The plan's `trusted-base-plan.md` documents each marker with
production-binding mechanism, boundary proof, and compensating evidence
(sibling obligations + external system references + file path/sha256).

## Findings (all non-blocker)

| Code | Severity | Artifact | Disposition |
|---|---|---|---|
| `E_SCHEMA_REFERENCE_FORK` | observation | PO-001, PO-002 `mirror_path` vs skill rubric `extern_path` | `owner_approved_no_action` |
| `E_SOURCE_REF_SHAPE` | minor | `verification/verus/extern_vb_storage_keys.rs:47` references nonexistent `production_inner/vb_storage_keys_production.rs` | `owner_approved_debt` (DEBT-CN2V4-001) |
| `E_COMMAND_EVIDENCE_MISSING` | minor | `verification/verus/vb_storage_keys_spec.rs` does not exist; plan contract references it for `assume_specification` | `owner_approved_debt` (DEBT-CN2V4-002) |

None of the three findings is severity `blocker`. The plan satisfies
all rubric criteria for reviewer approval. The two `owner_approved_debt`
findings describe pre-existing infrastructure gaps that the
proof-writer (State 5) must resolve as part of the executor's scope
before running the Verus command; this is documented in
`proof-plan-findings.jsonl` with `debt_ref` values for downstream
tracking.

## Gate Script Compatibility

`scripts/check-verus-production-binding.sh`:

- Line 67: `*/extern_*.rs` files SKIPPED (companion modules, not spec files).
- Line 70: `*/production_inner/*` files SKIPPED.
- Lines 75-120: spec files require `proof fn` + `#[path=...]` +
  `assume_specification[...]`. PO-001 + PO-002 reference the extern
  companion file; once `vb_storage_keys_spec.rs` is created at State 5
  with the right shape, the spec will classify as STRONG or WEAK_EXTERN
  compliant.

`scripts/check-production-inner-drift.sh`: the Verus obligations cite
this as `drift_detection`; the gate runs at State 12 (formal-verifier).

## Hash Chain

The appended state-4 reviewer ledger row is the chained entry that
follows the existing state-2 row in `agent-invocation-ledger.jsonl`.
SHA-256 hash of this row follows the canonical algorithm (sort keys,
compact JSON, hash excluding `entry_hash`).

## Disposition

The plan satisfies the proof-plan-reviewer rubric:

- 6 obligations, all with proper schema fields.
- Default Rust profile lanes (verus, kani, proptest) all `required`
  with concrete `required_obligation_ids` references and matching
  `proof-obligation/v1` rows.
- Conditional lanes (flux-rs, loom, miri, cargo-fuzz) all
  `not_applicable` with concrete evidence refs (sha256-prefixed) and
  compensating evidence pointing to sibling obligations.
- 13 verifier-lane-review rows align 1:1 with the 13 planner
  lane-decision rows; no orphans, no duplicates, no self-reviews.
- Kani split-harness shape is well-defined (in-place if/else with
  `kani::cover` reachability).
- Verus WEAK_EXTERN production-binding mechanism is well-formed; both
  obligations carry `production_binding` with valid mechanism,
  production_path, production_lines, assume_specification_targets, and
  mirror_path.
- Shared `require_non_zero_run` helper is exercised by PO-005 (per-prefix
  coverage of all 5 call sites) and PO-006 (mutation-resistance).
- 18 test flips are scoped to test-writer (proof-planner Non-Goals)
  but surface in the plan via contract.md#C5, seed-007, and
  VLD-PROPTEST-CN2V4-003.
- Findings are all non-blocker (`owner_approved_no_action` or
  `owner_approved_debt`).
- No `behavior_affecting: true` waivers (E_BEHAVIOR_WAIVER forbidden).
- No `assume`/`axiom`/`admit`/`sorry`/`external_body` introduced.

Per the rubric, approval is granted when no finding carries
`disposition: blocker` and the plan is precise enough for proof-writer
and proof-to-implementation.

STATUS: APPROVED
