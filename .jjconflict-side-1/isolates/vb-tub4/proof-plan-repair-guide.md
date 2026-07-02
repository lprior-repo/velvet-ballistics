# Proof Plan Repair Guide: vb-tub4

## Required Fixes for State 4 → State 4 Re-run

Apply all fixes to `proof-obligations.planned.jsonl` before proof-writer can execute obligations.

---

## Fix 1: Add `schema_version` to Every Obligation Entry

**Current State**: All 27 JSON objects in `proof-obligations.planned.jsonl` lack `schema_version`.

**Required State**: Each obligation object must have `"schema_version": "proof-obligation/v1"` as a top-level field.

**Example Transformation**:

Before:
```json
{"id":"vb-tub4-obl-001","seed_id":"vb-tub4-seed-001","target":"crates/vb_core/src/budget.rs:1639",...}
```

After:
```json
{"schema_version":"proof-obligation/v1","id":"vb-tub4-obl-001","seed_id":"vb-tub4-seed-001","target":"crates/vb_core/src/budget.rs:1639",...}
```

**Applies to**: All 27 obligations (vb-tub4-obl-001 through vb-tub4-obl-027).

---

## Fix 2: Add `workdir` to Every Obligation Entry

**Current State**: No `workdir` field in any obligation.

**Required State**: Each obligation must have `"workdir": "/home/lewis/src/velvet-ballistics"` (the workspace root).

**Example Transformation**:

Add to each obligation:
```json
"workdir": "/home/lewis/src/velvet-ballistics"
```

**Applies to**: All 27 obligations.

---

## Fix 3: Rename `bound` Field to `model_bounds`

**Current State**: Obligations use `"bound"` as the field name for symbolic bounds.

**Required State**: Per `proof-obligation/v1` schema, the field must be named `"model_bounds"`.

**Example Transformation**:

Before:
```json
"bound": "kani::any::<u64>() x kani::any::<u64>() with assume(a <= u64::MAX/2 || b <= u64::MAX/2)"
```

After:
```json
"model_bounds": "kani::any::<u64>() x kani::any::<u64>() with assume(a <= u64::MAX/2 || b <= u64::MAX/2)"
```

**Applies to**: All 27 obligations.

---

## Fix 4 (Advisory): Rename `owner_state` to `owner`

**Current State**: Obligations use `"owner_state": 5`.

**Required State**: Schema uses `"owner"` not `"owner_state"`.

**Example Transformation**:
```json
"owner": "proof-writer"
```

**Note**: This is advisory. The field name `owner_state` conveys state ownership but is not schema-compliant. Rename to `owner`.

---

## Fix 5 (Advisory): Add `artifact` Field

**Current State**: No `artifact` field specifying the verification harness file.

**Required State**: Each obligation should have an `artifact` field pointing to the harness source file.

**Example**:
```json
"artifact": "crates/vb_core/src/budget.rs"
```

---

## Fix 6 (Advisory): Add `required` and `behavior_affecting` Fields

**Current State**: These boolean fields are absent.

**Required State**: Add to each obligation:
```json
"required": true,
"behavior_affecting": false
```

All vb-tub4 obligations are required fixes for GOD RULE compliance and do not affect production behavior (they only change harness internals).

---

## Proof-Plan-Reviewer Gate Status

| Lane | Disposition |
|------|-------------|
| vb-tub4-seed-001 (kani) | accepted |
| vb-tub4-seed-002 (kani) | accepted |
| vb-tub4-seed-003 (kani) | accepted |
| vb-tub4-seed-004 (kani) | accepted |
| vb-tub4-seed-005 (kani) | accepted |
| vb-tub4-seed-006 (kani) | accepted |
| vb-tub4-seed-007 (kani, delete) | accepted |
| vb-tub4-seed-008 (kani, delete) | accepted |

**8/8 lane decisions accepted.** No replanning needed for lane decisions.

---

## After Fixes

Once all 6 fixes are applied to `proof-obligations.planned.jsonl`:

1. Re-run proof-plan-reviewer to validate schema compliance
2. If approved, advance to State 5 (proof-writer may execute obligations)

---

## What NOT to Change

- **Lane decisions**: All 8 kani lane decisions are approved. Do not alter `verifier-lane-decisions.jsonl`.
- **Waiver candidates**: No waivers needed. Do not modify `waiver-candidates.jsonl`.
- **Trusted base plan**: Scoping is correct. Do not modify `trusted-base-plan.md`.
- **Proof coverage matrix**: Coverage is complete across 27+2 obligations.
- **Traceability matrix**: All 8 seeds mapped. Minor gap (no per-obligation mapping) is non-blocking.