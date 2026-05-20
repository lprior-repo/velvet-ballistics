# Proof Repair Guide: vb-te1i — Binary IPC BDD Acceptance

## Repairs Required for Approval

### 1. Add Formal Waivers for Blocked Required Obligations (CRITICAL)

**Affected obligations**: KAN-001, KAN-002, KAN-003, VERUS-001, VERUS-002, VERUS-003, VERUS-004

**Problem**: These obligations are `required: true` in `proof-obligations.planned.jsonl` but blocked by pre-existing workspace tooling issues. The `waiver` field is `null` instead of containing a formal waiver record.

**Fix**: Update `proof-obligations.planned.jsonl` to add formal waiver entries for each blocked required obligation. Example format for each entry:

```json
{
  "id": "KAN-001",
  "waiver": {
    "reason": "pre_existing_workspace_blocker",
    "detail": "vb_storage crate has broken Kani harnesses (missing kani::Arbitrary impls for RunId, EventSeq, CapabilitySet, RuntimePolicy; unresolved import recover_runtime_summary_from_events) that prevent compilation of entire workspace under Kani",
    "owner": "vb-te1i owner",
    "compensating_evidence": [
      "UNIT-002: decode_rejects_invalid_magic — behavioral test for POST-005",
      "BDD-003: ipc_rejects_bad_magic_before_payload_allocation — integration test for POST-005"
    ],
    "followup": "Repair vb_storage Kani harnesses to enable formal verification"
  }
}
```

Apply same pattern to KAN-002, KAN-003, and VERUS-001..004 with their specific blocking reasons:
- **KAN-002/003**: Same vb_storage compilation blocker
- **VERUS-001..004**: Workspace dependency resolution failure (serde, vb_core not resolvable by verus single-file invocation)

### 2. Fix JSONL Schema: Duplicate Field in VERUS-002

**Problem**: `proof-obligations.jsonl` line 5 has duplicate `contract_clause` field

**Fix**: Remove one instance of `contract_clause` from VERUS-002 entry

### 3. Fix Mapping Inconsistency for UNIT-001

**Problem**: `proof-obligations.planned.jsonl` maps UNIT-001 to POST-001, but `proof-obligations.jsonl` maps UNIT-001 to POST-002

**Fix**: Update `proof-obligations.planned.jsonl` line 8 to change `"contract_clause":"POST-001"` to `"contract_clause":"POST-002"` (matching the actual behavioral evidence in proof-evidence.md where `header_getter_returns_expected_value` maps to POST-002 and `decode_frame_succeeds_with_valid_header_and_payload` maps to POST-001)

### 4. Update Line Count in proof-evidence.md

**Problem**: proof-evidence.md claims 735 lines, actual is 727 lines

**Fix**: Update the line count reference in proof-evidence.md from 735 to 727

---

## Verification After Repairs

After applying repairs, run:

```bash
# Validate JSONL
jq -c . .beads/vb-te1i/proof-obligations.planned.jsonl >/dev/null && echo "JSONL valid"

# Re-run evidence commands (should still pass)
cargo test --package vb_ipc
cargo test --package velvet-ballastics-workspace-tests --test vb_te1i_binary_ipc_acceptance
cargo clippy --package vb_ipc --lib --bins --examples -- -D warnings
```

---

## Priority

1. **CRITICAL**: Add formal waivers for KAN-001, KAN-002, KAN-003 (required obligations without waiver)
2. **CRITICAL**: Add formal waivers for VERUS-001, VERUS-002, VERUS-003, VERUS-004 (required obligations without waiver)
3. **Minor**: Fix duplicate contract_clause in VERUS-002
4. **Minor**: Align UNIT-001 mapping
5. **Minor**: Update line count

Once items 1 and 2 are addressed, STATUS can escalate to APPROVED without re-running tests.
