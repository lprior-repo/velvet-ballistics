bead_id: vb-6r5
phase: 3
updated_at: 2026-05-18T01:45:00Z

# Domain Model Review

## Analysis
This is a CLI tooling bead, not a domain-model-heavy feature. The domain model is straightforward:

- **CrateInfo**: Represents a workspace crate with its dependencies and available proof lanes
- **Lane**: Represents a test/proof command with metadata (timeout, required flag, profile membership)
- **Scheduler**: DAG-based executor that respects crate dependencies and bounded parallelism
- **Logger**: Structured JSONL writer for per-run, per-crate, per-lane results

## Illegal States
- A lane cannot be both required and optional simultaneously (enforced by type)
- A crate cannot depend on itself (validated during discovery)
- A run-id must be unique (timestamp + random suffix)
- Parallel job count must be >= 1 (validated at CLI parse time)

## Type Boundaries
- All CLI input is validated before entering the scheduler
- Tool availability is detected once at startup, not per-lane
- cargo metadata is parsed into CrateInfo structs, not raw JSON

## DDD Assessment
No Scott Wlaschin refactoring needed. The domain is simple enough that the type model above makes illegal states unrepresentable without additional complexity.

STATUS: ACCEPTED
