# Workflow Model — vb-xi2f.33: Digest Covers Ask Semantics

## Digest Computation Workflow

### States

```
┌──────────────────┐
│  WorkflowSource  │  (YAML parsed, validated)
│  (Start)         │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  canonical       │
│  _digest start   │  Initialize blake3::Hasher
└────────┬─────────┘
         │
         ▼
┌──────────────────┐     ┌───────────────┐
│  Hash version    │────▶│ Hash name     │
└──────────────────┘     └───────┬───────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │  Hash trigger    │  Version + name + trigger
                        └────────┬─────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │  For each step:  │
                        │  - Hash step.id  │  Loop over steps
                        │  - Hash prim.    │
                        └────────┬─────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │  finalize()      │  Complete blake3
                        └────────┬─────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │  WorkflowDigest  │  Output
                        │  (End)           │
                        └──────────────────┘
```

### Ask Primitive Hash Sub-Workflow

```
┌──────────────────────────┐
│  StepPrimitive::Ask      │
│  { prompt, timeout }     │
└───────────┬──────────────┘
            │
            ▼
┌──────────────────────────┐
│  hasher.update(b"ask")   │  Primitive tag
└───────────┬──────────────┘
            │
            ▼
┌──────────────────────────┐
│  hasher.update(          │  Hash prompt bytes
│    prompt.as_bytes())    │  (empty string OK)
└───────────┬──────────────┘
            │
       ┌────┴────┐
       │ timeout? │
       └────┬────┘
            │
     ┌──────┴──────┐
     ▼             ▼
┌─────────┐  ┌──────────────┐
│ Some(t) │  │ None         │
└────┬────┘  └──────┬───────┘
     │              │
     ▼              ▼
┌─────────┐  ┌──────────────┐
│ update  │  │ update       │
│ "timeout│  │ "no_timeout" │
│ " +     │  │              │
│ t.bytes │  │              │
└────┬────┘  └──────┬───────┘
     │              │
     └──────┬───────┘
            │
            ▼
     (return to caller)
```

## State Transitions (current BUGGY behavior)

| Current State | Input | Actual Transition | Expected Transition |
|---------------|-------|-------------------|---------------------|
| `digest_step_primitive` sees `Ask{prompt, timeout}` | Any prompt, any timeout | `hasher.update(b"ask")` ONLY | `hasher.update(b"ask")` + `hasher.update(prompt)` + `hasher.update(timeout\|sentinel)` |

## Post-Fix Transitions

| State | Guard | Action | Next State |
|-------|-------|--------|------------|
| `Ask { prompt, timeout }` | Always | `hasher.update(b"ask")` | → hash_prompt |
| hash_prompt | Always | `hasher.update(prompt.as_bytes())` | → hash_timeout |
| hash_timeout | `timeout = Some(t)` | `hasher.update(b"timeout"); hasher.update(t.as_bytes())` | → done |
| hash_timeout | `timeout = None` | `hasher.update(b"no_timeout")` | → done |

## Workflow Invariants

### WF-INV-001: Deterministic Path
The sequence `canonical_digest(source)` → `digest_step_primitive` → hash finalization MUST produce identical output for identical input. No randomness, no timestamp, no platform dependency.

### WF-INV-002: Step Independence
The digest contribution of step N is independent of step N+1. The hasher accumulates via `update()` which is order-sensitive but each step's contribution is self-contained. Changing step order changes the digest.

### WF-INV-003: Field Order Determinism
Within the Ask hash, field order is fixed: tag → prompt → timeout. No iteration over HashMap or other unordered collection.

### WF-INV-004: All Ask Fields Consumed
Every field of `StepPrimitive::Ask` (prompt, timeout) MUST be consumed by the hasher. No field may be skipped.

## Termination Guarantees

- `canonical_digest` always terminates: finite number of steps, finite string lengths, blake3 always terminates.
- `digest_step_primitive` with proper `Ask` arm always terminates: no recursion, no loops within the Ask arm.

## Rollback / Error Paths

- `canonical_digest` is infallible (returns `WorkflowDigest`, not `Result`). No error paths in the digest computation itself.
- If the source is invalid (e.g., fails YAML parsing), `canonical_digest` is never called — parsing fails first.
- The digest computation has no side effects, so no rollback is needed.

## Retry Semantics

- Not applicable. `canonical_digest` is a pure computation. Retrying produces the same result.

## Idempotence

- `canonical_digest(S)` is idempotent by determinism: calling it twice on the same `S` produces the same digest.

## Concurrency

- `canonical_digest` takes `&WorkflowSource` (shared reference). It reads but does not mutate the source.
- `digest_step_primitive` takes `&mut blake3::Hasher` but the hasher is not shared across threads.
- No concurrency hazards in the digest computation itself. The duplicate implementations (`part_05.rs` and `compile/mod.rs`) are called from separate compilation paths that should not run concurrently, but even if they did, they operate on independent hashers.

## Cancellation

- Not applicable. `canonical_digest` performs no I/O, has no await points, and cannot be meaningfully cancelled mid-computation.
