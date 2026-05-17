# Black Hat Review — vb-qi37.12.2

STATUS: REJECTED

Startup authority read and applied:
- `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md` lines 12-16 require exact contract parity and immediate rejection on parity failure; lines 18-21 require rigor and meaningful tests.
- `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md` lines 12-16 require exact contract parity and immediate rejection on parity failure; lines 18-21 require rigor and meaningful tests. `.agents` wins and matches.

## Findings

### F1 — CRITICAL — The new registry still does not bind source to the returned error

- Contract: `.beads/vb-qi37.12.2/contract.md:10` requires source runtime/storage errors to be preserved across resume boundaries.
- Implementation: `crates/vb_runtime/src/shard/types.rs:28-36` keeps sources in a same-thread `ResumeSourceRegistry` with a `pending` queue and address-keyed `bound` queue.
- Implementation: `crates/vb_runtime/src/shard/types.rs:47-64` records a source independently of any `ResumeError` value, then binds the next caller of `source_runtime_error()` to the oldest pending source.
- Implementation: `crates/vb_runtime/src/shard/types.rs:439-442` still returns the public unit variant `ResumeError::JournalAppendFailed`; no source identity is stored in the error.
- Implementation: `crates/vb_runtime/src/shard/types.rs:452-460` computes identity from `std::ptr::from_ref(self).addr()`, i.e. the address of the current stack/object location, not a stable failure identity.

This is the same side-channel defect with a queue bolted onto it. The source is not correlated to the returned error reference until the first accessor call. A fresh/manual `ResumeError::JournalAppendFailed` can steal a pending source if it is observed before the real returned error. A previously observed error can also lose its source when moved to a new address and converted later. Address-of-`self` is not a semantic identity for an enum value; it changes on move and clone.

Mandated fix: stop pretending a unit enum plus TLS queue is source preservation. Use an internal error envelope/result type that carries `(ResumeError, RuntimeError)` until conversion, or add a stable opaque source token/identity that is created with the returned failure and survives observation/move/clone semantics as required by the public contract. If public semver forbids adding a variant field, the public `source_runtime_error()` guarantee must be narrowed or removed; it cannot honestly promise error-bound source identity with this implementation.

### F2 — HIGH — The new regression tests miss the exact stale-pending theft path

- Tests: `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs:313-338` observe `first_error.source_runtime_error()` before creating `second_error`, so `first_error` consumes/binds its source before the later failure. That dodges the bug.
- Tests: `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs:340-360` also observe `prior_error.source_runtime_error()` before constructing the fresh unit error, so the pending queue is already drained. Again, the stale-pending defect is not exercised.
- Tests: `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs:362-384` repeat the same mistake before conversion.

The missing killer case is simple: create a sourced resume failure, do **not** call `source_runtime_error()` on it, then construct/convert a fresh `ResumeError::JournalAppendFailed` on the same thread. Under `types.rs:54-64`, the fresh value will pop and bind the real failure's pending source. The tests pass because they drain the queue before trying the negative checks.

Mandated fix: add tests that fail against the current registry:
1. sourced error created but unobserved; fresh unit `JournalAppendFailed.source_runtime_error()` must be `None`;
2. sourced error created but unobserved; `RuntimeError::from(ResumeError::JournalAppendFailed)` must use fallback, not the pending source;
3. sourced error observed once, then moved into `RuntimeError::from(error)` must still convert with the same source if that is the claimed API contract;
4. clone/drop/move behavior must be specified and tested, or `Clone` must not imply source observability.

### F3 — HIGH — Public semver compatibility is being used to hide a contract downgrade

- `crates/vb_runtime/src/error/conversions.rs:21-38` exposes source preservation during `From<ResumeError> for RuntimeError` through the same address/TLS accessor.
- `crates/vb_runtime/src/shard/types.rs:413-435` keeps `ResumeError::JournalAppendFailed` as a unit variant, so the public error value still carries no source.
- `.beads/vb-qi37.12.2/machine-gate-report.md:17` says `cargo semver-checks` passed, but semver compatibility is not contract compatibility.

Observable source is now conditional on call order, object address, and whether another same-thread value drains the queue first. That does not satisfy R5. Passing semver only proves the API shape did not change; it does not prove the API still truthfully represents source causality.

Mandated fix: either carry source in a real internal/public typed object until all conversions are complete, or update the contract to admit that `ResumeError::JournalAppendFailed` cannot preserve sources under the current public enum shape. Do not ship a misleading accessor.

### F4 — MEDIUM — Evidence artifacts approve R5 without adversarial source-binding analysis

- `.beads/vb-qi37.12.2/machine-gate-report.md:13-18` reports passing tests/clippy/semver/mutation, but no gate covers unobserved pending-source theft.
- Prior proof/evidence artifacts mark R5 as passed while the current implementation remains a mutable same-thread side channel.

State 11 PASS is not persuasive for this defect. The machine gates prove the written tests pass; the written tests are not the right tests.

Mandated fix: refresh verification artifacts after fixing implementation and tests. R5 must include an explicit invariant: no unrelated `JournalAppendFailed` can observe or consume a source recorded for a different resume failure, regardless of observation order.

## Non-blocking observations

- Failed `Resumed` append restores `RuntimeState::Resumable` at `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:183-188`.
- Drive-run failure now returns `Err` instead of being silently swallowed at `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:197-215`.
- `NotResumable` carries `run_id` and `current_state` at `crates/vb_runtime/src/shard/types.rs:420-426` and is populated at `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:149-153`.

## Verdict

REJECTED. State 12 still fails the previous defect. The old stale slot became a bounded same-thread registry, but source binding is still ambient and call-order dependent, not bound to the returned resume error. Contract R5 remains false.

## Routing

Route back to implementation + test repair. Add adversarial unobserved-source tests, replace the TLS queue design with real error-bound source preservation or narrow the public contract, rerun State 11, then rerun black-hat.
