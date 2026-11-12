# Code Review — vb-7x0ol (Independent Integration Review)

**Bead**: vb-7x0ol
**Reviewer**: local-5090 independent reviewer (DeepSeek v4 Pro via OMP)
**Source checkout**: `/home/lewis/src/isoloated/velvet-ballistics-integration-review-local5090-20261021`

## Frozen Source SHAs (reviewed)

| Role | SHA |
|------|-----|
| Review merge (`@`) | `08af7113e984fa9ddaa0103d23faa57afcf33ac8` |
| Integration candidate parent | `90ab770633c2130501ae2db889374c205a960d69` |
| Digest-repair parent | `c789b1b35cb5d19a60cf2f786cbda5947109dcb1` |

**Final candidate (author push during review):** integration parent rewritten to
`1e7aeee2495f2cfca0e6c1e7719727dc7515ea0a` (merge `442ebc679db4340ae0f9ab0ad1da59f253338f62`).
Final-delta audit in §G does **not** resolve the blockers below.

## Gate Result

**STATUS: REJECTED**

---

## Findings (Ordered by Severity)

### F-1 [CRITICAL] Executor conformance surface is unwired; recovery marks replayed runs `Live` and the receipt guard is dead code

**Location**: `crates/vb_runtime/src/runtime/registered_executor.rs:45`, `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:431`

**Problem**:
- `Runtime::validate_registered_recovery` is **dead code** — zero callers anywhere in the
  repo (confirmed by compiler: `warning: method \`validate_registered_recovery\` is never used`
  and by grep). It is the only function that would require `require_receipt_recovery` for a
  recovered run with a pending action.
- `Runtime::execute_registered_action` and `Runtime::reconcile_registered_action` are `pub`
  but have **no callers** anywhere (CLI, IPC, shard drive loop, tests). The whole
  "opaque checked suite/catalog + atomic Fjall target+receipt adapter" surface is disconnected
  from any dispatch.
- The legacy recovery path unconditionally restores
  `registered_execution: RegisteredExecutionMode::Live`
  (`shard/lifecycle/chunk_001.rs:431`) for a journal-replayed run. The only read of that mode
  is `registered_executor.rs:24` (`execute_registered_action` requires `Live`), which is itself
  uncalled. `RunCheckpointV1::restore` (retry path) correctly sets `ReconcileOnly`
  (`run_checkpoint.rs:213`), but the legacy path does not.

**Evidence**:
```
warning: method `validate_registered_recovery` is never used
  --> crates/vb_runtime/src/runtime/registered_executor.rs:45:19
```
`grep -rn "execute_registered_action|reconcile_registered_action|validate_registered_recovery"`
across `crates/` returns only the definitions in `registered_executor.rs`.

**Required Fix**: either wire `execute_registered_action`/`reconcile_registered_action` into a
real dispatch (IPC/CLI/shard drive loop) that consults `RegisteredExecutionMode`, and call
`validate_registered_recovery` during recovery to downgrade replayed runs to `ReconcileOnly`;
or explicitly descope the executor-conformance acceptance criterion. Accepting point #4 as-is
is unsound: "recovery NEVER executes live side effects during replay" is not enforced by any
reachable code.

---

### F-2 [CRITICAL] Retry durable path is a stub — CLI always reports "durable retry execution is unavailable"

**Location**: `crates/velvet_ballistics/src/run_ops.rs:59`

**Problem**: `cmd_retry` (dispatched from `Command::Retry { run_id, step, db, output }` in
`dispatcher.rs:96`) validates the failed step, then unconditionally emits:

```rust
let message = format!(
    "run {run_id} failed at step {requested_step}, but durable retry execution is unavailable"
);
...
return CliExitCode::LifecycleError.into();
```

It never constructs/persists a `RunCheckpointV1`, never resets the selected failed step's
attempt counter (`attempt++ reset only selected failed step`), never restores
`ValueStore`/frame/taint/`action_attempts`/`collect_state`/admission/contracts, and never
cold-reopens to reproduce the result. `RunCheckpointV1::restore` is reachable only through
`RecoverRunCommand.retry_checkpoint`, which has **no** CLI/IPC producer
(`with_retry_checkpoint` is used only in `shard/types.rs` helper; no caller).

**Required Fix**: implement the durable retry execution path end-to-end, or mark contract
point #3 explicitly incomplete and separate-land it. The current code is a planner/checkpoint
scaffold, not a feature.

---

### F-3 [HIGH] C6 retained-input acceptance test cannot run — helper parses YAML as JSON

**Location**: `crates/velvet_ballistics/tests/vb_md3w_retained_input_recovery.rs:41,49`

**Problem**: `submit` invokes the CLI with `--emit yaml` (line 41) then parses stdout with
`serde_json::from_str` (line 49). `--emit yaml` routes through `output::json_out` →
`serde_saphyr::to_string`, producing YAML (`run_id: <n>\ndigest: ...\nstatus: submitted`), not
JSON. The parse fails with `expected value at line 1 column 1`.

**Evidence**: reproduced with `cargo test -p velvet-ballistics --test vb_md3w_retained_input_recovery`
(exit 101):
```
test retained_inputs_recovered_from_cold_storage_with_runtime_api ... FAILED
Error: Error("expected value", line: 1, column: 1)
```
Full log: `.beads/vb-7x0ol/evidence/c6_retained_input_failure.log`.

**Impact**: the C6 contract ("public CLI retained source/input files removable before recover
with same runtime result") is not demonstrated end-to-end. The runtime recovery plumbing
(`load_retained_inputs_for_recovery`, `validate_input_record_binding/artifact/workflow`) is
present and appears plausible, but the acceptance gate is broken at the test-harness boundary.

**Required Fix**: make the helper parse YAML (or emit/parse a machine format the CLI actually
produces) and re-run to prove the public-API recovery path.

---

### F-4 [HIGH] Canonical digest fails open on `#[non_exhaustive]` variants and silently saturates string lengths

**Location**: `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs`

**Problem**:
- `digest_author_value` unknown variant hashes constant `0xFF` (`:119-122`); `TriggerAst`
  unknown `0xFF` (`:172-175`); `Save`/`Finish` `ScalarValue` unknown `0xFF` (`:343-345`,
  `:468-470`). All these enums are `#[non_exhaustive]` (`vb_yaml/src/yaml_ast/types/step.rs`).
  Only `digest_step_primitive` correctly fails closed (`:483-487` returns
  `CompileError::CanonicalYaml`). A future variant addition silently produces a constant
  `0xFF` hash → cross-variant collision and admission-binding bypass, with no compiler error.
- `digest_string` uses
  `u32::try_from(value.len()).map_err(|_| "string length exceeds u32 range").unwrap_or(u32::MAX)`
  (`:28-29`). The `.map_err(...)` result is discarded and the length silently saturates to
  `u32::MAX`; the "fail closed on out-of-bounds length" contract is not honored and the
  error branch is dead code.

**Required Fix**: thread `Result` through `digest_author_value`/`digest_string` and return a
typed `CompileError` on unknown variant or over-`u32` length, matching the
`digest_step_primitive` fail-closed pattern. Do not hash a sentinel `0xFF`.

---

### F-5 [MEDIUM] `at_once: None` vs `Some(1)` digest semantics silently flipped

**Location**: `crates/vb_compile/src/tests/foreach_digest_tests.rs:129-160` (assertion flip);
production `digest_optional_u32` in `part_05_digest.rs:67-79`.

**Problem**: the digest-repair commit changed two tests from `assert_eq!` (None == Some(1),
"semantic equivalence") to `assert_ne!`. If the compile layer lowers `at_once: None` and
`Some(1)` to the same numeric IR, then two semantically identical workflows now produce
different source digests, breaking digest stability (master §59) and the source→artifact
binding used by admission/recovery. The change needs an independent oracle proving the two
are genuinely distinct at the source-authoring identity layer.

**Required Fix**: confirm against `part_07` lowering whether `None`/`Some(1)` lower to the same
IR; if so, revert to canonical-equivalent hashing; document the decision.

---

### F-6 [LOW] `part_05_digest.rs` exceeds cold-path function-length guidance

**Location**: `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs` (492 lines;
`canonical_digest` ~97 lines at `:145-241`, `digest_step_primitive` ~166 lines at `:322-488`).

**Problem**: master doc §125 permits large cold-path validation functions only with a
bead-linked justification. None is present. This is a cold compile-time path, so not a hot-path
violation, but the coordinator-listed "integration blocker" (large file/functions) remains
unjustified.

---

## Quality Gates (verified)

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p velvet-ballistics --test vb_md3w_retained_input_recovery` | ❌ FAIL (exit 101) | `expected value at line 1 column 1` |
| `vb_runtime` lib compile | ✅ (1 dead-code warning) | `validate_registered_recovery` never used |
| Digest unit/integration test files | ⚠ not run in this review (not needed for verdict) | source-level collision-oracle review only |

---

## §G Final-Delta Audit (new candidate `1e7aeee2`)

Author pushed `1e7aeee2495f2cfca0e6c1e7719727dc7515ea0a` during review
(delta `90ab7706..1e7aeee2`: 34 files, +610/−244). Verified against the live working tree:

- **F-2 unchanged** — `run_ops.rs` not in the delta; `cmd_retry` still returns
  "durable retry execution is unavailable" (`run_ops.rs:59`).
- **F-3 unchanged** — C6 helper still `--emit yaml` + `serde_json::from_str` (delta is
  import reorder/line-wrap only; 15 lines removed, none touching the YAML/JSON mismatch).
- **F-1 unchanged** — `registered_executor.rs` grew (+108) and `registered_completion.rs`
  (+102) and the conformance test (+125), but `execute_registered_action` /
  `reconcile_registered_action` / `validate_registered_recovery` still have **no callers**.
  The surface remains unwired; legacy recovery still sets `Live`.
- The delta is predominantly clippy reformatting (`if let` chains, import ordering) plus
  test expansions — it does not close any blocker.

**Verdict for final candidate: REJECTED** (same blockers F-1, F-2, F-3 persist).

---

## Verdict

**STATUS: REJECTED** (frozen `08af7113` and final `1e7aeee2`).

### Summary

Three integration blockers remain: (1) executor conformance is an unwired, dead surface with
recovery marking replayed runs `Live` and the receipt guard uncalled; (2) the retry durable
path is a stub that always returns "unavailable"; (3) the C6 retained-input acceptance test is
broken at the harness boundary (YAML parsed as JSON), so C6 is unverified. The digest repair
introduces a fail-open `0xFF` fallback on `#[non_exhaustive]` enums and dead saturation code.

## Required Repair Actions

1. [CRITICAL] Wire registered execution/reconciliation into dispatch and invoke
   `validate_registered_recovery` to force `ReconcileOnly` on replayed runs — or descope the
   executor-conformance acceptance.
2. [CRITICAL] Implement durable `RunCheckpointV1` retry end-to-end (or mark point #3
   incomplete and separate-land).
3. [HIGH] Fix the C6 test helper (YAML vs JSON) and prove the public-API recovery path.
4. [HIGH] Make `digest_author_value`/`digest_string`/`ScalarValue`/`TriggerAst` fail closed
   (typed error) instead of hashing `0xFF`/saturating.
5. [MEDIUM] Resolve the `at_once None vs Some(1)` digest-stability question with an oracle.
6. [LOW] Add bead-linked justification for the large cold-path digest functions.
