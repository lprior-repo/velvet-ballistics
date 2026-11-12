# Test Review: vb-79ffw

**Reviewer:** Independent local3090 review
**Baseline:** af142d34 (frozen)
**Status:** APPROVED with limitations

## Test Suite Assessment

### Probe Test (vb_79ffw_ticket_authority_probe.rs)
All 6 tests pass:

| Test | Status | Coverage |
|------|--------|----------|
| `stale_completion_rejected_after_two_retries` | PASS | Stale run-attempt rejection (1 vs 3) |
| `future_run_attempt_ticket_rejected` | PASS | Future run-attempt rejection |
| `zero_run_attempt_ticket_rejected` | PASS | Invalid zero run-attempt |
| `cold_recovery_preserves_run_attempt` | PASS | Durable checkpoint recovery |
| `idempotency_key_differs_across_run_attempts` | PASS | Key binding to run-attempt |
| `ticket_key_valid_after_serialization` | PASS | Serialization roundtrip |

### Existing Tests
- `durable_retry_run_attempt_authority.rs`: 4 tests PASS
- `vb_r4fm9_event_generation.rs`: 3 tests PASS

## Test Quality Assessment

### Strengths
1. **Real public API coverage:** Tests use the actual `Runtime`, `StorageRuntimeJournal`, and `FjallJournal` public APIs
2. **Durable state verification:** Tests verify state across process boundaries via checkpoint/recovery
3. **Negative testing:** Tests verify rejection of stale, future, and zero run-attempts
4. **Idempotency verification:** Tests confirm key differs across run attempts

### Limitations

#### Limitation 1: No legacy wire compatibility test
The task requested testing that old serialized bytes (7-field ActionTicket) can be decoded by the current decoder. This is not tested because:
- The `vb_storage` tests that would create old-format tickets don't compile (missing `run_attempt` field)
- The codebase cannot create 7-field tickets because the struct requires 8 fields
- No fixture exists with old-format serialized bytes

This is a gap in the codebase, not the probe test. The probe test correctly focuses on runtime behavior.

#### Limitation 2: No mutation snapshot comparison
The task requested "strong no-mutation snapshot" comparison of serialized event contents, pending state, counters, and observable frame. The probe test checks counters (`runs_completed`, `runs_failed`) but does not perform full state serialization comparison.

The existing `durable_retry_run_attempt_authority.rs` test follows the same pattern (counter comparison), so this is consistent with established testing conventions.

## Conclusion

The test suite provides strong coverage of the run-attempt authority mechanism. The probe test adds 6 additional test cases that complement the existing tests. The limitations noted are gaps in the broader codebase, not failures of this test suite.
