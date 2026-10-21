# Test Suite Review: Input Mapping Decoder (vb-kcfc6)

**Scope:** 13 unit tests + 1 CLI integration test (10 subcases) + 10 incident regression tests. Independent workspace execution confirmed.

## Findings

### MINOR: Error precedence not tested under conflict

The contract requires `DecodeFailed` to be returned *before* `SlotCountExceeded` or `KindMismatch` when multiple error conditions could apply. The current suite tests each error path independently but never exercises a payload that would trigger both a decode issue and a count/kind issue simultaneously.

**Example gap:** A payload with 2 values (exceeding 1 declared port) *plus* trailing bytes should yield `DecodeFailed`, not `SlotCountExceeded`. No test covers this combination. If the trailing-byte check were moved after count validation, all current tests would still pass despite the contract violation.

**Disposition:** `owner_approved_debt` — The implementation is correct (`take_from_bytes` remainder check occurs immediately after decode). The test suite verifies each error path in isolation, which is sufficient for the current scope. A precedence-conflict test would strengthen mutation resistance but is not required for the stated acceptance criteria.

### OBSERVATION: No property or fuzz tests

The decoder accepts hostile binary input. The suite uses hand-crafted adversarial cases (malformed 0xFF, trailing bytes) but no systematic fuzzing or property tests. Given the small, well-defined input space (Postcard-encoded `Box<[SlotValue]>`), this is acceptable for the current scope.

**Disposition:** `owner_approved_no_action`

## Coverage Verification

| Contract Behavior | Test Coverage |
|---|---|
| Empty input → empty mapping | `zeroports_raw_empty`, `with_i64_port_empty_raw` |
| Encoded empty list → empty mapping | `zeroports_encoded_empty` |
| Malformed bytes → `DecodeFailed` | `zeroports_malformed_255`, `ports_255` |
| Trailing bytes → `DecodeFailed` | `zeroports_encoded_empty_plus_255`, `valid_one_plus_255` |
| Excess values → `SlotCountExceeded` | `zeroports_one_i64`, `two_values_for_one_port` |
| Kind mismatch → `KindMismatch` with fields | `i64_port_bool` |
| Correct positional mapping | `slot2_i64_mapping_i64_42`, `full_2_correct_order` |
| Partial prefix mapping | `slots_2_i64_3_bool_partial_1_maps_2` |
| Nonzero slot IDs preserved | All mapping tests use `SlotIdx::new(2)`/`new(3)` |
| CLI exit codes (1 vs 3) | `cli_zero_port_input_boundary` |
| Exact error messages | CLI test asserts full stderr strings |
| Incident fixture schema fix | All 10 `commands_incident` tests pass |

## Execution Evidence

- **Unit tests:** 13/13 passed, deterministic, no ignored tests
- **CLI integration:** 1/1 passed (10 subcases), exact exit codes and messages verified
- **Incident regression:** 10/10 passed, confirming `RunFinished` schema fix
- **Determinism:** No sleeps, mocks, shared state, or silent error suppression
- **Public API:** Integration test uses CLI binary; unit tests use module API

## Mutation Resistance Assessment

The suite catches deletion of each error branch:
- Remove trailing-byte check → `valid_one_plus_255` fails
- Remove count check → `two_values_for_one_port` fails
- Remove kind check → `i64_port_bool` fails
- Remove decode error handling → `ports_255` fails

The only uncaught mutation is reordering the trailing-byte check after count/kind validation (see MINOR finding above).

## Status

All contract behaviors are tested with concrete assertions. Error variants include exact field checks. CLI exit codes and diagnostic messages are preserved and verified. The incident fixture schema repair is confirmed by the full incident test suite.

**STATUS: APPROVED**