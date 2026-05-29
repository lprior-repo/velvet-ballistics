# Proof Plan Repair Guide — vb-7m21 State 4 Replan

## Root Cause

The original proof plan (proof-planner-vb-7m21-state4-001) was approved with the default Rust behavior profile (Verus + Kani + Flux + proptest) plus TLA+ and fuzz, yielding 39 required proof obligations across 8 verifiers. This was over-scoped for a **test-first bead** whose primary deliverable is `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` — a test fixture corpus file.

## Required Repair: Replan with Reduced Scope

### Smallest State to Rerun

State 4 (proof-planner). The State 3 contract artifacts (`contract.md`, `proof-seeds.jsonl`, `traceability-matrix.jsonl`, etc.) are accepted and do not need rework.

### Lane Decision Changes

The planner must produce new `verifier-lane-decisions.jsonl` and `proof-obligations.planned.jsonl` reflecting this profile:

| Verifier | Seeds PS-001..003 (codec boundary) | Seeds PS-004..008 (integration) | PS-009 (REQ-16) |
|----------|-------------------------------------|---------------------------------|------------------|
| tla-plus | not_applicable | not_applicable | not_applicable |
| verus | not_applicable | not_applicable | not_applicable |
| kani | **required** (bounded encode/decode) | not_applicable | not_applicable |
| flux-rs | not_applicable | not_applicable | not_applicable |
| loom | not_applicable (no concurrency) | not_applicable | not_applicable |
| miri | not_applicable (no unsafe) | not_applicable | not_applicable |
| proptest | **required** | **required** | not_applicable |
| cargo-fuzz | **required** (binary envelope) | not_applicable | not_applicable |

### Justification

1. **No Verus**: Test-first bead has no new production implementation functions to bind `spec fn`/`proof fn` contracts to. The existing `vb_storage` API surface is the target of test assertions, not a new implementation requiring Verus proof.

2. **No Flux**: Test-first bead introduces no new behavior-affecting Rust code to annotate with refinement types. The existing `vb_storage` codec already has refinements (if any) from prior beads.

3. **No TLA+**: The bead is local, deterministic, synchronous test infrastructure per `boundary-map.md:36-39`. Temporal ordering (journal gaps, duplicate events, snapshot recovery) is observable through behavior test assertions on public APIs, not through TLA+ model checking.

4. **Kani bounded to codec seeds only**: Seeds PS-001 (oversized payload), PS-002 (unknown schema), and PS-003 (truncated header) exercise bounded codec classification over RECORD_HEADER_BYTES frames. Kani is well-suited for bounded panic-freedom and typed outcome classification here. Seeds PS-004 through PS-008 operate through higher-level public storage APIs (FjallJournal, events_for_run, etc.) where bounded model checking adds little value over proptest + behavior test assertions.

5. **Proptest for all behavior seeds**: All 8 behavior-affecting seeds (PS-001 through PS-008) require proptest obligations for deterministic fixture generation with typed outcome assertions in `restate_storage_blackhat_fixture_corpus.rs`.

6. **Cargo-fuzz for codec seeds only**: Parser/codec/hostile byte-input surfaces exist only for seeds PS-001, PS-002, and PS-003 (binary envelope decode). Seeds PS-004 through PS-008 have no parser/codec surface.

### Expected New Obligations

- **Kani**: 3 obligations (PO-vb-7m21-002, 007, 012) — exact commands, workdir, and model bounds from existing plan preserved.
- **Proptest**: 8 obligations — deterministic fixture generation runs via `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` with per-seed expected typed outcomes.
- **Cargo-fuzz**: 3 obligations — 60-second smoke runs for binary envelope hostile input.

**Total: 14 required obligations** (down from 39).

### Unchanged Artifacts

- `waiver-candidates.jsonl`: The single non-behavior-affecting waiver candidate remains valid.
- `trusted-base-plan.md`: Must be replanned for the reduced obligation set but the trust strategy is unchanged.
- `proof-to-implementation-input.md`: Must reflect the reduced obligation set (14 obligations rather than 39).

### Verification After Repair

After the planner produces new lane decisions and obligations:
1. Run the proof-plan-reviewer schema validation script from `state4-pre-review-validation-evidence.json`.
2. Ensure 72 lane decisions × new applicability, 14 required obligations, 0 blocked tooling, 0 behavior waivers.
3. Submit for proof-plan-review.
