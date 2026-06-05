# ADR 010 (v1): Whole-Workflow Boundedness

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

The compiler computes a conservative `WholeWorkflowBudget` before accepting any artifact. The budget is validated against both the workflow `ResourceContract` and global `BoundednessPolicy`.

## Invariants

- No accepted workflow has unknown bounds.
- Unbounded `for_each`, `collect`, `repeat`, `try_again`, `wait`, or `ask` is rejected according to master rules.
- Nested loops multiply bounds conservatively.
- Branches take maximum possible work.
- Parallel branches contribute to maximum in-flight work.

## Consequences

- Some practically terminating workflows are rejected if bounds cannot be proven.
- Conservative rejection is preferred to runtime resource surprise.

## Master Anchors

- Section 13: Resource Contracts
- Section 56: Runtime Profile Defaults
- Section 64: Whole-Workflow Boundedness Analysis
- Section 67: Architectural Drift Register, DRIFT-3
