# Hazard Analysis: `together` Primitive Acceptance

## Hazard Categories

### 1. Parser Boundary Hazards

| Hazard | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| Unknown field rejection | `"together"` key rejected as unknown because `is_primitive()` lacks it | HIGH | Add `"together"` to `is_primitive()` |
| Backward compatibility gap | Existing YAML using `"parallel"` breaks if replaced without alias | MEDIUM | Accept `"parallel"` as temporary alias |
| Empty branches bypass | `together: {}` with no `branches` field bypasses parse guard | HIGH | Enforce `branches` field presence in `parse_parallel()` |
| Case-sensitive mismatch | `Together:` (capitalized) not handled | LOW | YAML keys are case-sensitive by spec; not a bug |

### 2. Validation Boundary Hazards

| Hazard | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| STEP_PRIMITIVES drift | `STEP_PRIMITIVES` arrays in 3 locations become inconsistent | MEDIUM | Update all 3 atomically |
| Unknown field after parse fix | After adding `"together"` to parse, validate still rejects if STEP_PRIMITIVES not updated | HIGH | Update all 3 STEP_PRIMITIVES arrays |

### 3. Compile Boundary Hazards

| Hazard | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| Display name mismatch | `Together { .. } => "parallel"` in compile/mod.rs returns wrong string | MEDIUM | Update to `"together"` |
| from_field asymmetry | `from_field("together")` returns `None` after parse fix | HIGH | Add mapping to `from_field()` |
| Budget overflow | No limit on `branches.len()` before hitting `max_together_branches` | MEDIUM | Budget check in `lower_together()` |

### 4. Concurrency Hazards

| Hazard | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| TOGETHER_BRANCH_LIMIT_EXCEEDED | Unbounded fan-out causes resource exhaustion | MEDIUM | Enforce budget in compile lowering |
| Non-terminating join | All branches must complete; no timeout at join | LOW | Runtime handles via budget/timeout |

### 5. Temporal Hazards

| Hazard | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| Regression: parallel rejected | Fix adds "together" but removes "parallel" recognition | HIGH | Ensure "parallel" is accepted as alias during transition |
| Schema drift | Language spec says "together" but code says "parallel" | MEDIUM | Align spec and code |

### 6. Unsafe/Provenance Hazards

| Hazard | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| None identified | together primitive is pure data transformation | N/A | — |

### 7. Hostile Input Hazards

| Hazard | Description | Severity | Mitigation |
|--------|-------------|----------|------------|
| Deeply nested branches | `branches` containing `Together` with nested `branches` | LOW | Stack depth bounded by workflow size |
| Huge branch count | `branches.len() == u16::MAX` | MEDIUM | Budget check gates at compile time |

## Risks That Remain Representable After Fix

1. **Empty branch list**: Currently representable as `Together { branches: [] }` — fix must add validation to reject this at compile time
2. **Duplicate branch labels**: Not validated at parse time — representable in IR
3. **Zero-length branch steps**: Not validated at parse time — representable in IR

## Risks Mitigated by Fix

1. ✅ `"together"` YAML key causes `UnknownField` error — fixed by adding to `is_primitive()`
2. ✅ `STEP_PRIMITIVES` arrays missing `"together"` — fixed by updating all 3 arrays
3. ✅ Display string `"parallel"` instead of `"together"` — fixed by updating match arms
