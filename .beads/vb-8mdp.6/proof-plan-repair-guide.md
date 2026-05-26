# Proof Plan Repair Guide — vb-8mdp.6

## Bead: vb-8mdp.6
## Reviewer: proof-plan-reviewer
## Review State: REJECTED

---

## Critical Issues (Must Fix)

### F001: Flux Waiver Weak (VLD-008, VLD-019)

**Finding**: `E_LANE_DECISION_WEAK` — Flux waiver for PS-VB-IDEM-003 and PS-VB-IDEM-009 is rejected.

**Root Cause**: Slot taint refinement (ensuring KeyRequired actions only use Clean slots) is exactly the use case for Flux refinement types. Kani+TLA+ provide runtime coverage only — they cannot prevent illegal type construction.

**Required Fix Options**:

**Option A (Preferred)**: Write the Flux refinement.
- Create/extend `verification/flux/vb_rpch_flux_rN.rs` (where N is next available number)
- Add refinement: `validate_idempotency_key_ingredients` only accepts `Taint::Clean` slots when action is KeyRequired
- Add obligation PO-VB-IDEM-003c (flux-rs, required) with proper harness
- Remove waiver status from VLD-008 and VLD-019
- Update `waiver-candidates.jsonl` to mark W001 as withdrawn

**Option B**: Downgrade contract enforcement level.
- Document that slot taint validation is RUNTIME only, not type-level
- Update contract clause PS-VB-IDEM-003 and PS-VB-IDEM-009 to reflect runtime validation scope
- This requires proof-planner to re-issue the lane decisions with updated contract language

### F002: VLD-031 Flawed Evidence Command

**Finding**: `E_COMMAND_EVIDENCE_MISSING` — `grep -i vb_storage` on cargo output is not reliable static analysis.

**Root Cause**: The command searches textual output which can be fragile (format changes, false negatives on failure, false positives from unrelated error messages).

**Required Fix**:
Replace VLD-031 evidence command with one of:
```bash
# Option A: cargo tree (preferred)
cargo tree -p vb_core -e normal 2>&1 | grep vb_storage || echo 'PASS: no vb_storage deps'

# Option B: Read Cargo.toml directly (most robust)
grep -E '^\[dependencies\]|^vb_storage' crates/vb_core/Cargo.toml || echo 'PASS: no vb_storage deps'
```

The grep-on-cargo-output approach is explicitly rejected.

### F003: Missing schema_version in Lane Decisions

**Finding**: `E_SCHEMA_VERSION_MISSING` — All rows in `verifier-lane-decisions.jsonl` missing `schema_version: 'verifier-lane-decision/v1'`.

**Required Fix**: Add `schema_version: 'verifier-lane-decision/v1'` to every JSON object in `verifier-lane-decisions.jsonl`.

### F004: Missing schema_version in Obligations

**Finding**: `E_SCHEMA_VERSION_MISSING` — All rows in `proof-obligations.planned.jsonl` missing `schema_version: 'proof-obligation/v1'`.

**Required Fix**: Add `schema_version: 'proof-obligation/v1'` to every JSON object in `proof-obligations.planned.jsonl`.

### F005: Missing Required Obligation Fields

**Finding**: `E_SCHEMA_MISSING_FIELD` — `proof-obligations.planned.jsonl` missing `workdir`, `trusted_base_refs`, and `tool_metadata`.

**Required Fix**: Add to each obligation:
- `workdir`: `/home/lewis/src/velvet-ballistics`
- `trusted_base_refs`: reference to `trusted-base-plan.md` entries
- `tool_metadata`: object with at minimum `version` key (e.g., `{"version": "cargo 1.x.x"}`)

---

## Minimal Rerun State

After fixes, re-run proof-plan-reviewer from state 6 with:
1. Updated `verifier-lane-decisions.jsonl` (F001, F002, F003 fixed)
2. Updated `proof-obligations.planned.jsonl` (F004, F005 fixed)
3. Updated `waiver-candidates.jsonl` (F001 fix — W001 withdrawn or updated)
4. Updated `verifier-lane-review.jsonl` (2 rows disposition changed from rejected to accepted)

## Accepted Items (No Action Required)

- VLD-001 through VLD-007, VLD-009 through VLD-017, VLD-018, VLD-020 through VLD-030, VLD-032 through VLD-037: All lane decisions accepted
- Non-applicability decisions for Miri, Loom, Cargo-fuzz: All accepted
- Trusted base plan: Adequate
- Waiver candidates for Miri, Loom, Cargo-fuzz not-applicable: All accepted
- Key collision defense-in-depth: Adequately reasoned
