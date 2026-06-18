STATUS: APPROVED
reviewer_skill: test-reviewer
reviewer_invocation_id: tier-a-0-002-s10-test-reviewer-rereview-sif-7bb1d2c4
writer_invocation_id: tier-a-0-002-s8-test-planner-repair-a3f91c7e
previous_reviewer_invocation_id: tier-a-0-002-s10-test-reviewer-rereview-5d6f7a91
review_state: 10
bead_id: tier-a-0-002
workspace: /home/lewis/src/femdation-tier-a-0-002
schema_version: test-plan-review/v1
reviewed_at: 2026-06-18T08:15:00.000000+00:00

# Test Plan Re-Review — tier-a-0-002

## Findings

None. The repaired State 8 plan now has contract parity with the hot-crate scan roots, exact diagnostic format, exhaustive tested GateError scenarios including `ScriptInvocationFailure`, allowlist precedence, and the hard 30-second fail-closed resource bound.

## Closure Matrix Against Prior Findings

| Prior finding | Closure status | Evidence reviewed | Reviewer disposition |
|---|---:|---|---|
| Original State 10: hot-crate paths were bypassed by `${TMPDIR}/src/lib.rs` staging | fixed_with_evidence | `test-plan.md` §1 and §2 now limit fixtures to the four contracted hot roots and require staging at `crates/vb_core/src/lib.rs`, `crates/vb_runtime/src/channel.rs`, and `crates/vb_core/src/allowlisted.rs`. | Closed. |
| Original State 10: GateError scenarios were incomplete | fixed_with_evidence | `test-plan.md` §2 steps 9-10, §2 Test 2 step 9, §2 Test 3 step 6, §6.8, and §7 M-7 require exact variant/exit assertions for `PatternFileMissing`, `AllowlistParseFailure`, `GlobUnreadable`, and `ScriptInvocationFailure`. | Closed. |
| Original State 10: allowlist precedence was mapped mainly to moon structural checks | fixed_with_evidence | `test-plan.md` §2 Test 3 separates the RRO-RQ-004 behavior fixture (`positive_allowlisted.rs` staged at `crates/vb_core/src/allowlisted.rs`) from moon wiring checks, with exact `allowlisted:` line, summary, and active-line omission. | Closed. |
| Original State 10: exact file/line diagnostics were stale and allowed `exact substring` wording | fixed_with_evidence | `test-plan.md` §2 asserts exact `<file>:<line_no>: RUNTIME-FMT: <forbidden_name>: <snippet>` lines for both active fixtures and explicitly rejects stale formatter wording. | Closed. |
| Original State 10: 30-second bound was measurement-only, not fail-closed | fixed_with_evidence | `test-plan.md` §1, §2 resource-bound rows, §6.4, and §7 M-8 require `timeout 30s` around every gate invocation and treat timeout exit 124 as a hard failure. | Closed. |
| Re-review blocker TPR-RR-001: canonical test plan remained stale after suite repair | fixed_with_evidence | `test-plan.md` was repaired after the prior review and now aligns with the repaired suite, including hot roots, exact diagnostics, allowlist fixture, all GateError scenarios, and hard timeout. | Closed. |
| Re-review blocker TPR-RR-002: `ScriptInvocationFailure` had no exact behavior scenario | fixed_with_evidence | `test-plan.md` §2 Test 3 step 6, §6.8, §7 M-7, and §9.2 require `FORBID_RUNTIME_FMT_FORCE_SCRIPT_INVOCATION_FAILURE='forced script invocation failure'`, exit 2, and exact `GateError:ScriptInvocationFailure: forced script invocation failure`. | Closed. |

## Contract-Parity Review

| Contract area | Plan evidence | Result |
|---|---|---:|
| Scan scope | Only `crates/vb_core/src`, `crates/vb_runtime/src`, `crates/vb_storage/src`, and `crates/vb_ipc/src` are in scope; cold crates and ad-hoc `${TMPDIR}/src/lib.rs` staging are excluded. | pass |
| Active residue behavior | Tests 1 and 2 assert exit 1 plus exact hot-crate file/line diagnostics and summary counts. | pass |
| Error behavior | Error sub-scenarios assert exit 2 and exact `GateError:<VariantName>:` text for all planned test-facing variants, including the previously missing `ScriptInvocationFailure`. | pass |
| Allowlist precedence | Test 3 requires the allowlisted tuple to emit `allowlisted:` with `active=0 allowlisted=1` and omit the active `RUNTIME-FMT:` line. | pass |
| Moon wiring | Test 3 checks the real task graph and a negative graph without the dependency. | pass |
| Determinism/resource bounds | Every gate invocation is planned under `timeout 30s`; the real repository scan also asserts nanoseconds under `30_000_000_000`. | pass |
| Mutation resistance | §7 names mutation classes M-1 through M-8 and maps each to a concrete test or validator/static-review evidence. | pass |

## Reviewed Evidence

```text
COMMAND: bash -n scripts/test-forbid-runtime-fmt.sh
exit_status=0

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import
[1/3] test_quarantine_gate_blocks_json_import
AssertionFailed: gate script is missing or not executable: /home/lewis/src/femdation-tier-a-0-002/scripts/forbid-runtime-fmt.sh
exit_status=1

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel
[2/3] test_quarantine_gate_blocks_unbounded_channel
AssertionFailed: gate script is missing or not executable: /home/lewis/src/femdation-tier-a-0-002/scripts/forbid-runtime-fmt.sh
exit_status=1

COMMAND: bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered
[3/3] test_moon_ci_quarantine_dependency_correctly_ordered
AssertionFailed: real moon task graph expected exit 0, got 1
Output:
MISSING-TASK: forbid-runtime-fmt not declared
exit_status=1
```

The failing-first executions are expected before State 11 because the gate implementation and moon wiring are not yet present. They prove that the repaired tests run, fail deterministically, and fail on the intended missing implementation boundary rather than silently passing.

## Disposition

Approved. All prior plan-review blockers are closed with evidence in the repaired plan and suite.
