---
reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: femdation-cheap25-batch-vb-rz9ey-state4-proof-plan-reviewer
planner_invocation_id: femdation-cheap25-batch-vb-rz9ey-state4-proof-planner
host_session_id: femdation-cheap25-batch
bead_id: vb-rz9ey
review_state: 4
reviewed_at: 2026-07-01T17:00:00Z
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
---

# Proof Plan Review — vb-rz9ey

**Bead**: vb-rz9ey — Fix vb_compile test compilation: WorkflowSourceParts private (Cargo self-reference, P0)
**Scope class**: `cargo-manifest-metadata-only`
**Behavior-affecting**: `false`

---

## Review Metadata

**Reviewer**: proof-plan-reviewer (direct child of femdation; no sub-agents invoked)
**Planner invocation**: `femdation-cheap25-batch-vb-rz9ey-state4-proof-planner` (independent ID, derived from host_session_id + bead_id + state + role)
**Reviewer invocation**: `femdation-cheap25-batch-vb-rz9ey-state4-proof-plan-reviewer` (independent ID; same host_session_id, distinct role tag)
**Hash chain**: this reviewer's `agent-invocation-ledger.jsonl` entry appends at `ledger_sequence: 3` with `previous_entry_hash = b8e12c0e12fc2ff097ec08175436468d238e73ef1f77efd06a0aa4dd8bd0a086` (existing state2 row).

---

## Reviewed Artifacts (with hashes)

| Artifact | sha256 | Status |
|----------|--------|--------|
| `contract.md` | `e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66` | reviewed |
| `codebase-map.md` | `7336795bdf60f345ae7d2af2641b16388e36fc79d27e653cf00db31affd66697` | reviewed |
| `domain-model.md` | (present) | reviewed |
| `type-contracts.md` | (present) | reviewed |
| `workflow-model.md` | (present) | reviewed |
| `error-taxonomy.md` | (present) | reviewed |
| `boundary-map.md` | (present) | reviewed |
| `hazard-analysis.md` | (present) | reviewed |
| `proof-seeds.jsonl` | `d95357c83d1d086b71376f452dadd20326bb2e05f183d97152fe10e9121551d1` | reviewed (8 rows) |
| `traceability-matrix.jsonl` | `101667a0a9c378006e1ed4dd740bae6e160e0961b9d62603948a6778a95143a1` | reviewed (8 rows) |
| `delivery-scope.jsonl` | `f35caf1e55e0c0d0c6f4a21a8d88251a7b78faeb453f8c5863dbc8cb2a3badf9` | reviewed |
| `proof-strategy.md` | `f9765849970a049eefd2fb04a4ef6cda1201b67aa1f16c0c5fcf49099d7f27f7` | reviewed |
| `verifier-lane-decisions.jsonl` | `9a577a51995a11468a46b0a9b7d97a487368d4ebb1ff8f5eec9a37ce225fde50` | reviewed (14 rows: 2 required + 12 not_applicable) |
| `proof-obligations.planned.jsonl` | `a8dc5fae7a553f693c97085e196c51c5da2f2675e354d4b16027cb214e092983` | reviewed (2 rows: PO-001, PO-002) |
| `proof-coverage-matrix.md` | (present) | reviewed |
| `verifier-lane-matrix.md` | (present) | reviewed |
| `trusted-base-plan.md` | `15ad62c6a6843af437a3aed89258e5665a8764d4324ca800313a8ad22367f1d2` | reviewed (zero entries) |
| `waiver-candidates.jsonl` | `a8d2771519552ed6757d3dea157ed75c351ddc0a2e86b873a429f85b960ffb7b` | reviewed (1 ledger-anchor row, behavior_affecting=false) |
| `proof-to-implementation-input.md` | (present) | reviewed |
| `agent-invocation-ledger.jsonl` | (existing 2 rows: state1 go-skill, state2 explore) | reviewed |

**Hash verification**: all 4 hashes cited in `proof-strategy.md` (contract, codebase_map, proof_seeds, traceability) and `delivery-scope.jsonl` hash match the SHA-256 of the corresponding on-disk artifacts.

---

## Review Summary

### 1. Scope Classification — PASS

`scope_class: cargo-manifest-metadata-only` and `behavior_affecting: false` are correctly asserted in:
- `contract.md` line 7–8 (`scope_class: cargo-manifest-metadata-only`, `behavior_affecting: false`)
- `proof-strategy.md` line 7–8
- `trusted-base-plan.md` (empty trusted base, consistent with metadata-only patch)
- `hazard-analysis.md` line 4–6 (no runtime contract changes)
- `waiver-candidates.jsonl` row `WVR-000` (`behavior_affecting: false`)

The forbidden-actions constraint (no behavior change, no public API widening) is preserved: the only mutation is a single `[dev-dependencies]` line `vb_compile = { path = ".", features = ["test-util"] }`, which is the canonical Cargo self-reference pattern (`specifying-dependencies.html#self-references`). Verified on-disk at `crates/vb_compile/Cargo.toml:18-19` (current `[dev-dependencies]` section, where the fix lands) and `:21-23` (`[features]` block: `default = []`, `test-util = []`, both empty features).

### 2. Lane Decision Coverage — PASS

14 lane decisions across 7 verifiers × 2 obligations (PS-001, PS-002):

| verifier | PO-001 (VLD) | PO-002 (VLD) | profile |
|---------|--------------|--------------|---------|
| proptest | VLD-001 `required` | VLD-008 `required` | default Rust profile |
| verus | VLD-002 `not_applicable` (surface_absent) | VLD-009 `not_applicable` (surface_absent) | default Rust profile |
| kani | VLD-003 `not_applicable` (surface_absent) | VLD-010 `not_applicable` (surface_absent) | default Rust profile |
| flux-rs | VLD-004 `not_applicable` (surface_absent) | VLD-011 `not_applicable` (surface_absent) | default Rust profile |
| loom | VLD-005 `not_applicable` (surface_absent) | VLD-012 `not_applicable` (surface_absent) | conditional — no concurrency surface |
| miri | VLD-006 `not_applicable` (surface_absent) | VLD-013 `not_applicable` (surface_absent) | conditional — no unsafe surface |
| cargo-fuzz | VLD-007 `not_applicable` (surface_absent) | VLD-014 `not_applicable` (surface_absent) | conditional — no parser/codec surface |

Every (proof_seed, verifier) tuple has a lane decision. No silent omissions.

### 3. Non-Applicability Evidence — PASS

All 12 `not_applicable` rows cite concrete `non_applicability_evidence_refs` with SHA-256 hashes (verified via `jq -c '.non_applicability_evidence_refs | length'`):

| VLD | verifier | evidence refs | limitation_kind |
|-----|----------|---------------|-----------------|
| VLD-002 | verus | 2 (contract + codebase-map) | surface_absent |
| VLD-003 | kani | 2 (contract + codebase-map) | surface_absent |
| VLD-004 | flux-rs | 2 (codebase-map + delivery-scope) | surface_absent |
| VLD-005 | loom | 1 (codebase-map) | surface_absent |
| VLD-006 | miri | 1 (codebase-map) | surface_absent |
| VLD-007 | cargo-fuzz | 1 (codebase-map) | surface_absent |
| VLD-009 | verus | 2 (contract + codebase-map) | surface_absent |
| VLD-010 | kani | 1 (codebase-map) | surface_absent |
| VLD-011 | flux-rs | 2 (codebase-map + delivery-scope) | surface_absent |
| VLD-012 | loom | 1 (codebase-map) | surface_absent |
| VLD-013 | miri | 1 (codebase-map) | surface_absent |
| VLD-014 | cargo-fuzz | 1 (codebase-map) | surface_absent |

No weak vocabulary (no "not needed", "too hard", "covered by other lane", "low risk", "we'll add this later"). Every `decision_reason` names the specific surface absence (e.g., `verification/verus/` is absent for `vb_compile`; no concurrency boundary; no FFI; no parser/codec hostile-input boundary). All `limitation_kind` values are typed (`surface_absent`).

**Spot-check**: VLD-003 (kani, PO-001) correctly identifies the pre-existing latent defect (OI-1: `crate::ast` does not re-export `WorkflowSource`) and explicitly defers it as out-of-scope per contract §10. The Kani harnesses are `cfg(kani)`-gated and do not participate in `cargo build --tests`. This is a faithful citation.

### 4. Production-Binding Plan Validation — PASS (N/A)

Per `proof-plan-reviewer/SKILL.md` "Production Binding Plan Validation (MANDATORY — NO BACKDOORS)": every Verus `proof-obligation/v1` row must carry a `production_binding` field with mechanism `STRONG | WEAK_MIRROR | WEAK_EXTERN`.

This bead emits **zero Verus obligations**. Verified:
- `proof-obligations.planned.jsonl` has exactly 2 rows; both `verifier: proptest`.
- `verifier-lane-decisions.jsonl` has 2 rows with `verifier: verus` (VLD-002 and VLD-009), both `applicability: not_applicable`.
- No `EXPLICITLY_ALLOWED`, `ALLOWED_EXCEPTIONS`, or `OFFLOAD` mechanisms appear in any planner artifact (verified via grep across all 5 reviewed JSONL/MD files).
- `proof-strategy.md` §5 correctly documents that `production_binding` is N/A and the Verus-binding gate (`scripts/check-verus-production-binding.sh`) does not apply.
- `trusted-base-plan.md` §4 mirrors this analysis for the analogous cargo-build binding plan.

**Conclusion**: The no-Verus-emitted precondition is satisfied; the production-binding requirement is structurally inapplicable.

### 5. Proof Obligation Schema — PASS

Both `proof-obligation/v1` rows (PO-001, PO-002) carry the full required schema per `proof-schemas.md`:

| field | PO-001 | PO-002 |
|-------|--------|--------|
| `schema_version` | `proof-obligation/v1` | `proof-obligation/v1` |
| `id` | `PO-001` | `PO-002` |
| `requirement_id` | `REQ-RZ9EY-TESTBUILD-COMPILE` | `REQ-RZ9EY-DOWNSTREAM-PRESERVE` |
| `contract_clause` | CC-1 (TC-1) | CC-4 (TC-4) |
| `domain_claim` | (concrete: cargo build compiles with 0 errors after dev-dep) | (concrete: cargo build -p vb_cli/workspace_tests exits 0; cargo doc shows 0 hits) |
| `risk` | `panic_freedom` (compile-time reduction) | `panic_freedom` (compile-time reduction) |
| `risk_tags` | 6 tags (rust_local, build_manifest, visibility, public_api, lockfile, test_only) | 4 tags (rust_local, public_api, downstream, feature_isolation) |
| `verifier` | `proptest` | `proptest` |
| `artifact` | `crates/vb_compile/Cargo.toml` | `crates/vb_compile/Cargo.toml` |
| `target` | `crates::vb_compile::yaml_ast::types::WorkflowSourceParts` | same |
| `command` | exact `cargo build -p vb_compile --tests --message-format=human` | exact `(cargo build -p vb_cli && cargo build -p workspace_tests && cargo doc -p vb_compile --no-deps | grep -c WorkflowSourceParts)` |
| `workdir` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey` | same |
| `expected_evidence` | concrete (exit 0, 0 E0432, 0 E0624, 9 affected test files compile, 1-line Cargo.lock diff, lint-src passes) | concrete (both cargo build exit 0, grep -c returns 0, [dependencies] grep returns 0) |
| `assumptions` | array (test-util declaration unchanged, cfg arms field-identical, self-reference syntax matches contract) | array (downstream Cargo manifests do not activate features, default=[] preserved, --no-deps uses default-features) |
| `model_bounds` | present | present |
| `tool_metadata` | present | present |
| `trusted_base_refs` | `[]` (empty) | `[]` (empty) |
| `required` | `true` | `true` |
| `behavior_affecting` | `false` | `false` |
| `mode` | `verify-proof` | `verify-proof` |
| `owner_state` | `4` | `4` |
| `rerun_from` | present | present |
| `status` | `planned` | `planned` |

**No legacy alias fields** (`layer`, `checker`, alias-only `claim`) detected. `target` is canonical. Commands are exact and workdir-aligned.

### 6. Required Lane → Obligation Pairing — PASS

| VLD | required_obligation_ids | paired obligation in planned JSONL |
|-----|-------------------------|-------------------------------------|
| VLD-001 | `[PO-001]` | ✓ present (PO-001 matches VLD-001's `verifier: proptest` and `proof_seed_id: PS-001`) |
| VLD-008 | `[PO-002]` | ✓ present (PO-002 matches VLD-008's `verifier: proptest` and `proof_seed_id: PS-002`) |

No `E_LANE_OBLIGATION_MISSING` or `E_LANE_OBLIGATION_MISMATCH`.

### 7. Proof-Seed Coverage — PASS (8/8)

`proof-coverage-matrix.md` §1 maps every input `proof-seed/v1` row to PO-001 and/or PO-002:

| proof_seed_id | subsumed_into_obligation |
|---------------|---------------------------|
| ps-vb-rz9ey-01 (VISIBILITY-INVARIANT) | PO-001 + PO-002 |
| ps-vb-rz9ey-02 (TESTBUILD-COMPILE) | PO-001 |
| ps-vb-rz9ey-03 (DOWNSTREAM-PRESERVE-1, vb_cli) | PO-002 |
| ps-vb-rz9ey-04 (DOWNSTREAM-PRESERVE-2, workspace_tests) | PO-002 |
| ps-vb-rz9ey-05 (LOCKFILE-MINIMAL) | PO-001 (sub-evidence) |
| ps-vb-rz9ey-06 (FEATURE-INERTNESS) | PO-001 (sub) + PO-002 |
| ps-vb-rz9ey-07 (FIELD-SHAPE-DIVERGENCE) | PO-001 (assumption) |
| ps-vb-rz9ey-08 (SELF-REF-PLACEMENT) | PO-001 + PO-002 |

8/8 covered. No orphan seed.

### 8. Risk-Tag and Contract-Clause Coverage — PASS

- 6 distinct risk_tags (`risk:public_api`, `risk:build`, `risk:test_only`, `risk:downstream`, `risk:lockfile`, repeated `risk:test_only`) all mapped to obligations and lane decisions (proof-coverage-matrix §3 + §5).
- 5 contract clauses (CC-1, CC-1.a, CC-2, CC-3, CC-3.a, CC-4) all mapped to obligations and lane decisions (proof-coverage-matrix §4).

### 9. TLA+ Compliance — PASS (N/A)

TLA+ is globally removed per `proof-pipeline-contract.md` and `verification-lane-policy.md` (since 2026-06-22). No TLA+ obligations, no TLA+ lane decisions, no TLA+ waivers in this bead. Temporal workflows are out of scope for `vb-rz9ey` (no temporal workflow surface, no state machine).

### 10. Waiver Candidates — PASS

`waiver-candidates.jsonl` has exactly 1 row (`WVR-000`, `PS-000`) acting as a self-audit ledger anchor with `behavior_affecting: false` and `reason` explicitly stating "no formal-verification verifier is required, so no obligation exists that would need a waiver". This is acceptable per `proof-schemas.md` `waiver-candidate/v1`: the row carries all required fields and a non-behavior-affecting flag, and it serves as a deliberate audit-trail record.

**No `E_BEHAVIOR_WAIVER`**: zero behavior-affecting waivers.

### 11. Trusted Base Plan — PASS

`trusted-base-plan.md` declares zero entries. No `assume` / `axiom` / `admit` / `external_body` / `#[trusted]` / `#[ignore]` / `opaque` / `extern_spec` markers are introduced. The `assumptions` arrays in PO-001 and PO-002 are preconditions of the Cargo manifest edit (verified by static source review at State 4b/8), not trust markers for a proof system. The 8-row self-audit checklist is fully satisfied.

### 12. Non-Vacuity — PASS (N/A)

Non-vacuity check is N/A because:
- Zero Verus obligations → no Verus standalone model risk
- Zero Kani obligations → no `cover!`-only risk
- Zero Flux obligations → no refinement trust abuse
- The 2 obligations are cargo-build/cargo-doc invocations against existing production Rust; rustc statically enforces the visibility invariant (`lib.rs:241`) and cargo enforces per-build-graph feature unification

### 13. Bridge Planning — PASS

`proof-to-implementation-input.md` provides:
- PO-001 → `Cargo.toml [dev-dependencies]` + `lib.rs:241` + 9 affected test files (with explicit list)
- PO-002 → `vb_cli/Cargo.toml:8` + `workspace_tests/Cargo.toml:39` + `lib.rs:241` doc surface
- Forbidden mutations list (8 paths per contract §3.3)
- Required mutation (single-line insertion in `[dev-dependencies]`)
- Lockfile regeneration strategy
- Sub-evidence commands for `holzman-rust` (State 6) and `black-hat-reviewer` (State 8)

Bridge is implementation-bound and concrete. No `E_PROOF_TO_RUST_MISSING`.

### 14. Review Provenance — PASS

- Reviewer's `invocation_id` (`femdation-cheap25-batch-vb-rz9ey-state4-proof-plan-reviewer`) is independent of planner's `invocation_id` (`femdation-cheap25-batch-vb-rz9ey-state4-proof-planner`).
- Same `host_session_id` (`femdation-cheap25-batch`) is allowed (control-plane convention).
- `verifier-lane-review.jsonl` carries 14 rows with `planner_invocation_id` and `reviewer_invocation_id` distinct per row, all `reviewer_disposition: accepted`.
- This `proof-plan-review.md` includes the canonical `reviewer_skill`, `reviewer_invocation_id`, `review_state`, reviewed-artifact hashes, and explicit STATUS line per `review-provenance.md`.

### 15. Forbidden Actions Honored — PASS

| forbidden action | confirmed absent |
|------------------|------------------|
| no behavior change | `behavior_affecting: false` in contract, strategy, trusted-base, obligations, waiver row |
| no public API widening | `Cargo.toml [features]` retains `default = []` (line 22); `WorkflowSourceParts` stays `pub(crate)` under `cfg(not(any(test, feature="test-util")))` (workflow.rs:107-127); `cargo doc -p vb_compile --no-deps | grep -c WorkflowSourceParts` expected 0 in PO-002 |
| no production Rust edit | PO-001 expected_evidence cites no source edits; hazard-analysis H-05 explicitly marks source files off-limits |
| no proof/model/harness code | `proof-writer` lane is SKIPPED per `proof-to-implementation-input.md` §7 |
| no test code added | existing 9 test files are the validation surface; no new tests |
| no CI config touched | no moon/CI files in scope |
| no Verus/Kani/Flux/Loom/Miri/cargo-fuzz obligation | all 12 default-profile lanes are `not_applicable` |
| no trust markers | `trusted-base-plan.md` has zero entries |

---

## Spot-Check Evidence (Reviewer-Independent Verification)

1. **`Cargo.toml` line citations verified**:
   - `[dev-dependencies]` section starts at line 18, ends before `[features]` at line 21 (per `read` of `crates/vb_compile/Cargo.toml`).
   - `[features]` block at lines 21-23 declares `default = []` and `test-util = []` exactly as the planner claims.
   - Hard constraints from `contract.md §3.1` ("line MUST live in [dev-dependencies]") are satisfied by the proposed insertion point.

2. **`lib.rs:241` cfg-gated re-export verified**:
   - `crates/vb_compile/src/lib.rs:241-242` reads `#[cfg(any(test, feature = "test-util"))] pub use yaml_ast::types::WorkflowSourceParts;` — the exact line rustc's help note pinpoints. Matches `codebase-map.md` line 87-89 and `traceability-matrix.jsonl` tm-vb-rz9ey-01.

3. **`ast.rs` does not export `WorkflowSource` (OI-1)**:
   - `crates/vb_compile/src/ast.rs` exports `WorkflowAst` (not `WorkflowSource`). Kani harnesses' `use crate::ast::{...WorkflowSource, WorkflowSourceParts}` is broken at compile time, but the Kani harnesses are `cfg(kani)`-gated and excluded from `cargo build --tests`. The planner correctly cites this as a pre-existing latent defect (OI-1), explicitly out-of-scope per `contract.md §10`. This does NOT affect PO-001.

4. **`ast/mod.rs:33-34` cfg-gated re-export verified**: consistent with the cargo-feature activation model.

5. **`vb_cli/Cargo.toml:8` and `workspace_tests/Cargo.toml:39` downstream declarations**: confirmed by `codebase-map.md` lines 162-169; both consumers declare `vb_compile = { path = "../vb_compile" }` with NO features activated, so cargo's per-build-graph feature unification isolates `test-util` to `vb_compile`'s own test binary. PO-002 is the verifier of this invariant.

---

## Findings

No findings at any severity. The plan is precise, scope-bounded, internally consistent, and faithful to the contract's hard constraints.

| severity | count | notes |
|----------|-------|-------|
| blocker | 0 | — |
| major | 0 | — |
| minor | 0 | — |
| observation | 0 | — |

---

## Cross-Cutting Observations (Non-Blocking, FYI Only)

These observations are not findings and do not affect approval. They are noted for downstream agents' situational awareness:

1. **Waiver ledger anchor `WVR-000`**: the single waiver-candidate row uses `proof_seed_id: PS-000` and `requirement_id: n/a (bead-scoped waiver ledger)`. This is a meta-ledger pattern (self-audit anchor). Downstream agents should not interpret it as a real waiver requiring `formal-waiver/v1` materialization.

2. **OI-1 Kani latent defect** (per `codebase-map.md` Q1 and `hazard-analysis.md` H-09): the 6 Kani harnesses at `src/kani_digest_ask_*.rs` and `src/kani_step_primitive_no_panic.rs` use `use crate::ast::{...WorkflowSource, WorkflowSourceParts};` but `crate::ast` re-exports `WorkflowAst` only. This is a pre-existing latent defect, explicitly out-of-scope for vb-rz9ey, and does not block the cargo-build/cargo-doc evidence surface of PO-001/PO-002. Flagged for a future bead.

3. **H-06 / H-08 follow-up hazards** (field-shape divergence and direct-downstream-import risks): cumulative drift hazards noted as out-of-scope for vb-rz9ey but worth tracking.

---

## Final Disposition

| disposition | count |
|-------------|-------|
| `fixed_with_evidence` | 0 |
| `owner_approved_debt` | 0 |
| `owner_approved_no_action` | 0 |
| `blocker` | 0 |

**All 14 verifier-lane-decision rows are accepted** (`VLR-001..VLR-014`, `reviewer_disposition: accepted`). The plan is internally complete, precise enough for `proof-writer` (which is SKIPPED for this bead, consistent with zero-obligation scope) and `proof-to-implementation` (which is bridged by `proof-to-implementation-input.md`), and conforms to all 15 review categories enumerated in `plan-review-rubric.md`.

---

## State Transition

`vb-rz9ey` is approved to advance from State 4 (proof-planner) through State 4b (proof-plan-reviewer) to:

- **State 5 (proof-writer)**: SKIPPED — zero proof/model/harness artifacts to write (per `proof-to-implementation-input.md` §7 and `proof-strategy.md` §11).
- **State 6 (holzman-rust)**: ACTIVE — edit `crates/vb_compile/Cargo.toml [dev-dependencies]` per `proof-to-implementation-input.md` §4 and `contract.md` §3.1; regenerate `Cargo.lock`; run `moon run :lint-src`.
- **State 7 (proof-to-implementation)**: ACTIVE — bridge is already produced as `proof-to-implementation-input.md` (no separate State 7 materialization needed).
- **State 8 (black-hat-reviewer)**: pending — verify PO-001 + PO-002 sub-evidence, Cargo.lock diff, downstream builds.
- **State 12 (formal-verifier)**: pending — run PO-001 and PO-002 evidence commands, populate `verification-ledger.jsonl`.

---

# STATUS: APPROVED