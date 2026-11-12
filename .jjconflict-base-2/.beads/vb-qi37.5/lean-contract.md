# Theorem Kernel Projection: vb-qi37.5

## Boundary

- TLA+-owned temporal model: retry/replay/admission lifecycle, duplicate/stale completion ordering, terminal admission/rejection.
- Verus-owned Rust core: decision-table totality/parity, certificate-summary soundness, replay tracker monotonicity.
- Theorem-owned kernel: none required at State 3.
- Rust/runtime shell: I/O, Fjall/Postcard persistence, filesystem, wall-clock, runtime scheduling, CLI rendering.
- External systems excluded from theorem proof: durable storage engine internals, OS failures, serialization byte layout, terminal output.

## Theorem-Owned Clauses

- None currently. Verus and TLA+ are the required formal layers.

## Theorem Obligations

- THM-WAIVE-001: No Lean/Aeneas/Hax module is planned for this bead unless Verus cannot express the certificate-summary refinement from accepted contracts to `VerificationProof` fields.

## Waivers

- Clause: all theorem-kernel clauses.
- Owner: State 3 rust-contract; reviewer must confirm.
- Reason: no algebraic theorem kernel beyond Verus/TLA+ has been identified. The core properties are finite decision-table classification, set/list summary soundness, and monotonic replay state.
- Expiry: before State 5 proof writing starts.
- Compensating evidence: Verus obligations `VERUS-DECISION-001`, `VERUS-CERT-001`, and `VERUS-REPLAY-001`; TLA+ obligations `TLA-RETRY-001`, `TLA-REPLAY-002`, `TLA-ADMIT-003`; Kani/proptest obligations for decision-table parity.
