# vb-avk4z TLC Runner Lock Repair Evidence

## Failure observed

Latest pre-repair `moon ci` output:

- Output file: `/home/lewis/.local/share/opencode/tool-output/tool_e739c8bf3001F2CdsrnL4Ezh0o`
- Summary: `Tasks: 28 completed (5 cached), 3 failed, 1 skipped`
- `velvet-ballistics:verify-tlc` failed while `velvet-ballistics:verify-tlc-workflow` succeeded on the same `WorkflowBoundedAdmission` model.
- Failure excerpt:
  - `Fatal errors while parsing TLA+ spec in file WorkflowBoundedAdmission`
  - `java.lang.NullPointerException: Cannot invoke "String.length()" because "str" is null`
  - `Error: Parsing or semantic analysis failed. Module-Table lookup failure for module name WorkflowBoundedAdmission derived from WorkflowBoundedAdmission file name.`

## Change

Updated `.moon/tasks/tlc.yml` so all TLC Moon tasks use an exclusive `target/moon-locks/tlc-runner.lock` instead of sharing `target/moon-locks/source-mutation.lock`.

Reason: the aggregate `verify-tlc` task and per-model TLC tasks can run concurrently during `moon ci`; TLC/SANY reads standard modules through `/tmp/*.tla`, so concurrent parser/model-checker invocations are serialized to avoid transient parser/module-table corruption while still leaving unrelated Rust/source-reader tasks unblocked.

## Commands and evidence

### Focused aggregate check before repair

Command:

```bash
moon run velvet-ballistics:verify-tlc
```

Result: PASS.

Relevant output:

- `WorkflowBoundedAdmission`: `Model checking completed. No error has been found.`
  - `2589 states generated, 1520 distinct states found, 0 states left on queue.`
  - `The depth of the complete state graph search is 7.`
- `IdempotencySafety`: `Model checking completed. No error has been found.`
  - `986 states generated, 306 distinct states found, 0 states left on queue.`
  - `The depth of the complete state graph search is 7.`

### Concurrent TLC task check after repair

Command:

```bash
moon run velvet-ballistics:verify-tlc velvet-ballistics:verify-tlc-workflow velvet-ballistics:verify-tlc-idempotency
```

Result: PASS (`Tasks: 3 completed`).

Relevant output:

- `verify-tlc-workflow`: `Model checking completed. No error has been found.`
  - `2589 states generated, 1520 distinct states found, 0 states left on queue.`
- `verify-tlc-idempotency`: `Model checking completed. No error has been found.`
  - `986 states generated, 306 distinct states found, 0 states left on queue.`
- `verify-tlc`: both models passed:
  - `WorkflowBoundedAdmission`: `2589 states generated, 1520 distinct states found, 0 states left on queue.`
  - `IdempotencySafety`: `986 states generated, 306 distinct states found, 0 states left on queue.`

### Full canonical gate after repair

Command:

```bash
moon ci
```

Result: FAIL, but the TLC blocker is repaired.

Output file: `/home/lewis/.local/share/opencode/tool-output/tool_e73a613aa001xAyMOStJuxFBRF`

Relevant TLC evidence in full CI:

- `verify-tlc-workflow`: PASS at lines 742-749 with `Model checking completed. No error has been found.`, `2589 states generated, 1520 distinct states found`, depth `7`.
- `verify-tlc-idempotency`: PASS at lines 886-893 with `Model checking completed. No error has been found.`, `986 states generated, 306 distinct states found`, depth `7`.
- `verify-tlc`: PASS at lines 779-786 for `WorkflowBoundedAdmission`, and lines 918-925 for `IdempotencySafety`.

Remaining full-CI blocker:

- `velvet-ballistics:source-length` still fails with hot-function and >300-line source violations.
- Full CI summary after this repair: `Tasks: 29 completed (5 cached), 2 failed, 1 skipped`.

## Model bounds/properties checked

`WorkflowBoundedAdmission.cfg`:

- Specification: `Spec`
- Invariants: `NoAckWithoutCertificate`, `NoAckOverCapacity`, `NoUncappedRunState`, `FailClosedNotRunnable`, `StepBudgetNeverNegative`, `NonTerminalProgressEnabled`
- Temporal properties: `EventuallyAckOrReject`, `EventuallyBlockedOrTerminal`

`IdempotencySafety.cfg`:

- Constants: `MaxRuns = 1`, `MaxActions = 1`, `MaxSeq = 3`, `NullDigest = 0`, `Digests = {0, 1}`
- Specification: `Spec`
- Invariants: `TypeOK`, `NoDuplicateJournalEvents`, `DigestBinding`, `TerminalStateInvariant`, `NoReplayOfResolvedActions`, `NoSuccessFailureConflict`, `JournalSeqMonotonicity`, `SeqWithinBound`, `OverflowIsTerminalFailSafe`, `TerminalHasNoNormalAppendEnabled`
- Temporal properties: `TerminalStateFinality`, `TerminalExactStepFinality`, `MonotonicCompletedActions`, `MonotonicFailedActions`, `RecoveryCorrectness`
- Deadlock check: `CHECK_DEADLOCK TRUE`

## Residual risk

- This bead repaired the TLC runner concurrency failure only.
- `moon ci` remains blocked globally by `source-length`; that must be handled under a separate bead.
- No Rust production code was changed.
- No performance claim was made.
