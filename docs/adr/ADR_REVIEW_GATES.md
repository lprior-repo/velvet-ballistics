# ADR Review Gates

Run these checks before landing ADR or master-decomposition changes.

## Manual Contract Checks

1. Every ADR cites master sections.
2. Every new ADR appears in `ADR_DEPENDENCY_GRAPH.md`.
3. Every new ADR appears in `ADR-TO-VERIFICATION-TRACEABILITY-MATRIX.md`.
4. Any new current-scope claim appears in `ADR_FREEZE_AUDIT.md` if it changes freeze status.
5. Any deferred-scope reference is labeled deferred, historical, or future-only.
6. No ADR claims implementation completion without command evidence.

## Suggested Mechanical Checks

```bash
rg -n "Velvet Ballastics|Velvet-ballastics|velvet-ballastics|vb-core|vb-runtime|vb-storage|vb-ipc|vb-compiler" docs
rg -n "generated Rust|maxperf|PGO|target-cpu=native|Makepad|UI" docs/adr docs/master-decomposition.md
rg -n "implementation complete|proved|verified|benchmark proves|crash safe|production ready" docs/adr docs/master-decomposition.md
rg -n "Status: draft|TODO|placeholder|Future|Upcoming" docs/adr docs/master-decomposition.md
```

The first command is expected to find known drift in older docs until cleanup beads close. It must not find stale naming inside new ADR files unless the text explicitly labels a migration or drift finding.

## Review Failure Conditions

Reject the ADR change if any condition holds:

1. It conflicts with `velvet-ballistics-MASTER.md`.
2. It reintroduces generated Rust, maxperf, PGO, native CPU, or UI as current backend blockers.
3. It describes raw `CompiledWorkflow` submit tests as production admission evidence.
4. It uses proof/model/tool-version output as behavior evidence without production binding.
5. It omits boundedness, capability, idempotency, taint, durability, or diagnostics from a decision that touches those domains.
6. It adds a decision but does not update dependency and traceability docs.
