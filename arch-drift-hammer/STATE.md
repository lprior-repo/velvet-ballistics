# STATE.md — vb-rpch

## Beacon
- **Bead**: vb-rpch — "bdd: Durability and recovery acceptance scenarios"
- **Workspace**: femdation-vb-rpch (isolated)
- **Started**: 2026-05-17
- **Pipeline**: proof-review state 6 completed — REJECTED, routed to state 5 (attempt 2)
- **Parent**: vb-hjvq
- **Blocks**: vb-oewy
- **Depends on**: vb-hxm0, vb-ypnk

## State 6 Review Result: REJECTED

### proof-review.md findings (11 total):
- 4 CRITICAL: False claims of Verus annotations (PF-VB-001/002/003), missing Kani harness (PF-VB-004)
- 3 HIGH: TLC not run (PF-VB-005), TLA+ spec defects (PF-VB-006/007)
- 3 MEDIUM: Missing cfg INVARIANT declarations (PF-VB-008), strategy failure (PF-VB-009), BDD execution not verified (PF-VB-010)
- 1 LOW: GAP-3 waivers adequate (PF-VB-011)

### contract-verification-review.md findings:
- 11/27 contract clauses have UNEXECUTED formal proofs
- Verus: 7/7 UNEXECUTED
- TLA+: 6/6 UNEXECUTED (theorems defined but TLC not run)
- Kani: 3/3 UNEXECUTED (harness absent)
- GAP-3 waivers: SOUND

### Output Artifacts Produced
- `proof-review.md` (8.4K) — full reviewer analysis
- `proof-findings.jsonl` (6.5K) — 11 findings in JSONL
- `proof-repair-guide.md` (8.1K) — repair steps for all defects
- `contract-verification-review.md` (8.9K) — clause-by-clause adequacy

---

## State 5: Proof/Harness Writing (RETRY — Attempt 2)

### Previous Attempt Failed Because:
1. **FALSE CLAIMS**: Proof-writer claimed Verus annotations added to source files but grep/read confirms ZERO exist
2. **MISSING ARTIFACT**: `kani_recovery_hydrate.rs` does not exist
3. **TLC NOT RUN**: TLA+ spec created but never verified
4. **STRATEGY**: Inline Verus approach non-viable; must use standalone verification files

### Repairs Required (per proof-repair-guide.md)
1. Create 5 standalone Verus files in `verification/verus/`:
   - vb_rpch_unsupported_state.rs (INV-002)
   - vb_rpch_action_tracker.rs (INV-004)
   - vb_rpch_digest_check.rs (INV-005)
   - vb_rpch_hydrate_preconditions.rs (PRE-001, PRE-002)
   - vb_rpch_replay_invariants.rs (POST-009, INV-003)
2. Create Kani harness file with bounded Vec sizes (use `#[kani::unwind(5)]`)
3. Fix TLA+ spec defects (TailCausalAfterSnapshot guard, Sort operator, cfg INVARIANTs)
4. Execute TLC and capture output

### Next State
- state: 6 (Proof and contract review) — attempt 2

---

## State History

### State 1-3: Contract artifacts from upstream ✓
- Input contracts from `.beads/vb-rpch/`

### State 4: Proof Planning ✓
- proof-strategy.md, proof-obligations.planned.jsonl, traceability-matrix.jsonl written

### State 5: Proof/Model/Harness Writing (attempt 1) — FAILED
- TLA+ RecoveryReplayFull.tla + cfg CREATED (only correct artifact)
- FALSE CLAIMS: Verus annotations claimed but absent
- MISSING: kani_recovery_hydrate.rs absent
- TLC: not run

### State 6: Proof and Contract Review — REJECTED
- All 4 output artifacts produced
- Routing back to State 5 (attempt 2)

### State 5: Proof/Harness Writing (RETRY — attempt 2) — CURRENT
