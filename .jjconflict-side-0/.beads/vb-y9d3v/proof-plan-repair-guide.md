# Proof Plan Repair Guide — vb-y9d3v

## Rerun State

- **State**: 4 (proof-planner sublane)
- **Repair agent**: proof-planner (re-run after fixes)
- **Planner invocation ID to use**: `vb-y9d3v-state4-proof-planner-attempt2`

## Blocking Repairs (Required Before Re-review)

### 1. Fix Seed 011 Obligation Targets (F-vb-y9d3v-0001)

**Files to modify**: `proof-obligations.planned.jsonl`

**Option A (Recommended): Rewrite obligations to target codec-specific verification.**

Modify obligations PO-vb-y9d3v-0042 through PO-vb-y9d3v-0045 as follows:

- **PO-0042 (Flux-rs)**: Change target from `#[sig] on validate_ticket_attempt` to target retry counter encode/decode functions. Suggested artifact: `crates/vb_runtime/src/verification/flux/vb_y9d3v_retry_codec_refinements.rs`, target functions that read/write retry counter bytes. Domain claim: "Flux refinements guarantee retry counter values stay within u16 bounds after decode and reject invalid byte sequences."

- **PO-0043 (Kani)**: Change target from `kani_attempt_fence_harnesses::check_attempt_fence` to target retry counter encode/decode or the function that reads retry count from metadata. Target function that does `u16::try_from(retry_value)` or equivalent checked conversion. Suggested target: `kani_retry_codec_harnesses::check_retry_decode_bounds`.

- **PO-0044 (Verus)**: Change target from `proof fn action_fence_correct` to target encode/decode round-trip. Suggested target: `proof fn retry_codec_roundtrip` proving `decode(encode(x)) == Ok(x)` for all valid `x`.

- **PO-0045 (proptest)**: Change target from `proptest_attempt_fence::prop_attempt_freshness` to target retry counter codec API. Suggested target: `proptest_retry_codec::prop_retry_encode_decode_roundtrip` with arbitrary u16 inputs.

**Option B (Also acceptable)**: Reclassify VLD-vb-y9d3v-0081 through VLD-vb-y9d3v-0084 as `not_applicable` for seed 011, with decision_reason citing that the retry counter is a trivially invertible u16 codec and cargo-fuzz provides sufficient coverage for the codec boundary. Update the not_applicable evidence refs accordingly. This reduces required lanes from 45 to 41 (removing 4 Rust-default lanes for seed 011).

### 2. Fix Proof-Strategy.md Obligation Counts (F-vb-y9d3v-0003)

**File to modify**: `proof-strategy.md`

Update the Obligation Grouping section:

```markdown
### Kani (11 obligations)
- PO-001 through PO-040 grouped by target: check_attempt_fence (11), fuzz scaffold (1)
- PO-043: Retry codec bounds

### Verus (11 obligations)
- PO-001 through PO-040 grouped by target: action_fence_correct (11)
- PO-044: Retry codec round-trip

### Flux (11 obligations)
- PO-001 through PO-040 grouped by target: validate_ticket_attempt (11)
- PO-042: Retry codec refinements

### proptest (11 obligations)
- PO-001 through PO-040 grouped by target: prop_attempt_freshness (11)
- PO-045: Retry codec properties

### cargo-fuzz (1 obligation)
- PO-041: Retry counter codec fuzz target
```

Remove the TLA+ obligation group entirely (PO-028 does not exist).

## Non-Blocking Repairs (Fix Before State 5 Dispatch)

### 3. Fix VLD-0096 owner_state (F-vb-y9d3v-0002)

**File to modify**: `verifier-lane-decisions.jsonl`, line 96

Change `"owner_state": 5` to `"owner_state": 4` to match the not_applicable convention.

### 4. Fix Verifier-Lane-Matrix.md TLA+ Column (F-vb-y9d3v-0004)

**File to modify**: `verifier-lane-matrix.md`

Line 20: Change TLA_PLUS_REMOVED+ for seed 012 from `required` to `not_applicable`.
Line 33: Change TLA_PLUS_REMOVED+ required count from `012 (1)` to `0` and not_applicable from `001-011 (11)` to `001-012 (12)`.

### 5. Strengthen Seed 012 Evidence Refs (F-vb-y9d3v-0005)

**File to modify**: `verifier-lane-decisions.jsonl`, lines 89-95

Update `non_applicability_evidence_refs` from `["proof-seeds.jsonl:1-11"]` to:
```json
["proof-seeds.jsonl:seed-012-notes", "verifier-lane-matrix.md:Default Lane Not-Applicable Evidence for Seed 012"]
```

## Minimal Rerun Scope

After repairs, the proof-planner must:
1. Rerun with invocation `vb-y9d3v-state4-proof-planner-attempt2`
2. Update only `proof-obligations.planned.jsonl`, `proof-strategy.md`, `verifier-lane-decisions.jsonl` (lines 89-96), and `verifier-lane-matrix.md`
3. Regenerate `proof-plan-review.md` and `verifier-lane-review.jsonl` as placeholders
4. Append sequence 6 row to agent-invocation-ledger

The proof-plan-reviewer will then re-review only the changed artifacts.
