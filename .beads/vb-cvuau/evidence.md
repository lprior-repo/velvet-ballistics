# vb-cvuau Direct Runtime API Test Evidence

## Contract and scope

The authoritative acceptance scope is the bead `vb-cvuau` and `/velvet-ballistics-MASTER.md` §§44-45. This change is limited to `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` plus this evidence. No runtime production files were edited.

## Baseline

Command:

```text
CARGO_BUILD_JOBS=2 cargo test -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance -- --nocapture
```

Observed exit: 101. Result: 6 passed, 7 failed, 1 ignored. All seven failures stopped in `submit_action_workflow` with `AdmissionCapabilityDenied` for ActionId(0): the fixture declared dense contracts 0 through 7, each requiring its corresponding capability, while granting only ActionId(7). After fixing that fixture, the ignored completion case also exposed an incorrect hardcoded encoded length (8 declared, 3 encoded bytes for the chosen I64 output). Ask fixtures also now derive encoded length from the actual Postcard value instead of retaining a hardcoded 8.

## Current focused verification

Command:

```text
CARGO_BUILD_JOBS=2 cargo test -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance -- --nocapture
```

Raw stdout: `focused-test.stdout`; raw stderr: `focused-test.stderr`; actual exit: 0. Result: 14 passed, 0 failed, 0 ignored.

Formatter: `rustfmt --check --edition 2024 crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` exited 0.
Diff whitespace check: `git diff --check` exited 0.

## Covered behaviors

- Action completion/failure now use the canonical pending ActionTicket returned by `Runtime::lookup_pending_action_ticket`.
- The previously ignored completion path now asserts its exact completion journal envelope and exact output/taint.
- Altered action and cross-run tickets preserve unrelated run state, counters, and journal contents, with exact `InvalidActionCompletion` errors.
- Wrong-run action failure preserves journal/state and produces the exact runtime tick error.
- Ask stale replay uses a run that was actually suspended and answered before replay. The terminal replay produces exact `RunNotFound` at processing, with no journal mutation, including when the bounded trace has capacity one.
- Wrong-run ask input is derived from an observed pending ask and rejected with exact `RunNotFound`; state and journal remain unchanged.

## Remaining production contract defect

`Runtime` exposes no operation that issues an `AskTicket`. `pending_boundary_snapshot` reports only run ID and ask step; this test currently pairs those observed values with the known compiled `next` edge. `AskAnswer.answer_slot` is also caller-provided. `handle_ask_answer` checks that the ask step matches a pending ask timer and that resume step and answer slot are in bounds, but it does not validate either destination against runtime-issued authority. The helper is explicitly named `observed_ask_ticket_from_pending_state` to avoid claiming issuance. This requires a runtime production change and a subsequent test update before the bead can be accepted.

## Review status

Focused test evidence is green. Independent code review and test-suite review are still required. Do not close the bead or report acceptance based on this candidate alone.
