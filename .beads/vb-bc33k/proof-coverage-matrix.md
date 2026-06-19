# Proof Coverage Matrix — vb-bc33k

| requirement_id | contract_clause | proof_seed | verus | kani | flux | loom | proptest | fuzz | tla | coverage_status |
|---|---|---|---|---|---|---|---|---|---|---|
| REQ-TYPE-ENFORCER-BOOL | vb-bc33k/expect_bool | vb-bc33k | YES | YES | N/A | N/A | YES | N/A | N/A | 3-of-3 applicable |
| REQ-TYPE-ENFORCER-I64  | vb-bc33k/expect_i64  | vb-bc33k | YES | YES | N/A | N/A | YES | N/A | N/A | 3-of-3 applicable |
| REQ-TYPE-ENFORCER-SYMBOL | vb-bc33k/expect_symbol | vb-bc33k | YES | YES | N/A | N/A | YES | N/A | N/A | 3-of-3 applicable |
| REQ-TYPE-ENFORCER-LIST | vb-bc33k/expect_list | vb-bc33k | YES | YES | N/A | N/A | YES | N/A | N/A | 3-of-3 applicable |
| REQ-TYPE-ENFORCER-OBJECT | vb-bc33k/expect_object | vb-bc33k | YES | YES | N/A | N/A | YES | N/A | N/A | 3-of-3 applicable |
| REQ-SLOTVALUE-PARTITION | vb-bc33k/slot_value_partition | vb-bc33k | YES | YES | N/A | N/A | YES | N/A | N/A | 3-of-3 applicable |

**Summary:**
- 6 requirements × 7 verifier lanes = 42 cells
- Required lanes covered: 18 (Verus×6, Kani×6, proptest×6 — bundled per requirement group)
- Non-applicable lanes justified: Flux×6, Loom×6, fuzz×6, TLA×6 = 24 with concrete evidence
- **Master §40 + §44 satisfied:** every Rust-behavior lane applicable to type enforcers has a planned obligation; no behavior is silently dropped.