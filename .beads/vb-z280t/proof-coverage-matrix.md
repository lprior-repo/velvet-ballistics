# Proof Coverage Matrix — vb-z280t

| requirement_id | contract_clause | proof_seed | verus | kani | flux | loom | proptest | fuzz | coverage_status |
|---|---|---|---|---|---|---|---|---|---|
| REQ-RESOURCE-LOOP-MUL | vb-z280t/loop_mul | vb-z280t | YES | YES | N/A | N/A | YES | N/A | 3-of-3 applicable |
| REQ-RESOURCE-SAT-MUL-U64 | vb-z280t/sat_mul_u64 | vb-z280t | YES | YES | N/A | N/A | YES | N/A | 3-of-3 applicable |
| REQ-RESOURCE-LOOP-MUL-BOUNDARIES | vb-z280t/boundaries | vb-z280t | — | YES | N/A | N/A | — | N/A | 1-of-1 applicable |
| REQ-RESOURCE-LOOP-MUL-ZERO | vb-z280t/zero | vb-z280t | — | YES | N/A | N/A | YES | N/A | 2-of-2 applicable |
| REQ-RESOURCE-LOOP-MUL-ONE | vb-z280t/one | vb-z280t | — | YES | N/A | N/A | YES | N/A | 2-of-2 applicable |
| REQ-RESOURCE-LOOP-MUL-UNDER-BOUND | vb-z280t/under_bound | vb-z280t | — | YES | N/A | N/A | YES | N/A | 2-of-2 applicable |
| REQ-RESOURCE-LOOP-MUL-OVERFLOW | vb-z280t/overflow | vb-z280t | — | YES | N/A | N/A | YES | N/A | 2-of-2 applicable |

**Summary:**
- Required lanes: Verus ×2 obligations + Kani ×5 harnesses + proptest ×5 properties.
- Non-applicable: Flux (non-linear spec), Loom (sync), fuzz (no parser), TLA+ (no temporal).
- **Master §64 (Resource Budget Arithmetic):** binding now proves spec ↔ production saturating semantics.