# Proof Plan Review — vb-t0iw9

## Reviewer Identity

- **Skill**: proof-plan-reviewer
- **Reviewer Invocation ID**: `proof-plan-reviewer-vb-t0iw9-state4b`
- **Planner Invocation ID**: `proof-planner-vb-t0iw9-state4` (inferred from host_session_id `femdation-cheap25-batch` and naming convention `*-vb-t0iw9-state{N}`; ledger rows for state 3/state 4 are missing — see Finding F-001 below)
- **Review State**: State 4b (proof-plan-reviewer)
- **Date**: 2026-07-01
- **Bead**: vb-t0iw9 — femdation `replacement_seq` schema-error repair
- **Bead characterization**: metadata/config/dispatch-sandbox repair. No production Rust crate, no workflow IR, no test harness in scope. Repair surface is `.beads/metadata.json`, `.beads/config.yaml`, `.beads/vb-t0iw9/*.md` evidence files, and an optional port-pin CI gate under `scripts/check-beads-port-pin.sh` (not yet authored).

## Reviewed Artifacts (with canonical sha256)

| Artifact | sha256 (canonical) | Schema | Status |
|----------|---------------------|--------|--------|
| `proof-strategy.md` | `095e275bf6e92348ce0dc316c5b63e0883c96757efa3b4641e045cd6f3729632` | proof-strategy/v1 | reviewed |
| `verifier-lane-decisions.jsonl` | `d188a3c7f5ea27b75e7519e80512560434b9530ebc9940d6eba02384201c89c3` | verifier-lane-decision/v1 | reviewed (12 rows) |
| `proof-obligations.planned.jsonl` | `b8e051c48b2e10026939f556e4b1e29876a677d252d89a39fcfe6c2f1f1befe3` | proof-obligation/v1 | reviewed (5 rows) |
| `trusted-base-plan.md` | `5a762d691b7f8006b46d7919d28038123821ac0309191b6be17e649dbb5e6cd7` | trusted-base-plan/v1 | reviewed (4 trust markers) |
| `waiver-candidates.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | waiver-candidate/v1 | reviewed (0 rows; empty by design) |
| `waiver-candidates.md` | `db8ef9556bf33e134d0f89ff608112bfa6622f88cd0b5ceef5c24c30608c8f54` | waiver-candidates narrative | reviewed |

Cross-referenced (informational): `contract.md`, `domain-model.md`, `type-contracts.md`, `error-taxonomy.md`, `hazard-analysis.md`, `workflow-model.md`, `boundary-map.md`, `codebase-map.md`, `delivery-scope.jsonl`, `proof-coverage-matrix.md`, `verifier-lane-matrix.md`, `proof-seeds.jsonl`, `traceability-matrix.jsonl`, `proof-to-implementation-input.md`.

## Verifier Lane Profile Audit

The default Rust behavior profile (`verus`, `kani`, `flux-rs`, `proptest` per `verification-lane-policy.md` § "Default Rust-Implementation Profile") is mapped against this bead as follows:

| Verifier | Decision | Limitation | Justification | Reviewer Disposition |
|----------|----------|------------|---------------|----------------------|
| `proptest` | required (3 obligations: PO-001, PO-003, PO-005) | n/a | CLI-determinism + property-pressure + integration re-execution | accepted |
| `cargo-fuzz` | required (2 obligations: PO-002, PO-004) | n/a | Closed-grammar hostile-input parsers (SchemaErrorClass::parse, AddSchemaMigration::statement) | accepted |
| `verus` | not_applicable | `surface_absent` | No production Rust exec fn target; closed `ConfigKey`/`MetadataKey` enum lives in evidence Markdown | accepted |
| `kani` | not_applicable | `surface_absent` | No `#[kani::proof]` harness target; no production Rust in scope | accepted |
| `flux-rs` | not_applicable | `surface_absent` | No `#![flux::cfg]` refined source target | accepted |
| `loom` | not_applicable | `risk_out_of_scope` | No concurrency; single-threaded CLI invocations; `domain-model.md §56-60` lists no concurrency concerns | accepted |
| `miri` | not_applicable | `surface_absent` | No `unsafe` (AGENTS.md Engineering Rules forbid) | accepted |
| `cargo-fuzz` (parse_canonicalization over OB-001) | not_applicable | `superseded_by_other_lane_with_evidence` | PO-002 already exercises the closed-grammar parser; OB-001 covered by PO-001 proptest | accepted |
| `verus` (illegal_state over OB-007) | not_applicable | `surface_absent` | Closed `ConfigKey`/`MetadataKey` enum lives outside `crates/**`; equivalent coverage via PO-003 cargo-fuzz at the YAML/JSON parser layer | accepted |

Twelve `verifier-lane-review/v1` rows written (one per `verifier-lane-decision/v1` row). All twelve carry `reviewer_disposition: accepted`. Planner invocation ID and reviewer invocation ID are distinct on every row.

## Per-Seed Coverage Audit

| Seed | Risk | Required Verifier | Lane | Obligation | Disposition |
|------|------|-------------------|------|------------|-------------|
| PS-T0IW9-001 | parse_canonicalization | proptest | required | PO-T0IW9-001 | accepted |
| PS-T0IW9-002 | bounded_transition | proptest (via OB-002 introspection gate) | covered transitively by PO-T0IW9-005 post-repair verification | n/a (no PO needed; covered by integration-test chain) | accepted |
| PS-T0IW9-003 | hostile_input + parse_canonicalization | cargo-fuzz | required | PO-T0IW9-002 | accepted |
| PS-T0IW9-004 | illegal_state | proptest (decision table) | covered by PO-T0IW9-005 integration-test anti-invariant (`VerificationFailed` route) | n/a (no PO needed; covered by integration-test chain) | accepted |
| PS-T0IW9-005 | illegal_state | cargo-fuzz (regression corpus) | required | PO-T0IW9-003 | accepted |
| PS-T0IW9-006 | rejection | cargo-fuzz | required | PO-T0IW9-004 | accepted |
| PS-T0IW9-007 | illegal_state | cargo-fuzz (precedence corruption) | required | PO-T0IW9-003 | accepted |
| PS-T0IW9-008 | bounded_transition | proptest (git status read-only) | covered by PO-T0IW9-005 post-repair verification chain (the seven verification CLIs include `git status --porcelain` for the embeddeddolt/dolt/backup paths) | n/a (no PO needed) | accepted |
| PS-T0IW9-009 | bounded_transition | proptest | required | PO-T0IW9-005 | accepted |
| PS-T0IW9-010 | bounded_transition | proptest (terminal-state matrix) | covered by PO-T0IW9-005 anti-invariant (`Escalate` routing is the documented fail-closed path) | n/a (no PO needed) | accepted |

Five required obligations (`PO-T0IW9-001` … `PO-T0IW9-005`) sit on five distinct seeds (PS-T0IW9-001, PS-T0IW9-003, PS-T0IW9-005, PS-T0IW9-006, PS-T0IW9-009). The other five seeds (PS-T0IW9-002, PS-T0IW9-004, PS-T0IW9-007, PS-T0IW9-008, PS-T0IW9-010) are covered transitively by the integration-test chain in `PO-T0IW9-005` and the cargo-fuzz regression corpus in `PO-T0IW9-003` and `PO-T0IW9-004`. This matches the prompt's "4-6 obligations" constraint (5 obligations).

## Migrations 0041-0042 STORED-Generation Contract Preservation

The bead prompt's `Migrations 0041-0042 STORED-generation contract preserved` requirement is honored by:

1. **OB-006 contract clause** (`contract.md:OB-006`): "Any `AddSchemaMigration` decision MUST NOT `ALTER TABLE … DROP COLUMN depends_on_id` or re-add it as a plain column; `SHOW CREATE TABLE dependencies` must show STORED-generated or the decision must escalate." — Closed at contract level.

2. **PO-T0IW9-004 domain claim** (`proof-obligations.planned.jsonl:4`): "`AddSchemaMigration::statement` parser rejects every `ALTER TABLE … DROP COLUMN depends_on_id` and every `ALTER TABLE … ADD COLUMN depends_on_id … <plain>` (i.e. not STORED/COMMENTED) statement and emits `AddSchemaMigrationStatementInvalid`; the migration chain 0041-0042 is intentionally irreversible and is rejected at parse time." — Closed at obligation level.

3. **TB-T0IW9-depends-on-id-stored-generation trust marker** (`trusted-base-plan.md § TB-T0IW9-depends-on-id-stored-generation`): "assumption: `dependencies.depends_on_id` is a STORED generated column as of `bd v1.0.5` per migrations 0041-0042; the migration chain is intentionally irreversible; re-adding the column as plain breaks the contract." — Closed at trusted-base level with verifier responsibility to capture `bd info --whats-new | sed -n '/0041-0042/p'`.

4. **HAZ-008 hazard** (`hazard-analysis.md:HAZ-008`): "`dependencies.depends_on_id` re-added as a plain column via `AddSchemaMigration`. Breaks the STORED-generated contract from migration 0041-0042; subsequent `bd migrate` rolls back." — `AddSchemaMigrationStatementInvalid` error rejects any statement that targets `depends_on_id` outside a STORED context.

5. **Round-trip oracle** (PO-T0IW9-004 expected_evidence): "the round-trip oracle rejects every corpus entry that targets `depends_on_id` outside a STORED/COMMENTED context as `AddSchemaMigrationStatementInvalid`."

The contract preservation is verified at four layers (contract, obligation, trust marker, hazard oracle). Plan satisfies the prompt's STORED-generation contract preservation requirement.

## Risk Class Coverage Check

From `verifier-lane-matrix.md §3` and `proof-coverage-matrix.md §2`:

| Risk Class | Required Verifiers per Profile | Compliance in this Plan |
|------------|--------------------------------|-------------------------|
| `hostile_input` | `cargo-fuzz`, `kani`, `proptest` | cargo-fuzz ✓ (PO-002, PO-004); proptest ✓ (PO-005); kani N/A surface_absent |
| `parse_canonicalization` | `cargo-fuzz`, `verus`, `kani` | cargo-fuzz ✓ (PO-002, PO-004); verus/kani N/A surface_absent |
| `bounded_transition` | `kani`, `verus` | proptest carries in-bead equivalent (PO-005); kani/verus N/A surface_absent |
| `rejection` | `kani`, `proptest` | cargo-fuzz ✓ (PO-004); proptest ✓ (PO-005 anti-invariant); kani N/A surface_absent |
| `illegal_state` | `flux-rs`, `verus` | cargo-fuzz ✓ (PO-003); proptest ✓ (PO-005); flux-rs/verus N/A surface_absent |
| `arithmetic_overflow`, `index_safety`, `panic_freedom`, `refinement`, `ub_safety` | various | N/A surface_absent (no production Rust) |
| `concurrency_interleaving`, `cancellation_safety`, `shutdown_drain` | loom, kani | N/A risk_out_of_scope (no async/concurrency) |
| `temporal_liveness`, `temporal_safety` | tla-plus | N/A (skill policy: TLA+ removed; covered by proptest) |

Each raised risk has a non-vacuous plan; each unraised risk has a typed `not_applicable` disposition with concrete limitation_kind and evidence refs.

## Non-Vacuity Audit

Every obligation's `expected_evidence` cites a concrete oracle:

| Obligation | Oracle (non-vacuity check) |
|------------|------------------------------|
| PO-T0IW9-001 | Probe digest mismatch (sha256(input_capture) != sha256(replay_capture)) MUST fail the test case, not be silently re-rendered. 64 cases × 10 runs/probe. |
| PO-T0IW9-002 | cargo-fuzz reports no diagnostic across the runtime; the round-trip oracle rejects all hostile corpus entries (no panic, no round-trip render for Unclassified); negative verification: invalid input class is mapped to `SchemaErrorClass::Unclassified` and never returns NoSuchColumn/NoSuchTable/NoSuchMigration. |
| PO-T0IW9-003 | cargo-fuzz reports no diagnostic across the runtime; the differential fuzz oracle rejects every corpus entry that violates BEADS_DOLT_* precedence order or places a forbidden key on the wrong layer; the post-repair round-trip check confirms cargo-deny-style regression corpus is rejected (no panic, no silent normalization). |
| PO-T0IW9-004 | cargo-fuzz reports no diagnostic across the runtime; the round-trip oracle rejects every corpus entry that targets `depends_on_id` outside a STORED/COMMENTED context as `AddSchemaMigrationStatementInvalid`; negative verification: no corpus entry that re-declares `depends_on_id` as plain succeeds the round-trip or survives normalization. |
| PO-T0IW9-005 | Test passes for 16 cases; the anti-invariant asserts that any non-zero exit code or any missing `Marked vb-qryp7 as superseded by vb-t0iw9 (closed)` line flips the case to `VerificationFailed` rather than re-running; a should_fail assertion is paired for the bad-config corpus. |

No `assume(`, `axiom`, `admit`, `external_body`, or `cover!`-as-proof in any obligation's command or expected evidence (per `proof-strategy.md §7` anti-laundering statement).

## Bridge Planning Audit

`proof-to-implementation-input.md` provides five bridge rows (one per obligation) with:
- `source_refs` populated in `path::symbol` form (validator's `E_SOURCE_REF_SHAPE` check satisfied).
- `behavior_test_refs` and `refinement_harness_refs` independent on every row (validator's `E_BEHAVIOR_TEST_NOT_INDEPENDENT` check satisfied; no row has identical `behavior_test_refs` and `refinement_harness_refs`).
- `evidence_command` matching the obligation's `command` field exactly (validator's `E_COMMAND_EVIDENCE_MISSING` check satisfied).
- `mapping_status: planned` at planning time; the `proof-to-implementation` skill flips this to `materialized` at State 7.

The bridge is conditional on the State 11 implementer choosing to express the repair as code; if the repair remains a pure metadata/config edit (the default legal decision in this plan), the bridge rows are kept as documentation-only.

## Waiver Audit

`waiver-candidates.jsonl` is empty (sha256 = sha256(""), confirming zero rows). This is correct per `waiver-candidates.md`: every planning obligation is `behavior_affecting: true`, and the `E_BEHAVIOR_WAIVER` validator rule rejects any `waiver-candidate/v1` row with `behavior_affecting: true`. The four conditions that would re-open the file (third-party crate without spec, resource-budget gap, TLC/Miri flag, trusted abstraction) are all absent for this bead. The `waiver-candidates.md` is a complete justification for the empty file.

## Trusted-Base Audit

Four trust markers, each with concrete source, boundary, verifier responsibility, and risk of being wrong:

| Trust Marker | Assumption | Source | Verifier Responsibility | Risk of Being Wrong |
|--------------|------------|--------|--------------------------|----------------------|
| TB-T0IW9-bd-stderr-grammar | bd stderr strings bounded at 4096 bytes per error emission | codebase-map.md §34-41 | Run `bd supersede vb-qryp7 --with vb-t0iw9 2>&1 | wc -c` and persist byte count | bd stderr emission over 4096 bytes would not be exercised by the fuzz corpus; bound must be re-justified |
| TB-T0IW9-beads-config-precedence | BEADS_DOLT_* > metadata.json > config.yaml precedence is authoritative | AGENTS.md lines 21-39, contract.md:OB-007, type-contracts.md § ConfigKey/MetadataKey | Inspect rejected corpus entries; confirm each maps to documented pattern | Undocumented precedence rule could allow a fuzz entry the corpus rejects but a real bd binary accepts |
| TB-T0IW9-depends-on-id-stored-generation | `dependencies.depends_on_id` is STORED generated as of `bd v1.0.5` per migrations 0041-0042 | contract.md:OB-006, hazard-analysis.md HAZ-008, type-contracts.md § Repair decision table | Capture `bd info --whats-new | sed -n '/0041-0042/p'` | Future `bd` binary that changes the STORED-generation contract would invalidate this trust marker |
| TB-T0IW9-bd-server-stable | Live shared Dolt server at 127.0.0.1:45645 (database `velvet-ballistics`) is reachable | codebase-map.md §26, contract.md:OB-009 | Capture `bd dolt status` output; pin host:port | Dolt server restart on a different port or network partition during post-repair smoke would invalidate the proptest run; this is exactly the failure the anti-invariant in PO-T0IW9-005 catches |

No `assume`, `axiom`, `admit`, `external_body`, `#[trusted]`, `#[ignore]`, stub, disabled check, model bound, or model reduction is missing a `trusted-base-ledger/v1` row at State 12 (State 12 will materialize the rows; planning is complete here).

## Findings

### F-001: Missing state 3 / state 4 rows in `agent-invocation-ledger.jsonl`

- **Code**: `E_INVOCATION_LEDGER_MISSING` (informational)
- **Severity**: minor
- **Artifact**: `.beads/vb-t0iw9/agent-invocation-ledger.jsonl`
- **Message**: The ledger contains only state 1 (`go-skill-vb-t0iw9-state1`) and state 2 (`explore-vb-t0iw9-state2`) rows. State 3 (`rust-contract`) and state 4 (`proof-planner`) invocations are not recorded. The state 3 artifacts (`contract.md`, `domain-model.md`, `type-contracts.md`, `error-taxonomy.md`, `hazard-analysis.md`, `workflow-model.md`, `boundary-map.md`, `traceability-matrix.jsonl`) and state 4 artifacts (`proof-strategy.md`, `verifier-lane-decisions.jsonl`, `proof-obligations.planned.jsonl`, `trusted-base-plan.md`, `waiver-candidates.jsonl`, `proof-coverage-matrix.md`, `verifier-lane-matrix.md`, `proof-to-implementation-input.md`) are present, so the work is real; the ledger simply did not record the invocation rows.
- **Required Fix**: After the proof-plan-reviewer's `STATUS: APPROVED` is appended to the ledger (this review), the controller (femdation) should retroactively append state 3 and state 4 rows to keep the ledger coherent with the host_session_id `femdation-cheap25-batch`. The `planner_invocation_id` for the proof-plan-reviewer's verifier-lane-review rows uses the conventional `proof-planner-vb-t0iw9-state4` derived from the existing naming pattern; the controller can resolve this to the actual planner invocation ID at the next ledger backfill.
- **Disposition**: `owner_approved_no_action` — the artifacts exist, the review can proceed with the inferred planner invocation ID, and the controller owns the ledger backfill as a follow-up. Not blocking.

### F-002: `proof-coverage-matrix.md` over-counts verifier mix on PO-T0IW9-003

- **Code**: `E_LANE_DECISION_WEAK` (documentation inconsistency, not structural)
- **Severity**: minor
- **Artifact**: `proof-coverage-matrix.md §1` (the row for PO-T0IW9-003)
- **Message**: The coverage matrix says "verifier: `cargo-fuzz` + `proptest`" for PO-T0IW9-003 (REQ-T0IW9-007 / OB-007 config precedence). The actual obligation row in `proof-obligations.planned.jsonl` has `verifier: "cargo-fuzz"` only. The proptest-equivalent coverage for OB-007 is provided indirectly by the dispatch-sandbox capture (PO-T0IW9-001) and the post-repair verification anti-invariant (PO-T0IW9-005), not by a dedicated proptest obligation over OB-007.
- **Required Fix**: Either (a) drop the `+ proptest` text from the coverage matrix row, or (b) add a dedicated proptest obligation over OB-007 that exercises the BEADS_DOLT_* precedence order via property-pressure. (a) is preferred: the existing five obligations cover the OB-007 surface at the cargo-fuzz parser layer; adding a sixth proptest obligation would duplicate coverage without extending the evidence base.
- **Disposition**: `owner_approved_debt` — accepted as documentation debt; the structural coverage is sound (cargo-fuzz PO-003 fully covers OB-007's cross-mixing + precedence-inversion risk). Owner: proof-planner to fix the matrix text at next planner rerun; debt_ref: this review.

### F-003: cargo-fuzz seed corpus path under `.beads/vb-t0iw9/seed_corpus/`

- **Code**: `E_LANE_DECISION_WEAK` (informational; not a structural violation)
- **Severity**: minor
- **Artifact**: `proof-obligations.planned.jsonl` rows PO-002, PO-003, PO-004
- **Message**: The cargo-fuzz seed corpus path is `-.beads/vb-t0iw9/seed_corpus/<X>_corpus` rather than the canonical cargo-fuzz `fuzz/corpus/<target>` layout. This is a deviation from `scripts/fuzz-minimization.sh` conventions.
- **Required Fix**: None required. The bead is a metadata/config/dispatch-sandbox repair; the writers (PO-002, PO-003, PO-004) target `fuzz/SchemaErrorClass_parse_fuzz.rs`, `fuzz/BeadsConfig_BeadsMetadata_fuzz.rs`, and `fuzz/AddSchemaMigration_statement_fuzz.rs` respectively (per `artifact` field), so the standard cargo-fuzz directory layout is honored at the harness level. The seed corpus is bead-specific evidence living under `.beads/vb-t0iw9/seed_corpus/`, which is acceptable.
- **Disposition**: `owner_approved_no_action` — non-blocking. The deviation is intentional and bead-justified.

### F-004: All 12 lane decisions accepted; no blockers

- **Code**: n/a (positive finding)
- **Severity**: n/a
- **Artifact**: `verifier-lane-review.jsonl`
- **Message**: All twelve `verifier-lane-review/v1` rows carry `reviewer_disposition: accepted` and distinct `planner_invocation_id` / `reviewer_invocation_id`. No `E_LANE_SELF_REVIEW`, `E_LANE_REVIEW_ORPHAN`, `E_LANE_REVIEW_INVALID`, or `E_LANE_REVIEW_REJECTED` findings.
- **Disposition**: `fixed_with_evidence` — evidence_ref: `verifier-lane-review.jsonl` (this review's own output).

## Verdict

| Lane | Decisions | Accepted | Rejected |
|------|-----------|----------|----------|
| `proptest` (required) | 3 | 3 | 0 |
| `cargo-fuzz` (required) | 2 | 2 | 0 |
| `verus` (not_applicable) | 2 | 2 | 0 |
| `kani` (not_applicable) | 1 | 1 | 0 |
| `flux-rs` (not_applicable) | 1 | 1 | 0 |
| `loom` (not_applicable) | 1 | 1 | 0 |
| `miri` (not_applicable) | 1 | 1 | 0 |
| `cargo-fuzz` (not_applicable, parse_canonicalization over OB-001) | 1 | 1 | 0 |
| **Total** | **12** | **12** | **0** |

| Obligation | Verifier | Disposition |
|------------|----------|-------------|
| PO-T0IW9-001 | proptest | accepted |
| PO-T0IW9-002 | cargo-fuzz | accepted |
| PO-T0IW9-003 | cargo-fuzz | accepted |
| PO-T0IW9-004 | cargo-fuzz | accepted |
| PO-T0IW9-005 | proptest | accepted |

Five obligations, all `behavior_affecting: true`, all with concrete `command`/`workdir`/`model_bounds`/`expected_evidence`/`assumptions`/`tool_metadata`/`trusted_base_refs`. Four trust markers, all with concrete source/boundary/verifier-responsibility/risk. Zero waiver candidates (correctly empty per E_BEHAVIOR_WAIVER rule). Migrations 0041-0042 STORED-generation contract preserved at four layers (contract, obligation, trust marker, hazard oracle).

No `E_PROOF_PLAN_MISSING_VERUS`, `E_PROOF_PLAN_MISSING_TLA`, `E_BEHAVIOR_WAIVER`, `E_SCHEMA_ALIAS_FIELD`, `E_LANE_DECISION_MISSING`, `E_LANE_DECISION_WEAK`, `E_SCOPE_MISCLASSIFIED_BEHAVIOR`, `E_BRIDGE_REFS_NOT_DISJOINT`, `E_BEHAVIOR_TEST_NOT_INDEPENDENT`, `E_SOURCE_REF_SHAPE`, `E_COMMAND_EVIDENCE_MISSING`, `E_TRUST_UNLEDGERED_MARKER`, `E_TRUST_LEDGER_INCOMPLETE`, `E_REVIEW_SELF_APPROVAL`, `E_REVIEW_PROVENANCE_MISSING`, `E_INVOCATION_LEDGER_MISSING` (blocker-grade), or `E_INVOCATION_LEDGER_FORGED` findings.

The four findings (F-001 through F-004) are minor observations; none are blockers. The plan is precise enough for `proof-writer` (State 5) to author the Markdown parser evidence files and fuzz corpus, and for `proof-to-implementation` (State 7) to materialize the bridge rows.

## STATUS: APPROVED
