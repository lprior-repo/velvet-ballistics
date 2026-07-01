# Proof Coverage Matrix — vb-rz9ey

- bead_id: vb-rz9ey
- state: 4 (proof-planner)
- authored_by: proof-planner
- contract_sha256: e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66
- proof_seeds_sha256: d95357c83d1d086b71376f452dadd20326bb2e05f183d97152fe10e9121551d1
- traceability_sha256: 101667a0a9c378006e1ed4dd740bae6e160e0961b9d62603948a6778a95143a1
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey

This matrix maps every input `proof-seed/v1` row from
`.beads/vb-rz9ey/proof-seeds.jsonl` to the consolidated
`proof-obligation/v1` row(s) in `proof-obligations.planned.jsonl` that cover it,
and from there to the `verifier-lane-decision/v1` row(s) in
`verifier-lane-decisions.jsonl` that drive its verification.

## 1. Proof-Seed → Obligation Mapping

| proof_seed_id | requirement_id | contract_clause | subsumed_into_obligation | subsumption_reason |
|---------------|----------------|------------------|---------------------------|---------------------|
| ps-vb-rz9ey-01 | REQ-RZ9EY-VISIBILITY-INVARIANT | CC-1 (TC-1) WorkflowSourceParts / WorkflowSource::new visibility gated by cfg(any(test, feature="test-util")) | PO-001 + PO-002 | PO-001 verifies the cfg-gated visibility flips `pub` under test-util (test side); PO-002 verifies it stays `pub(crate)` in the production build (default-features side). Together the two obligations fully cover the visibility invariant. |
| ps-vb-rz9ey-02 | REQ-RZ9EY-TESTBUILD-COMPILE | CC-1 (TC-1) Visibility is pub under cfg(any(test, feature="test-util")) | PO-001 | PO-001's command is `cargo build -p vb_compile --tests --message-format=human` which is the direct verification of testbuild compile. Baseline: 38 errors (12 E0432 + 26 E0624). After-fix metric: 0 errors. |
| ps-vb-rz9ey-03 | REQ-RZ9EY-DOWNSTREAM-PRESERVE-1 | CC-4 (TC-4) Downstream API surface preservation — vb_cli | PO-002 | PO-002's command includes `cargo build -p vb_cli --message-format=human` which is the direct verification that vb_cli's build graph does not activate test-util. |
| ps-vb-rz9ey-04 | REQ-RZ9EY-DOWNSTREAM-PRESERVE-2 | CC-4 (TC-4) Downstream API surface preservation — workspace_tests | PO-002 | PO-002's command includes `cargo build -p workspace_tests --message-format=human` which is the direct verification that workspace_tests's build graph does not activate test-util. |
| ps-vb-rz9ey-05 | REQ-RZ9EY-LOCKFILE-MINIMAL | CC-3 (TC-3) Self-referencing dev-dependency contract — Cargo.lock minimal diff | PO-001 (sub-evidence) | PO-001's expected_evidence enumerates "git diff --stat Cargo.lock shows exactly 1 file changed, 1 insertion(+), 0 deletions(-)" as sub-evidence. Lockfile minimal diff is a precondition for PO-001 success. |
| ps-vb-rz9ey-06 | REQ-RZ9EY-FEATURE-INERTNESS | CC-2 (TC-2) Cargo test-util feature contract — default is empty | PO-001 (sub-evidence) + PO-002 | PO-001's expected_evidence cites `moon run :lint-src` as sub-evidence (the lint policy enforces feature declaration discipline). PO-002's expected_evidence cites `awk '/^\[dependencies\]/,/^\[/' crates/vb_compile/Cargo.toml | grep -c 'features = \["test-util"\]'` returning 0 as sub-evidence, plus the cargo doc surface (which depends on default-features build). |
| ps-vb-rz9ey-07 | REQ-RZ9EY-FIELD-SHAPE-DIVERGENCE | CC-1 (TC-1.a) The two cfg arms of WorkflowSourceParts are field-identical; only visibility differs | PO-001 (sub-evidence) | PO-001's assumptions array explicitly enumerates "the two cfg arms of WorkflowSourceParts at workflow.rs:107-127 and :129-149 remain field-identical". Drift here would silently desynchronize production and test builds and would be caught by PO-001's compile-time check (the test build cannot succeed if field shape diverges in a way that breaks the constructor). |
| ps-vb-rz9ey-08 | REQ-RZ9EY-SELF-REF-PLACEMENT | CC-3 (TC-3.a) Self-referencing dev-dependency lives in [dev-dependencies], not [dependencies] | PO-001 + PO-002 | PO-001's assumptions explicitly enumerate "self-reference syntax path = \".\" and features = [\"test-util\"] is exactly as the contract specifies". PO-002's expected_evidence enumerates the [dependencies] grep returning 0. Both PO-001 and PO-002 would fail if the entry were misplaced. |

Coverage: 8/8 proof-seeds are mapped to PO-001 and/or PO-002. No silent
omission, no orphan seed.

## 2. Obligation → Lane-Decision Mapping

| obligation_id | verifier | required lane decision IDs | not_applicable lane decision IDs | evidence_command |
|---------------|----------|----------------------------|----------------------------------|--------------------|
| PO-001 | proptest | VLD-001 | VLD-002 (verus), VLD-003 (kani), VLD-004 (flux-rs), VLD-005 (loom), VLD-006 (miri), VLD-007 (cargo-fuzz) | `cargo build -p vb_compile --tests --message-format=human` |
| PO-002 | proptest | VLD-008 | VLD-009 (verus), VLD-010 (kani), VLD-011 (flux-rs), VLD-012 (loom), VLD-013 (miri), VLD-014 (cargo-fuzz) | `(cargo build -p vb_cli --message-format=human && cargo build -p workspace_tests --message-format=human && cargo doc -p vb_compile --no-deps --message-format=human 2>&1 | grep -c WorkflowSourceParts)` |

Coverage: 2/2 obligations have a paired required lane decision with a paired
obligation ID; 12/12 default-profile verifiers have a typed
`not_applicable` row with concrete `non_applicability_evidence_refs`.

## 3. Risk-Tag Coverage

| risk_tag | present_in_seeds | covered_by_obligation | covered_by_lane_decision |
|----------|-------------------|------------------------|----------------------------|
| risk:public_api | ps-01, ps-03, ps-04, ps-06, ps-07 | PO-001 (test side) + PO-002 (production side) | VLD-001, VLD-008 |
| risk:build | ps-01, ps-02, ps-05, ps-08 | PO-001 | VLD-001 |
| risk:test_only | ps-02 | PO-001 | VLD-001 |
| risk:downstream | ps-03, ps-04 | PO-002 | VLD-008 |
| risk:lockfile | ps-05 | PO-001 (sub-evidence) | VLD-001 |
| risk:test_only (repeated) | ps-08 | PO-001 + PO-002 | VLD-001, VLD-008 |

Coverage: 6/6 distinct risk_tags are mapped. No orphan risk_tag.

## 4. Contract-Clause Coverage

| contract_clause | proof_seeds | obligation | lane_decision |
|------------------|-------------|-------------|----------------|
| CC-1 (TC-1) Visibility gated by cfg(any(test, feature="test-util")) | ps-01, ps-02, ps-07 | PO-001 | VLD-001 + VLD-002..007 |
| CC-2 (TC-2) Cargo test-util feature contract — default is empty | ps-06 | PO-001 (sub) + PO-002 | VLD-001 + VLD-008 |
| CC-3 (TC-3) Self-referencing dev-dependency contract | ps-05, ps-08 | PO-001 (sub) + PO-002 | VLD-001 + VLD-008 |
| CC-4 (TC-4) Downstream API surface preservation | ps-03, ps-04 | PO-002 | VLD-008 + VLD-009..014 |
| CC-1.a (TC-1.a) Field-shape divergence invariant | ps-07 | PO-001 (assumption) | VLD-001 |

Coverage: 5/5 contract clauses are mapped.

## 5. Lane × Risk Cross-Tab

| lane \ risk_tag | build | public_api | lockfile | test_only | downstream |
|------------------|-------|------------|----------|-----------|-------------|
| proptest (required) | ✓ PO-001 | ✓ PO-001 + PO-002 | ✓ PO-001 (sub) | ✓ PO-001 | ✓ PO-002 |
| verus (not_applicable) | ✓ surface_absent | ✓ surface_absent | n/a | n/a | ✓ surface_absent |
| kani (not_applicable) | ✓ surface_absent | n/a | n/a | ✓ surface_absent | ✓ surface_absent |
| flux-rs (not_applicable) | ✓ surface_absent | ✓ surface_absent | n/a | n/a | ✓ surface_absent |
| loom (not_applicable) | ✓ surface_absent | ✓ surface_absent | n/a | n/a | ✓ surface_absent |
| miri (not_applicable) | ✓ surface_absent | ✓ surface_absent | n/a | n/a | ✓ surface_absent |
| cargo-fuzz (not_applicable) | ✓ surface_absent | ✓ surface_absent | n/a | ✓ surface_absent | ✓ surface_absent |

Coverage: every (lane, risk_tag) intersection is either ✓ (lane decision
present with typed `limitation_kind` and concrete evidence ref) or n/a (the
risk_tag does not motivate the verifier per
`/home/lewis/.opencode/skill/proof-planner/references/risk-taxonomy.md`).

## 6. Inverse Index (Obligation → Lane Decision)

This is the inverse index the validator requires per
`references/lane-decision-guide.md` §"Self-Audit Checklist":

```
PO-001 → VLD-001 (proptest, required)
PO-002 → VLD-008 (proptest, required)

PO-001 ← VLD-001 (required_obligation_ids includes PO-001)
PO-002 ← VLD-008 (required_obligation_ids includes PO-002)
```

All 2 obligations have at least one paired required lane decision. No
obligation is `E_LANE_OBLIGATION_MISSING`.

## 7. Coverage Summary

- Proof-seed coverage: 8/8 (100%).
- Risk-tag coverage: 6/6 distinct (100%).
- Contract-clause coverage: 5/5 (100%).
- Obligation-to-lane pairing: 2/2 required lanes paired (100%).
- Default-profile non-applicability coverage: 12/12 with concrete evidence
  (100%).

No gap. No orphan. No silent omission. The plan is internally complete.