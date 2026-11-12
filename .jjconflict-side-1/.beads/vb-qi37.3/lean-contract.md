# Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: recovery/resume ordering, durable-before-resume, per-run/per-slot isolation over time, and duplicate/stale/out-of-order rejection as temporal state transitions.
- Verus-owned Rust core: cursor/page bounds, state key equality, decode identity validation, state-transition preservation, and error taxonomy totality for collect page ordering.
- Theorem-owned kernel: none required at State 3.
- Rust/runtime shell: `RunFrame`, `ValueStore`, journal adapters, shard queue, wall-clock time, Fjall persistence, and evidence collector I/O are excluded from Lean theorem proving.
- External systems excluded: Fjall, Moon, cargo-nextest, postcard runtime implementation, timers, and OS clock.

## Theorem-Owned Clauses
- None. The Rust-local algebra is small enough for Verus obligations once proof surfaces exist. TLA+ owns temporal workflow behavior.

## Theorem Obligations
- No Lean/Aeneas/Hax proof obligation is required for this bead unless State 4 finds that the collect-extra tagged codec/refinement cannot be expressed in Verus.

## Waivers
- THM-WAIVER-001: Lean/Aeneas/Hax waived for all clauses. Owner: State 4 reviewer. Reason: no tiny theorem kernel beyond Verus/TLA+ has been identified; all critical algebraic clauses are finite state transitions, bounds, identity checks, or temporal workflow properties. Expiry: revisit if State 4 introduces a custom binary extra envelope whose tag/refinement cannot be covered by Verus/proptest/fuzz. Compensating evidence: Verus obligations for core transitions, TLA+ temporal obligation, codec fuzz/property obligations, and exact runtime/storage recovery scenarios.
