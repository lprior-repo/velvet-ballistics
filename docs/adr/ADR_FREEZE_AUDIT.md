# ADR Freeze Audit

This audit records the review status for the `velvet-ballistics/v1` ADR set.

## Verdict

Status: acceptable as an architecture baseline, not as implementation proof.

The ADRs are coherent enough for implementation agents to use. Narrative docs have been reconciled for the known naming and current/deferred scope contradictions, but the master document still remains authoritative over every supporting doc.

## Consistent Decisions

1. Runtime trust is anchored on accepted artifacts, not YAML source.
2. Backend / IR Interpreter Complete is the active milestone.
3. Generated Rust, maxperf, PGO, and native UI are deferred or historical for current acceptance.
4. Runtime core excludes YAML interpretation, JSON routing, HTTP serving, dynamic string lookup, and unbounded resources.
5. The runtime is shard-owned, synchronous until suspension, and no-async in core crates.
6. Fjall and Postcard are required for current-scope durability records.
7. Evidence gates separate architecture acceptance from implementation completion.

## Sharp Edges

### 1. Narrative docs can drift again

Known stale product spelling, hyphenated crate names, draft language status, future-only IR wording, future-only recovery wording, and PGO current-gate wording have been reconciled. The remaining risk is recurrence, so ADR review gates must be run on future doc edits.

### 2. Recovery is still the highest-risk current-scope claim

The master drift register marks recovery partially resolved. Any public crash-safety statement requires end-to-end recovery evidence.

### 3. Idempotency is often overstated

The verifier can check key shape and action declarations. It cannot prove an external service honors idempotency. ADR-012 deliberately calls this attestation.

### 4. Raw submit paths exist for tests and can launder evidence

Tests that bypass runtime admission do not prove production admission. ADR-016 requires admission-bound evidence for production claims.

### 5. Deferred scope is noisy

The master preserves UI and codegen extension material, but current acceptance excludes it. ADR-021 is the fence.

### 6. Performance governance still needs evidence discipline

Master section 41 removes PGO and maxperf from current release gates. Any PGO task is future or advisory unless a future bead reopens it. Current performance claims still require measured IR-interpreter evidence.

## Required Follow-up Beads

1. Add an automated doc drift scan for these ADR gates.
2. Keep recovery evidence gaps tracked in implementation beads until pending-action recovery and strict acknowledgement behavior are closed by raw evidence.
