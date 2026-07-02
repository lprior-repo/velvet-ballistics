# Test Plan Review: vb-fb52

## STATUS: APPROVED WITH NOTES

---

## 1. Acceptance Criteria Coverage

| Contract §2 Precondition | Test Coverage | Status |
|---|---|---|
| P1: Empty batch construction | U1, HP-11 | ✓ |
| P2: `put_workflow_source` digest verify | I1, I13, U18 | ✓ |
| P3: `put_blob` digest verify | I2, I14, I15 | ✓ |
| P4: `put_compiled_ir` | I3 | ✓ |
| P5: `put_run_header` | I4 | ✓ |
| P6: `put_snapshot` | I5 | ✓ |
| P7: `append_event` | I6 | ✓ |
| P8: `put_*_index` operations | I11, HP-10 | ✓ |
| P9: `commit()` with staged ops | I7, I8 | ✓ |
| P10: `commit()` on empty batch | I9, I10, HP-8, HP-9 | ✓ |

**Error path coverage:**

| Contract §2 Error | Test Coverage | Status |
|---|---|---|
| A1: workflow_source digest mismatch | I13, EP-1 | ✓ |
| A2: blob digest mismatch | I14, EP-2 | ✓ |
| A3: encode failure | I27, I28, I29 | ✓ |
| A4: Fjall commit failure | P5 (all-or-nothing) | ✓ |
| A5: strict() + commit() | I12, P6 | ✓ |

---

## 2. Invariant Coverage

| Invariant | Tests | Status |
|---|---|---|
| I1: `!Sync + !Send` | U4 | ✓ |
| I2: `len()==0` iff `is_empty()` | U1, U3, P2 | ✓ |
| I3: `len()>0` after put | U2, P1, P3 | ✓ |
| I4: Batch consumed after commit | P4 | ✓ |
| I5: 60-byte header | U5, U6, U7, P9 | ✓ |
| I6–I11: Magic values | U8–U13 | ✓ |
| I12: 33-byte digest keys | U14 | ✓ |
| I13: 17-byte run_event keys | U15 | ✓ |
| I14: 9-byte run_header keys | U16 | ✓ |
| I15: 17-byte run_snapshot keys | U17 | ✓ |
| I16: All-or-nothing commit | P5 | ✓ |
| I17: No partial state | I16, I17 | ✓ |
| I18: Strict durability | I12, P6 | ✓ |
| I19: Digest verification mandatory | P7, P8 | ✓ |
| I20: Duplicate detection | I19 | ✓ |
| I21: Monotonic sequences | I20, I21 | ✓ |

---

## 3. BDD Scenario Alignment

| Scenario | Contract Ref | Test | Status |
|---|---|---|---|
| S1: Atomic batch commit | HP-1, HP-7 | I7, I8 | ✓ |
| S2: Empty batch commit | HP-8, HP-9 | I9, I10 | ✓ |
| S3: workflow_source digest mismatch | EP-1 | I13 | ✓ |
| S4: blob digest mismatch | EP-2 | I14 | ✓ |
| S5: Duplicate event rejection | EP-7, I20 | I19 | ✓ |
| S6: Sequence gap | EP-8, I21 | I20 | ✓ |
| S7: Process lock | EP-5, EP-6 | I16, I17 | ✓ |
| S8: Strict durability | HP-6, I18 | I12, P6 | ✓ |
| S9: 60-byte header | I5 | U5–U7, P9 | ✓ |
| S10: CRC detection | EP-17 | I26 | ✓ |
| S11: Future schema version | EP-12 | I25 | ✓ |
| S12: Monotonic sequence | I21 | I21 | ✓ |
| S13: Payload boundary | EP-14 | I27 | ✓ |
| S14: All-keyspace failure | I16 | P5 | ✓ |

---

## 4. Issues

### MINOR: P10 references non-existent invariant I70

`P10` claims to verify `I6–I11, I70`. The contract defines invariants only up to **I21**. Invariant `I70` does not exist in this contract.

**Impact:** Low — all actual invariants I6–I11 are covered by dedicated tests U8–U13.

**Recommendation:** Correct `P10` to read `I6–I11` only, or confirm whether `I70` was intended as a placeholder for a future invariant.

### MINOR: `put_compiled_ir` digest not tested

Contract precondition P4 does **not** require digest verification for `put_compiled_ir` (that requirement is specific to `workflow_source` and `blob` per I19). This is correct. No action needed.

### NOTE: `KeyCapacity` error (0x4003) not explicitly tested

The error taxonomy defines `KeyCapacity` (0x4003) for key encoding failures, but no test explicitly triggers this. However, the key layout invariants (I12–I15) are tested via unit tests U14–U17, and key encoding is deterministic from the key components. Acceptable.

---

## 5. Test Distribution Summary

| Layer | Count | Contract Coverage |
|---|---|---|
| Unit | 18 | Encoding invariants, structural invariants, key layout |
| Integration | 29 | Happy path commits, error paths, process lock, durability |
| Property-based | 10 | Invariant checking across randomized inputs |
| **Total** | **57** | |

*(Note: count exceeds test-plan.md header (52) due to actual test entries)*

---

*Review generated for vb-fb52*
