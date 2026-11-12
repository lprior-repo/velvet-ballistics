# Domain Model Review: vb-qi37.1

## Verdict
STATUS: REVIEW_REQUIRED_BY_INDEPENDENT_AGENT

This State 3 artifact is authored by `rust-contract`; it does not approve itself.

## Model adequacy
- `RecoveryError` is the primary error algebra and is sufficiently typed for journal failure, digest mismatch, missing data, corrupt snapshot, replay divergence, terminal mismatch, and dimension overflow.
- `RecoveryHydration` correctly separates summary-only recovery from full `RecoveryFrameSeed`; runtime conversion must preserve that split.
- `UnsupportedRecoveryState` is a critical reject lattice. Any true flag means live-frame hydration is not supported.
- `RecoveryFrameSeed` is the correct data carrier for pc, dimensions, step state, slot value/taint, pending actions, terminal summary, and unsupported facts.
- `RunSnapshot` plus tail events model snapshot+journal replay, but tail sequence and run identity must remain hard preconditions.

## Illegal-state pressure points
- Summary-only recovery must not be representable as a hydrated live frame.
- Missing slot taint must not degrade to `Taint::Clean` unless a proof establishes that clean taint is durably correct.
- Pending actions must not be dropped just because current `RunFrame` cannot resume them.
- Full digest verification must not report success while action ABI or policy digest checks are deferred, unless the mode explicitly excludes those checks.
- Crash recovery must not use YAML reparsing as a substitute for durable accepted artifact and journal evidence.

## Required model refinements for downstream states
- Resolve OQ-001 by documenting the durable taint representation or retaining fail-closed unsupported-taint behavior.
- Resolve OQ-002 by either projecting waits/asks/retries/collect pagination into live-frame state or documenting typed fail-closed diagnostics.
- Resolve OQ-003 by implementing full digest checks or splitting action ABI/policy digest proof into an explicit blocker.

## Review handoff
Independent contract-verification reviewer must check:
- every contract clause maps to at least one proof/test obligation;
- JSONL artifacts parse as JSONL;
- TLA+ model path is treated as planned/blocked, not falsely executed;
- Verus target `verification/verus/recovery_verification.rs` is adequate or marked insufficient by a follow-up bead.
