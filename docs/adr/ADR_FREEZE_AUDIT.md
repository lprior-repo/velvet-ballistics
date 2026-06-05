# ADR Freeze Audit

This audit records the review status for the `velvet-ballistics/v1` ADR set.

## Verdict

Status: acceptable as an architecture baseline, not as implementation proof.

The ADRs are coherent enough for implementation agents to use, but several repository docs still drift from the master contract and must not be treated as authoritative.

## Consistent Decisions

1. Runtime trust is anchored on accepted artifacts, not YAML source.
2. Backend / IR Interpreter Complete is the active milestone.
3. Generated Rust, maxperf, PGO, and native UI are deferred or historical for current acceptance.
4. Runtime core excludes YAML interpretation, JSON routing, HTTP serving, dynamic string lookup, and unbounded resources.
5. The runtime is shard-owned, synchronous until suspension, and no-async in core crates.
6. Fjall and Postcard are required for current-scope durability records.
7. Evidence gates separate architecture acceptance from implementation completion.

## Sharp Edges

### 1. Existing docs still contain stale product and crate names

`docs/runtime-architecture.md`, `docs/language-spec.md`, and several related docs use old product spelling or hyphenated crate names. The ADRs use canonical module names, but the old docs need cleanup.

### 2. Recovery is still the highest-risk current-scope claim

The master drift register marks recovery partially resolved. Any public crash-safety statement requires end-to-end recovery evidence.

### 3. Idempotency is often overstated

The verifier can check key shape and action declarations. It cannot prove an external service honors idempotency. ADR-012 deliberately calls this attestation.

### 4. Raw submit paths exist for tests and can launder evidence

Tests that bypass runtime admission do not prove production admission. ADR-016 requires admission-bound evidence for production claims.

### 5. Deferred scope is noisy

The master preserves UI and codegen extension material, but current acceptance excludes it. ADR-021 is the fence.

### 6. Existing performance governance still mentions PGO tasks

Master section 41 removes PGO and maxperf from current release gates. Any PGO task is future or advisory unless a future bead reopens it.

## Required Follow-up Beads

1. Clean existing docs for canonical product spelling and crate names.
2. Reconcile `docs/language-spec.md` with current backend scope and deferred UI status.
3. Update `docs/compiled-ir.md` so current expression/action IR features are not described as future.
4. Update `docs/storage-journal.md` so recovery is not described as future-only.
5. Remove or quarantine PGO current-gate language from `docs/rust-governance.md`.
6. Add an automated doc drift scan for these ADR gates.
