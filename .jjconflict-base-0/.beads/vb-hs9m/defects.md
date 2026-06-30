# Defects Log — vb-hs9m

## Current Status: DEFECTS FIXED IN CANONICAL SOURCE CHECKOUT

Canonical source: `/home/lewis/src/velvet-ballistics`
Workdir (stale): `/home/lewis/src/vb-hs9m-workspace` — out of sync, requires re-sync.

---

## 🔴 CRITICAL DEFECTS

### DEFECT-8: Waivers Cite Non-Executable Kani as Compensating Evidence
- **File:** `proof-obligations.planned.jsonl`
- **Problem:** TLA+/Lean/Concert waivers cite Kani proofs (OBL-TRC-001 through OBL-TRC-004) that cannot execute (CBMC targets missing)
- **Impact:** Formal verification case is hollow
- **Status:** PRE-EXISTING formal verification debt — tracked separately
- **Fix Required:** Reword waivers to remove false compensating evidence claims

---

## 🔴 HIGH PRIORITY DEFECTS — RESOLVED

### ✅ DEFECT-1: YAML Format Uses JSON Serializer — FIXED
- **Canonical File:** `/home/lewis/src/velvet-ballistics/xtask/src/evidence/bundle.rs:287`
- **Fixed Code:** `serde_yaml::to_string(bundle)`
- **Workdir (stale):** `bundle.rs:287` still shows `serde_json::to_string_pretty` (not synced)
- **Verification:** POST-008 contract satisfied

### ✅ DEFECT-2: `TraceRing::new(0)` Panics — FIXED
- **Canonical File:** `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/trace.rs:28`
- **Fixed Code:** `RingBuffer::new(capacity.max(1))`
- **Workdir (stale):** `trace.rs:24` still shows `RingBuffer::new(capacity)` (not synced)
- **Verification:** PRE-001 contract satisfied; doc comment explicitly notes normalization

### ✅ DEFECT-3: Path Existence Not Validated — FIXED (stale wc)
- **File:** `crates/workspace_tests/src/acceptance_catalog.rs:454-467`
- **Problem:** Was a workdir stale-copy issue only
- **Status:** Resolved in canonical source

---

## 🟡 MEDIUM PRIORITY DEFECTS — PRE-EXISTING (Non-Blocking)

### DEFECT-4: `snapshot_for_run` Bound Uses `capacity` — NOT A DEFECT
- **Canonical File:** `trace.rs:131-133`
- **Analysis:** Bound is correct; using `capacity` is safe since `history.len() <= capacity` always holds
- **Status:** Finding withdrawn

### DEFECT-5: `parse_bundle_schema_version` Exceeds 25 Lines
- **File:** `bundle.rs:154-212` (58 lines)
- **Status:** PRE-EXISTING — Farley debt, not blocking

### DEFECT-6: Unwrapped Primitives in EvidenceBundle
- **File:** `bundle.rs:18-36`
- **Status:** PRE-EXISTING — validation is runtime, type-level enforcement deferred

### DEFECT-7: Unwrap in Error Path
- **File:** `bundle.rs:277`
- **Status:** PRE-EXISTING — `unwrap_or("")` is safe for error context

---

## 🟢 MINOR DEFECTS

### DEFECT-9: Verbose Character Iteration
- **File:** `bundle.rs:132-146`
- **Status:** MINOR — style only, not blocking

---

## Fix History

| Attempt | Date | Claimed Fixes | Verified |
|---------|------|---------------|----------|
| State 10 | - | DEFECT-1 + DEFECT-2 | ❌ NOT in workdir (stale) |
| State 12 (source checkout) | 2026-05-19 | DEFECT-1 + DEFECT-2 | ✅ FIXED in canonical source |
| Workdir re-check | 2026-05-19 | - | ❌ Workdir out of sync |

---

## Workdir Sync Required

The workdir `/home/lewis/src/vb-hs9m-workspace` is out of sync with canonical source `/home/lewis/src/velvet-ballistics`. The following files need re-syncing:

- `crates/vb_runtime/src/trace.rs`
- `xtask/src/evidence/bundle.rs`

---

**END OF DEFECTS LOG**
