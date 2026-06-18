# Exhaustive Sweep

Generated 2026-05-24. This summarizes the subsystem sweep used to create the `vb-8mdp` backlog.

## Sweep Domains

1. Runtime execution and lifecycle tests: delayed actions, retry generation, kill/cancel outcomes, idempotency, watermarks, fairness, bounded resources.
2. Storage and recovery tests: envelope decode order, Fjall key prefixes, journal side indexes, recovery watermarks, snapshots, explicit migrations, tail scans, corruption fixtures.
3. IPC and protocol tests: magic/header validation, reserved flags, frame fragmentation, typed decode errors, shell-only identity/capability boundaries.
4. Core domain-type tests: partitioned numeric IDs, retry/deadline math, Postcard newtype compatibility.
5. CLI/doctor tests: cold bounded storage scans, projections, safe filters, derived replay timelines, typed diagnostics.

## Master Filter

Only invariants compatible with the Master contract survived. The following ideas are rejected for velvet-ballistics v1: distributed rule updates, HLC clocks, HTTP/gRPC runtime coupling, runtime JSON/YAML interpretation, implicit migrations, unbounded queues, hot string maps, task-per-step async orchestration, and implementation-body copying.

## Planner Output

The planner expansion produced 29 child beads under `vb-8mdp`. Black Hat review required repair before acceptance: canonical naming cleanup, executable acceptance criteria, duplicate/narrowing decisions, and explicit no-copy fences.
