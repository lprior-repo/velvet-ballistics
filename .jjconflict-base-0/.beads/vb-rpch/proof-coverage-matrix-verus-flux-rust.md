# Proof Coverage Matrix — vb-rpch Verus/Flux/Rust

| Clause | Risk | Verus | Flux RS | Production Rust / Holzman | Notes |
|---|---|---|---|---|---|
| INV-002 | Rust-local invariant | Required: `VFR-VERUS-001` | BLOCKED_TOOLING: `VFR-FLUX-001` | Required: `VFR-RUST-ATTACH-001` | `SUPPORTED` all false; `union` algebra. |
| INV-003 | Bounded arithmetic | Required: `VFR-VERUS-004` | BLOCKED_TOOLING: `VFR-FLUX-002` | Required: `VFR-RUST-ATTACH-004` | Positive dimensions on successful non-empty/evidence-bearing seed paths. |
| INV-004 | Rust-local invariant | Required: `VFR-VERUS-002` | BLOCKED_TOOLING: `VFR-FLUX-003` | Required: `VFR-RUST-ATTACH-002` | Tracker resolution monotonicity. |
| INV-005 | Rust-local invariant | Required: `VFR-VERUS-003` | BLOCKED_TOOLING: `VFR-FLUX-004` | Required: `VFR-RUST-ATTACH-003` | Explicit digest-level rank/inclusion. |
| PRE-001 | Untrusted input / bounds | Required: `VFR-VERUS-005` | BLOCKED_TOOLING: `VFR-FLUX-005` | Required: `VFR-RUST-ATTACH-005` | Snapshot/tail preconditions and nonzero dimensions. |
| PRE-002 | Untrusted input / bounds | Required: `VFR-VERUS-006` | BLOCKED_TOOLING: `VFR-FLUX-006` | Required: `VFR-RUST-ATTACH-006` | Empty events and successful dimension checks. |
| POST-009 | Temporal/state-machine local Rust refinement | Required: `VFR-VERUS-007` | BLOCKED_TOOLING: `VFR-FLUX-007` | Required: `VFR-RUST-ATTACH-007` | TLC round-3 approved bounded abstraction remains preserved; Verus/Rust must bind local implementation. |

Flux blocker evidence: `cargo flux --version` in `/home/lewis/src/vb-jpq7-jj-fix` returned `error: no such command: flux`.

Verus availability evidence: `command -v verus && verus --version` returned `/home/lewis/.local/bin/verus` and Verus `0.2026.05.05.d03e906`.
