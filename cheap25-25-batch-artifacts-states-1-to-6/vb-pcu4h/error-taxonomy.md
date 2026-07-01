# Error Taxonomy — vb-pcu4h

- bead_id: vb-pcu4h
- artifact_owner: rust-contract
- scope: errors relevant to the recovery-reducer test-edit lane; no production-error taxonomy change.

## Producer error type (read-only)

`recover_runtime_frame_seed_from_events` returns `RecoveryResult<RecoveryFrameSeed>` where `RecoveryResult<T> = Result<T, RecoveryError>` and `RecoveryError` is defined in `crates/vb_storage/src/recovery/mod.rs` (re-exported at `replay::summary::*`).

The three PRIMARY test fixtures are constructed to yield `Ok(seed)`; under ordinary operation the reducer does not raise `RecoveryError`. The taxonomy below therefore matters for the SECONDARY targets and for any future hostile-input scenario, not for the PRIMARY test path.

## Recovery error variants observed in the recovery lane

| Variant | Triggers | Test-relevance |
|---------|----------|----------------|
| `RecoveryError::ReplayDivergence { step, detail }` | Event sequence is internally inconsistent (e.g., `StepStarted` for a step already terminal in seed). | Test C's preamble (`RunAccepted`, `StepStarted`, `ActionScheduledTicket`) is consistent; reducer is `Ok`. |
| `RecoveryError::WorkflowDigestMismatch { expected, observed }` | `recover_runtime_frame_seed_from_events_with_workflow` is called with a workflow whose digest does not match the journal's `RunAccepted.workflow`. | Not exercised by PRIMARY or SECONDARY targets. |
| `RecoveryError::InvalidEventForWorkflow` | Reserved for workflow-bound reduction failures. | Not exercised by PRIMARY or SECONDARY targets. |
| `RecoveryError::Unsupported { kind }` | Recovery classified the seed as unsupported; `kind` discriminates `PendingActions`, `SlotValues`, etc. | Test A's seed has `unsupported.pending_actions == true`; the reducer still returns `Ok(seed)` (unsupported is metadata, not an error). |

## Test-side error reporting

The bead's fix changes how the test REPORTS a recovery failure. Two layers:

### Layer 1: `expect("…")` panic-on-Err

```
let recovered = seed.expect("seed recovery must succeed for single ActionScheduled");
```

- Panics if `seed == Err(_)`; the panic message names the failure mode and the `Err(_)` Debug payload.
- Replaces the outer `assert!(matches!(seed, Ok(_) if …))` pattern in Test A.
- The panic is a test failure (the test framework reports `panicked at … expect("…") … Err(…)`).
- Severity: this is a test-failure report, NOT a production error path. Production code is unaffected.

### Layer 2: `assert_eq!` Vec / struct panic

```
assert_eq!(
    recovered.pending_actions,
    vec![RecoveredPendingAction { step: StepIdx::new(3), action: ActionId::new(9) }]
);
```

- Panics with Rust's default `assert_eq!` formatter that prints both sides as their `Debug` repr, including:
  - Length mismatch: `assertion `left == right` failed: … left.len() != right.len()` (via the derived `Debug` for `Vec`).
  - Element mismatch: `assertion `left == right` failed: … RecoveredPendingAction { step: …, action: … }` (via the derived `Debug` for the struct).
- Severity: test failure; the panic names the field that drifted.
- The replacement does NOT use `assert!` with a hand-built message; the default formatter is sufficient because both `RecoveredPendingAction` and `Vec` have derived `Debug`.

## Forbidden error-shapes (anti-patterns)

- ANTI-E-1 — Hiding `Err(_)` behind `matches!`. `assert!(matches!(seed, Ok(recovered) if …))` reads as "assert recover succeeds AND inner condition", but the matches! returns false for `Err`, the outer assert! panics on false (good), AND the inner condition is never evaluated on `Err`. The audit's silent-pass concern therefore applies only when the matches! result is consumed without an outer assert! (which is NOT the case here). The contract STILL recommends the `.expect()` rewrite because:
  - The `.expect()` panic message is more diagnostic (names the fixture event).
  - It removes a redundant `Ok(_)` arm so the assertion site reads as a single linear block.
- ANTI-E-2 — Asserting only `unsupported.pending_actions`. The boolean is a single Bool; it cannot catch Vec length drift (FAIL-2), phantom-duplicate (FAIL-3), or field drift (FAIL-4). The contract REQUIRES pairing the boolean with Vec-equality in Test A.
- ANTI-E-3 — Asserting only `summary.steps_started` (or any derived counter). The audit's "only checks steps_started count" phrasing applies to this anti-pattern. No current PRIMARY or SECONDARY target uses this anti-pattern, but the contract flags it as the historical bug shape.

## Failure classification

| Failure | Detection | Severity | Test failure mode |
|---------|-----------|----------|---------------------|
| `Err(_)` returned by reducer | `.expect("…")` | Test fail | Panic with named message + Debug `Err(_)` |
| Vec empty | `assert_eq!` on Vec | Test fail | Panic with `Vec` Debug (length 0 vs 1) |
| Vec length > 1 | `assert_eq!` on Vec | Test fail | Panic with `Vec` Debug (length N vs 1) |
| Field drift | `assert_eq!` on Vec (per-element) | Test fail | Panic with `RecoveredPendingAction` Debug (both fields) |
| Unsupported flag false (Test A) | `assert!(... pending_actions)` | Test fail | Panic with Boolean message |
| Compiled-time shape drift on `RecoveredPendingAction` | Drift gate (production mirror unchanged) | CI fail | Drift gate failure (`check-production-inner-drift.sh`) |

## Out-of-scope error variants

- `VbError` / runtime error variants — the bead is bounded to recovery tests; runtime errors are not modified.
- Codec error variants in `vb_storage/src/codec/*` — not touched; the bead does not modify the codec.
- Verus `assume` / `axiom` rows — no new trusted-base rows are added because the bead is test-only and no production code is modified.