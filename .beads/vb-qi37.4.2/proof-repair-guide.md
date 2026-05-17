<<<<<<< HEAD
# Proof Repair Guide: vb-qi37.4.2

## STATUS: REJECTED

## Owner State and Rerun Targets

| Finding | Owner State | Rerun From | Next Agent |
|---|---|---|---|
| PR-006 (missing verified ledger) | 6 | 6 | proof-writer (State 5/8) |
| PR-007 (15 unexecuted Kani obligations) | 3, 6 | 3, 8 | proof-writer (State 5/8) |
| PR-008 (static-scan obligations) | 11 | 11 | proof-writer (State 5/11) |
| PR-009 (gauntlet obligations) | 12 | 12 | proof-writer (State 5/12) |
| PR-010 (stale evidence) | 5 | 5 | proof-writer (State 5/8) |

## Required Fixes

### PR-006: Produce Verified JSONL Ledger

**Command to execute:**
```bash
# Regenerate proof-obligations.planned.jsonl with updated status field as proof-obligations.verified.jsonl
# Each row should have status updated to PASS, WAIVED, or BLOCKED
# Include evidence field with path to raw command output
# Include owner_state and rerun_from fields

# Example (pseudo-code):
jq -s '
  map(if .id == "VB-CORE-TAINT-001" then
    . + {status: "PASS", evidence: "verus-report.md", verified_at: "2026-05-15"}
  elif .id == "VB-CORE-STATE-001-KANI" then
    . + {status: "PASS", evidence: "kani-report.md", verified_at: "2026-05-15"}
  # ... similar for all 59 rows
  else . end)
' .beads/vb-qi37.4.2/proof-obligations.planned.jsonl > .beads/vb-qi37.4.2/proof-obligations.verified.jsonl
```

**Expected output:** `proof-obligations.verified.jsonl` with all 59 rows having status PASS, WAIVED, or BLOCKED (not "planned")

### PR-007: Execute 15 Unexecuted Kani Obligations

**For each obligation, either create harness and run, or provide waiver:**

| Obligation ID | Harness Needed | Command |
|---|---|---|
| VB-CORE-TAINT-006-KANI | kani_taint_propagation | `cargo kani -p vb_core --harness kani_taint_propagation` |
| VB-CORE-BUDGET-001 | kani_step_budget_zero | `cargo kani -p vb_core --harness kani_step_budget_zero` |
| VB-CORE-BUDGET-002 | kani_step_budget_one | `cargo kani -p vb_core --harness kani_step_budget_one` |
| VB-CORE-BUDGET-003-KANI | kani_step_budget | `cargo kani -p vb_core --harness kani_step_budget` |
| VB-CORE-IDX-001 | kani_index_access | `cargo kani -p vb_core --harness kani_index_access` |
| VB-CORE-RESOURCE-004 | kani_resource_budget_bounded | `cargo kani -p vb_core --harness kani_resource_budget_bounded` |
| VB-IPC-DECODE-001 | kani_ipc_header | `cargo kani -p vb_ipc --harness kani_ipc_header` |
| VB-IPC-DECODE-002 | kani_ipc_header_rejects_oversize | `cargo kani -p vb_ipc --harness kani_ipc_header_rejects_oversize` |
| VB-IPC-DECODE-003 | kani_ipc_header | `cargo kani -p vb_ipc --harness kani_ipc_header` |
| VB-STORAGE-DECODE-001 | kani_record_magic | `cargo kani -p vb_storage --harness kani_record_magic` |
| VB-STORAGE-DECODE-002 | kani_record_schema | `cargo kani -p vb_storage --harness kani_record_schema` |
| VB-STORAGE-DECODE-003 | kani_record_kind | `cargo kani -p vb_storage --harness kani_record_kind` |
| VB-STORAGE-DECODE-004 | kani_record_payload_len | `cargo kani -p vb_storage --harness kani_record_payload_len` |
| VB-STORAGE-DECODE-005 | kani_record_crc | `cargo kani -p vb_storage --harness kani_record_crc` |
| VB-EXPR-002 | kani_expr_stack | `cargo kani -p vb_expr --harness kani_expr_stack` |

**If waiver needed:** Provide waiver with:
- `waiver: true`
- `reason: <why cannot execute>`
- `compensating_evidence: <what else covers this>`
- `owner_state: 3` (for proof-lane deferral) or `owner_state: 8` (for harness-missing deferral)
- `expiry: <when this waiver expires>`
- `follow_up: <what resolves this waiver>`

### PR-008: Execute Static-Scan Obligations

```bash
# VB-CORE-IDX-002
cargo xtask forbidden-scan --pattern as_usize_index --crate vb_core

# SRC-LINT-001 (no unsafe code)
cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings 2>&1 | grep -c 'unsafe' || echo "0"

# SRC-LINT-002 (no panic)
cargo clippy --workspace --lib -- -D warnings 2>&1 | grep -i panic || echo "0 panic found"
```

Or defer to owner_state=11 with explicit waiver.

### PR-009: Gauntlet Obligations

These cannot execute until all prior obligations complete. Defer to owner_state=12.

### PR-010: Update Stale Evidence

Update `.beads/vb-qi37.4.2/proof-evidence.md` to reflect that lines 113-121 are stale from attempt 4, and add evidence entries for the successful repair runs from attempt 5.

## Verification

After repairs, run:

```bash
# Verify JSONL ledger
jq -s '[.[].status] | group_by(.) | map({status: .[0], count: length})' .beads/vb-qi37.4.2/proof-obligations.verified.jsonl

# Should return: no "planned" status remaining
# Should have: PASS, WAIVED, or BLOCKED with counts
```

## Handoff to Next Agent

State 6 is BLOCKED. Nearest available owner for remaining work:
- State 5/8 for Kani harness creation and proof-evidence updates
- State 11 for static-scan obligations
- State 12 for gauntlet obligations (deferred)

The repairs from attempt 5 successfully resolved three of five prior findings (PR-002, PR-003, and partial PR-001). The remaining blockers (PR-006, PR-007, PR-008, PR-009) require targeted completion before State 6 approval can be granted.
=======
# Proof Repair Guide - vb-qi37.4.2

STATUS: REPAIR_REQUIRED

## Required Repairs

1. Update `.beads/vb-qi37.4.2/proof-obligations.jsonl` so `VERUS-ENV-006` names `verification/verus/accepted_envelope_model.rs`, uses checker `verus`, records command `verus verification/verus/accepted_envelope_model.rs`, and points at an existing evidence artifact.
2. Align all executed TLA+/Verus obligation evidence paths with actual files. Either create `.beads/vb-qi37.4.2/tla-report.md` and `.beads/vb-qi37.4.2/verus-report.md`, or update evidence fields to `.beads/vb-qi37.4.2/proof-evidence.md` with section references.
3. Execute or explicitly waive every required planned lane: `PO-007`, `PO-008`, `PO-009`, `PO-010`, `PO-011`, and `PO-012`.
4. For any lane deferred by lifecycle state, make the deferral explicit in the current review target: owner, expiry condition, and compensating evidence. Do not leave required rows as plain `planned` and then request proof approval.
5. Re-run the proof-review verification set after the ledger and evidence repairs.

## Minimum Rerun Targets

```bash
python -c 'import json, pathlib; [json.loads(line) for path in [".beads/vb-qi37.4.2/proof-obligations.jsonl", ".beads/vb-qi37.4.2/proof-obligations.planned.jsonl", ".beads/vb-qi37.4.2/traceability-matrix.jsonl"] for line in pathlib.Path(path).read_text().splitlines() if line.strip()]'
TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp tlc -metadir .beads/vb-qi37.4.2/review-tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla
verus verification/verus/capability_artifact_model.rs
verus verification/verus/accepted_envelope_model.rs
```

Add the Kani, fuzz, proptest, static scan, mutation, and CI commands once their artifacts are present or waivers are formally recorded.
>>>>>>> origin/go-skill-p0-vb-qi37-4-2
