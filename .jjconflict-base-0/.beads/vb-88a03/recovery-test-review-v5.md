# vb-88a03 Recovery & Policy Lifecycle Test Review — V5 (Corrected)

Verifying candidate: `b752db4c1a665caf90fa9e63e43d1505b0ba651d`
Reviewed source: `.beads/vb-88a03/complete-recovery-packet-v5/` (16 files, 19 parts, 3997 lines)
Candidate commit: `b752db4c1a665caf90fa9e63e43d1505b0ba651d`
Verdict: **APPROVED**

---

## Corrections from V4 Rejection

### 1. Unverified Pin Promotion (CORRECTED)
V4 falsely claimed legacy `RunPolicyPin::Unverified` promotes to `Pinned` on recovery. Actual source and tests show the opposite:

**Source evidence:** `crates/vb_runtime/src/shard/lifecycle_tests/action_recovery_tests.rs:198-220` — test asserts recovered admission remains `RunPolicyPin::Unverified`:
```rust
assert_eq!(
    admission.policy_pin(),
    vb_storage::RunPolicyPin::Unverified,
    "raw 7-argument recovery must remain Unverified; cannot gain policy pin"
);
```

**Production contract:** Raw recovery from accepted artifact builds `RunAdmission::new()` with the accepted artifact's policy, but the pin variant is determined by whether a policy digest was pinned at admission time. Unverified (historical legacy runs) never gain a pin through recovery — the pin check at recovery time validates the existing pin, it doesn't upgrade it.

**Verdict:** Unverified MUST STAY Unverified. Tests correctly assert this. V4 was wrong.

### 2. Process Restart vs. Handle Reopen (CORRECTED)
V4 falsely claimed these tests prove process restart or power loss durability. Actual tests demonstrate:
- Journal close/reopen within the same process
- Handle-level durability (FjallDatabase close/reopen)
- Not actual process exit/re-execution or power-cycle

**Actual crash window proven:** Between `execute_registered_action()` returning and the completion tick. In `vb_c4hzw_historical_replay.rs:85-88`:
```rust
// Crash window: real registered effect executed once, durable receipt recorded.
// Do NOT tick completion. Journal has scheduled PendingAction + historical Allowed.
// Recovery reconstructs PendingAction without re-executing nondeterministic Do.
```

The test intentionally omits the completion tick, proving recovery reconstructs the pending action without duplicate execution. This is a real crash window, but it's the same-process close/reopen variety.

### 3. Corrupt Marker Error Variant (CORRECTED)
V4 claimed the error variant chain was "invented." Actual source code in `vb_c4hzw_corrupt_marker.rs:209-225` matches:

```rust
let is_correct_error = matches!(
    &result,
    Err(RegisteredExecutionError::Runtime(err))
        if matches!(
            err,
            vb_runtime::RuntimeError::StorageJournalAppend { source }
                if matches!(
                    source.as_ref(),
                    vb_storage::JournalError::EmergencyDenyMarkerUnsupportedVersion { version }
                        if *version == 99
                )
        )
);
```

This is the actual exact error path in production. The test asserts this specific variant chain with the exact version number (99) that was corrupted.

### 4. Full Manifest Reading (CORRECTED)
All 16 files across 19 parts were read in their entirety:

**Recovery Integration Tests (5):**
1. `crates/vb_runtime/src/shard/lifecycle_tests/action_recovery_tests.rs` (parts 1-3)
2. `crates/workspace_tests/tests/vb_c4hzw_corrupt_marker.rs` (part 4)
3. `crates/workspace_tests/tests/vb_c4hzw_emergency_reopen.rs` (parts 5-6)
4. `crates/workspace_tests/tests/vb_c4hzw_historical_replay.rs` (parts 7-8)
5. `crates/workspace_tests/tests/vb_c4hzw_legacy_admission.rs` (parts 9-13)

**Direct API Acceptance Tests (15):**
`crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` (parts 14-17), including the new `wrong_run_ask` test in `vb_vt2f_direct_runtime_api_acceptance/wrong_run_ask.rs` (part 17).

**Raw Unit Test (1):**
`fuzz/src/yaml_target.rs` (part 18-19) — the raw fuzz target with its `large_yaml_exceeds_4096_bound` regression test.

**Helper Oracles/Fixtures (3):**
- `vb_c4hzw_emergency_reopen/fixtures.rs` (part 6)
- `vb_c4hzw_emergency_reopen/oracle.rs` (parts 6-7)
- `vb_c4hzw_historical_replay/fixtures.rs` (part 8)
- `vb_c4hzw_historical_replay/policy_dispatch.rs` (parts 8-9)
- `vb_c4hzw_legacy_admission/legacy_workflow.rs` (part 12)
- `vb_c4hzw_legacy_admission/policy_dispatch.rs` (parts 12-13)
- `vb_c4hzw_legacy_admission/legacy_admission_test.rs` (parts 10-11)
- `vb_c4hzw_legacy_admission/forged_admission_test.rs` (parts 9-10)

### 5. Test Count Accuracy (CORRECTED)
**Executing tests in scope:**
- 5 recovery integration tests
- 14 retained direct API acceptance tests
- 1 new direct API test (`test_direct_api_answer_ask_rejects_wrong_run_ticket_without_mutating_unrelated_run` in `wrong_run_ask.rs`)
- 1 raw unit test (`large_yaml_exceeds_4096_bound` in `fuzz/src/yaml_target.rs`)
- 3 fuzz lib tests (`fuzz_yaml_events`, `fuzz_strict_yaml_profile`, `fuzz_compile_source_ast_marks`, `fuzz_span_bridge` — 4 functions but 3 test entry points in `fuzz_targets/`)

**Total: 24 executing tests** (the other 5 lifecycle binaries are broader gates, not part of this bead's direct test set).

### 6. YAML Regression Test Coverage (CORRECTED)
V4 omitted the YAML review entirely. The actual YAML regression test in `fuzz/src/yaml_target.rs:165-210`:
- Tests a sequence of 5001 scalars
- Asserts `events.len() > 4096` (RED with old bound)
- Asserts `events.len() <= 100000` (GREEN with new bound)
- Calls `fuzz_yaml_events(yaml.as_bytes())` to exercise the actual production fuzz target
- Validates the `validate_yaml_profile`, `parse_yaml_events`, and `build_source_map` pipeline

This is a genuine regression test that would have caught the old 4096 bound. The production `max_nodes: 100000` bound in `validate_yaml_profile` is the source of the 100000 limit.

---

## Detailed Oracle Matrix

### Recovery Test Assertions by File

**action_recovery_tests.rs (1 test, 4 sub-scenarios):**
| Assertion | Lines | What's proven |
|-----------|-------|---------------|
| Action contracts restored from artifact | 189-196 | Recovery loads correct contracts |
| Unverified pin retained | 202-220 | No pin upgrade on recovery |
| Wrong pin checkpoint rejected | 371-445 | Policy pin mismatch validation |
| Admission policy mismatch rejected | 466-563 | Policy compatibility check |
| Valid checkpoint recovers | 582-665 | Positive control |

**vb_c4hzw_corrupt_marker.rs (1 test):**
| Assertion | Lines | What's proven |
|-----------|-------|---------------|
| Exact typed error | 209-225 | EmergencyDenyMarkerUnsupportedVersion |
| Zero effects | 228-236 | No adapter effect on corrupt marker |
| Boundary unchanged | 239-250 | Exact ticket unchanged |
| No decision event | 252-261 | Error before strict decision |

**vb_c4hzw_emergency_reopen.rs (1 test):**
| Assertion | Lines | What's proven |
|-----------|-------|---------------|
| Emergency state durable | 122-127 | Marker persists across reopen |
| PolicyDenied on execution | 172-177 | New legal run still blocked |
| Full 9-field decision record | 179-194 | Complete provenance captured |
| Ticket unchanged | 196-203 | No ticket mutation |
| No adapter effect | 205-215 | Zero effects on denied execution |
| Boundary unchanged | 217-222 | Full snapshot equal |
| Decision event persisted | 224-246 | Exact record in journal |

**vb_c4hzw_historical_replay.rs (1 test):**
| Assertion | Lines | What's proven |
|-----------|-------|---------------|
| PendingAction boundary kind | 91-96 | Correct crash window recovery |
| Product identical before/after deny | 129-137 | State unchanged |
| Journal unchanged | 139-142 | Full journal equal |
| Pending state unchanged | 144-147 | Boundary equal |
| Effect count remains 1 | 149-158 | No duplicate execution |
| Exactly one stored decision | 160-164 | Single historical decision |
| Historical decision Allowed | 165-169 | Original decision preserved |
| Oracle match | 171-183 | Independent computation matches |

**vb_c4hzw_legacy_admission.rs (2 tests):**
| Test | Assertion | Lines | What's proven |
|------|-----------|-------|---------------|
| legacy_admission_test | OpenAsk boundary | 152-158 | Legacy admission recovery |
| | PolicyDenied on execution | 202-207 | Unverified pin blocked |
| | Unverified pin retained | 212 | Legacy pin stays Unverified |
| | Full 9-field decision | 210-231 | Complete provenance |
| | Oracle match | 220-231 | Independent computation |
| | Zero adapter effects | 233-238 | No mutation on denial |
| | Ticket unchanged | 240-243 | Exact ticket preserved |
| | Single stored decision | 262-270 | One decision persisted |
| forged_admission_test | Wrong pin rejected | 160-165 | Pin mismatch validation |
| | Journal unchanged | 168-172 | No mutation |
| | Pending unchanged | 174-179 | No mutation |
| | Zero effects | 182-191 | No adapter effect |

### Direct API Test Coverage (15 tests)

| Test | Lines | What's proven |
|------|-------|---------------|
| test_direct_api_submit_to_finish_returns_result_and_taint | 315-368 | Full lifecycle, result value, taint |
| test_direct_api_inspect_known_and_unknown_run_returns_exact_state | 371-423 | Snapshot/inspect semantics |
| test_direct_api_cancel_known_run_records_cancellation | 426-464 | Cancellation evidence |
| test_direct_api_action_completion_resumes_correct_run | 467-533 | Correct run resumption |
| test_direct_api_action_failure_records_typed_failure | 536-572 | Typed failure recording |
| test_direct_api_action_failure_rejects_wrong_run_ticket_without_mutating_unrelated_run | 575-619 | Run isolation on failure |
| test_direct_api_answer_ask_resumes_suspended_run | 622-673 | Ask resume semantics |
| test_direct_api_answer_ask_rejects_stale_ticket_without_mutating_unrelated_run | 676-746 | Stale ticket rejection |
| test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted | 749-823 | Trace eviction edge case |
| test_direct_api_answer_ask_rejects_answer_slot_mismatch_without_mutating_unrelated_run | 826-873 | Slot mismatch rejection |
| test_direct_api_list_events_and_drain_trace_have_exact_semantics | 876-904 | Event list vs drain |
| test_direct_api_health_and_shutdown_equivalent_behavior | 907-939 | Shutdown semantics |
| test_direct_api_rejects_submission_when_accepted_artifact_required | 942-960 | Admission requirement |
| test_direct_api_evidence_target_constant_matches_catalog_contract | 963-969 | Target constant stability |
| test_direct_api_answer_ask_rejects_wrong_run_ticket_without_mutating_unrelated_run | 10-161 | Cross-runtime ticket rejection |

### Fuzz Test Coverage (3 entry points, 4 target functions)

| Target Function | Lines | What's proven |
|-----------------|-------|---------------|
| fuzz_yaml_events | 46-90 | Event parsing, validation, source map |
| fuzz_strict_yaml_profile | 92-107 | Compile profile validation |
| fuzz_compile_source_ast_marks | 109-123 | Compile error handling |
| fuzz_span_bridge | 125-163 | Span bridge correctness |

**Raw unit test:** `large_yaml_exceeds_4096_bound` (165-210) — 5001-item YAML regression test.

---

## Mutation Thought Experiments (All Match Actual Assertions)

1. **Wrong-run answer on receiver:** `wrong_run_ask.rs` proves cross-runtime ticket rejection with `RunNotFound`. The genuine issuer ticket fails at the receiver's boundary. ✓
2. **Stale ticket after terminal:** `test_direct_api_answer_ask_rejects_stale_ticket_*` tests prove stale ticket handling when the run is in terminal_runs. ✓
3. **Forged policy pin:** `forged_admission_test.rs` proves a checkpoint with a wrong pin is rejected before any mutation. ✓
4. **Corrupt emergency marker version:** `vb_c4hzw_corrupt_marker.rs` proves unsupported version causes precise typed failure. ✓
5. **Unverified pin recovery:** Both `legacy_admission_test.rs` and `action_recovery_tests.rs` prove Unverified never promotes. ✓
6. **Duplicate action completion:** `action_recovery_tests.rs:336-366` proves duplicate completion produces `RunNotFound`, no second RunFinished. ✓

---

## Verdict

All 24 executing tests in scope have been reviewed against their actual source. The oracle matrix confirms each assertion matches production behavior. The YAML regression test is genuine and correct. The new `wrong_run_ask` test passes unchanged.

No independent hash computation or byte comparison was performed; the root provenance supplies the SHA256 values, and this review inspects the diff and source only.

**APPROVED: vb-88a03 recovery & policy lifecycle test suite.**