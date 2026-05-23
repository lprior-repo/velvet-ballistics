# Proof Plan Review Refresh: vb-jpq7.3

reviewer_skill: proof-plan-reviewer  
reviewer_invocation_id: proof-plan-reviewer-gpt55-2026-05-23-vb-jpq7-3-refresh  
review_state: pre-proof-plan-review-refresh  
verdict: REJECT

## Reviewed Artifacts

- `.beads/vb-jpq7.3/proof-strategy.md` sha256 `22255a177024aba48a0706da7783a8ffb59c89133a4695c632b42744317bec9a`
- `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl` sha256 `cb081836b89fd67d03254e6487ce596c229e050ade9c8f98bd2cbc52f4272eeb`
- `.beads/vb-jpq7.3/proof-obligations.planned.jsonl` sha256 `b1e868292cbd366061b759033a0b6f825fc5a9e111bbf47aa70184f5efe9fd74`
- `.beads/vb-jpq7.3/trusted-base-plan.md` sha256 `51cbdc3bfc8fdbb8413162e0cde1cb3c42d27ac3882d7cdace501a24662559c4`
- `.beads/vb-jpq7.3/waiver-candidates.jsonl` sha256 `a7f6e2fb11cedc1e3c5a5cb10dd01dadece273f03d168afbab403c02cd3adfde`
- `.beads/vb-jpq7.3/traceability-matrix.jsonl` sha256 `418e829346ad50523f92ff34544962e6f46b6d54cd0e35f60ff181da61416d92`
- `.beads/vb-jpq7.3/verification-ledger.jsonl` sha256 `6a3fe49e4db0d6c5a6a44f8c2e5a3586b52702b9dd3e5d6cf9c9aae0588e45e7`
- `.beads/vb-jpq7.3/delivery-scope.jsonl` sha256 `35e9f449a45fc330fcd717ec339b43e255b0e64e0c7902c665c6ed8a9d6d287e`
- `.beads/vb-jpq7.3/global-readiness-report.md` sha256 `e01acba662a86b1cca8db5cd83a62ca6256fba5848e9f6b4db9675a08de2c6bb`
- `.beads/vb-jpq7.3/agent-invocation-ledger.jsonl` sha256 `376c9d84e39355900fe6d95afbec872851b66505121fb29172f481c3e33bbbb9`

## Refresh Judgment

The test repair evidence improved the behavior-test story for snapshot authority and global formatting: `events_for_run` is now reported as 24 passing tests, `latest_durable_snapshot_seq` as 4 passing tests, `trimming` as 25 passing tests, and `cargo fmt --all -- --check` is now reported PASS. That does not make this proof plan approvable. The proof-plan gate is still blocked by schema-invalid planner artifacts, an incomplete core verifier lane matrix, and required formal lanes that self-declare they do not prove the bead-critical claims.

The current live global blocker is correctly recognized as `moon ci` failure, not cargo fmt: `velvet-ballastics:panic-surface` fails on production `unreachable!(...)` in `crates/vb_codegen/src/parity.rs:438` and `:444`, and `velvet-ballastics:check` fails on workspace-test dead code under `-D warnings`.

## Current Blockers

### B1 — Proof pipeline schemas are still not satisfied

`verifier-lane-decisions.jsonl`, `proof-obligations.planned.jsonl`, `waiver-candidates.jsonl`, and `verification-ledger.jsonl` remain prose-shaped/ad-hoc rows, not canonical proof pipeline rows.

Required examples still missing include:

- lane decisions: `schema_version`, `id`, `requirement_id`, `contract_clause`, `proof_seed_id`, canonical `verifier`, `applicability`, `required_obligation_ids`, `non_applicability_evidence_refs`, `limitation_kind`, `owner_state`, canonical `status`;
- proof obligations: `schema_version`, `domain_claim`, `risk_tags`, `target`, `workdir`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, `behavior_affecting`, `owner_state`, `rerun_from`;
- waiver candidates: `schema_version`, `requirement_id`, `contract_clause`, `behavior_affecting`, `boundary_proof`, `owner`, `expiry`, `review_status`;
- verification ledger: `schema_version`, `id`, `obligation_id`, `obligation_kind`, `behavior_affecting`, canonical `verifier`, `result`, `workdir`, `exit_status`, `tool_version`, `flags`, `bounds`, `raw_log`, `formal_verifier_invocation_id`, `rerun_from`.

Accepted `verifier-lane-review/v1` rows cannot be issued for non-schema planner rows.

### B2 — Core verifier lane matrix is still incomplete

The core verifier set requires per `(requirement_id, contract_clause, proof_seed_id)` decisions for `tla-plus`, `verus`, `kani`, `flux-rs`, `loom`, `miri`, `proptest`, and `cargo-fuzz`. The submitted file remains one coarse row per lane. It still lacks:

- any `flux-rs` decision;
- separate `proptest` and `cargo-fuzz` decisions;
- per-requirement/per-proof-seed applicability and non-applicability evidence;
- canonical `blocked_tooling` treatment for unresolved Kani.

### B3 — TLA+ lane remains too weak for vb-jpq7.3 critical claims

The TLA+ lane still states that the current model does not encode concrete `EventSeq` `N+1` tail arithmetic or corrupt latest snapshot authority. Those are central acceptance criteria for strict snapshot-tail replay, bounded/range-start replay, typed error propagation, and fail-closed recovery. A required TLA+ lane cannot be accepted while explicitly excluding those states.

Required repair: revise `verification/tla/EngineYamlRecovery.tla`/`.cfg` or add schema-valid non-applicability decisions. If TLA+ remains required, planned evidence must include bounded integer/error transitions, `snapshot.seq + 1`, corrupt latest snapshot fail-closed behavior, sequence gaps, and no hydrated success from incomplete durable evidence. Planned command:

```bash
tlc -workers 1 -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla
```

### B4 — Verus lane remains disconnected/auxiliary

The Verus row still admits the replay artifact predates strict `snapshot+1` wording and was previously rejected as disconnected/abstract. `trusted-base-plan.md` still says the existing Verus replay artifact is auxiliary until repaired. That cannot discharge critical claims for exact replay gaps, bounded replay, typed errors, taint fail-closed, or explicit durability result propagation.

Required repair: schema-valid Verus obligations with production-shaped source bindings, non-vacuity checks, model bounds, trusted-base refs, and bridge/refinement obligations. Existing commands may be retained only if the artifacts are repaired:

```bash
verus verification/verus/recovery_hydration_contracts.rs
verus verification/verus/vb_jpq724_events_for_run_production.rs
```

### B5 — Kani remains unresolved

`cargo kani list -p vb_storage` remains listed only as feasibility discovery. There is no Kani proof harness, no harness command, no arbitrary/generator plan, no unwind/model bounds, and no non-vacuity expectation for replay arithmetic/error lattice/taint fail-closed behavior. For this repository, Kani harnesses must not rely on hardcoded dummy structures.

Required repair: add bounded Kani proof obligations and commands, or mark Kani as schema-valid `blocked_tooling` that blocks acceptance. Example command shape:

```bash
cargo kani -p vb_storage --harness <harness_name>
```

### B6 — Waivers remain coarse and schema-invalid

Miri/Loom/Fuzz waiver candidates may be plausible, but they are still not schema-valid and are not tied to individual requirements and contract clauses. The `proptest-fuzz` merged lane remains invalid because `proptest` and `cargo-fuzz` are separate core lanes. No behavior-affecting proof obligation may be waived because proof is difficult.

### B7 — Global readiness is now moon-ci blocked, not cargo-fmt blocked

`cargo fmt --all -- --check` is now reported PASS. The live closure blocker is `moon ci`:

```bash
moon ci
```

Reported failures:

- `velvet-ballastics:panic-surface`: production `unreachable!(...)` in `crates/vb_codegen/src/parity.rs:438` and `:444`;
- `velvet-ballastics:check`: workspace-test dead code under `-D warnings`.

This blocks bead closure unless repaired under prerequisite beads or waived by the release owner. Additionally, `proof-strategy.md`, `trusted-base-plan.md`, and `delivery-scope.jsonl` still contain stale rustfmt/global-readiness wording that should be reconciled with the live global-readiness report.

## Required Repair Set

1. Regenerate proof seeds, verifier lane decisions, proof obligations, waiver candidates, and verification ledger rows using canonical `proof-schemas.md`.
2. Build per-requirement/per-proof-seed decisions for all core lanes: `tla-plus`, `verus`, `kani`, `flux-rs`, `loom`, `miri`, `proptest`, `cargo-fuzz`.
3. Split `proptest-fuzz` into separate `proptest` and `cargo-fuzz` lane decisions.
4. Repair or formally block TLA+ coverage for `EventSeq` bounded arithmetic, `snapshot.seq + 1`, corrupt latest snapshot authority, sequence gaps, and fail-closed recovery outcomes.
5. Repair Verus so proofs bind to production-shaped replay/recovery/durability claims and include non-vacuity/bridge obligations.
6. Add Kani harness obligations or schema-valid `blocked_tooling` rows that explicitly block acceptance.
7. Make waiver candidates schema-valid and non-behavior-affecting, with per-requirement boundary proof and expiry.
8. Resolve current `moon ci` blocker or obtain explicit release-owner waiver; do not treat cargo fmt as the current blocker.
9. Re-run proof-plan-reviewer after artifact repair.

STATUS: REJECTED
