# CLI Retry Contract

## Domain

- **Run**: existing persisted runtime execution identified by parsed non-zero `RunId`.
- **Failed step**: the exact `StepIdx` observed from the latest eligible failure evidence in the ordered journal. It is not inferred by incrementing a previous successful numeric index.
- **Retry attempt**: a new bounded execution attempt for that run; its number must advance monotonically and prior-attempt action completions/failures cannot mutate the current attempt.
- **Authority**: the run's accepted artifact, workflow digest, journal prefix, and runtime recovery product. Caller-provided step labels alone do not grant execution authority.

## Transition

`Failed(run, step, attempt_n, prefix)` --retry after validation--> `Active(run, step, attempt_n+1, prefix + retry evidence)`.

Reject missing runs, non-failed states, terminal finished/cancelled runs, malformed or inconsistent journal evidence, retry-attempt overflow, stale caller-selected steps, missing/corrupt accepted artifacts, non-resumable recovery products, and retry-unsafe action contracts. Rejection must leave durable journal bytes/events unchanged.

## Invariants

1. Retry preserves every event in the prior journal prefix and appends evidence; it never truncates or rewrites the prefix.
2. Resume selection comes from the observed failed step and accepted workflow graph, including nonconsecutive step identifiers and `u16::MAX` boundaries.
3. The next attempt is checked arithmetic; overflow is a typed rejection.
4. Every newly emitted action ticket and journal event uses the new attempt. Events carrying an earlier attempt are rejected.
5. Exactly one failed step is re-executed; preceding successful step outputs are recovered from durable authority.
6. Lifecycle validation occurs before mutation; any validation or recovery rejection leaves the journal unchanged.
7. The CLI reports success only after durable retry evidence is appended and execution is actually resumed.

## Boundaries and failure taxonomy

- CLI parser converts run ID and optional step selection into constrained values.
- Journal read and accepted-artifact validation are storage boundaries.
- Retry selection and attempt progression are deterministic calculations.
- Runtime recovery, append, and execution are mutation boundaries.
- Errors remain typed at the runtime/storage boundary and map to lifecycle, validation, or storage CLI exit codes without success-shaped output.

## Executable scenarios

- Given a persisted failed run with a durable accepted artifact and a retryable failed step, when the operator retries it, then the same run resumes at that observed step, the old event prefix is intact, retry evidence is appended, and new execution events use the next attempt.
- Given an active, cancelled, or finished run, when retried, then the command rejects and journal evidence is unchanged.
- Given a missing run, malformed journal, stale selected step, unavailable artifact, or non-resumable state, when retried, then the command rejects and journal evidence is unchanged.
- Given attempt `u16::MAX`, when retried, then checked overflow rejects before any journal mutation.
