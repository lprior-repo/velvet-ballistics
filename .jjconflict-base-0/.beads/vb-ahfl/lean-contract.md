# Theorem Kernel Projection: vb-ahfl

## Boundary

- TLA+-owned temporal model: waived for the State 2 UI schema parity scope.
- Verus-owned Rust core: metadata completeness, bounded collection invariants, redaction projection, graph/event structural validity, and canonicalization determinism.
- Theorem-owned kernel: none required at contract time for accepted UI-scope artifacts; `BLOCKER-SCOPE-001` is resolved here by explicit State 2 delivery scope. If owner/orchestrator selects engine YAML-to-IR semantics instead, regenerate State 3 before implementation consumption.
- Rust/runtime shell: CLI JSON/JSONL emission, Makepad rendering, filesystem, clocks, and runtime data acquisition.
- External systems excluded from theorem proof: CLI process execution, Makepad UI rendering, wall-clock generation, runtime storage, and source control.

## Theorem-Owned Clauses

- None.

## Rationale

The scoped properties are first-order data-shape and refinement obligations that Verus can express directly over Rust-local pure functions and production-bound harnesses. A Lean/Aeneas/Hax theorem kernel would add toolchain and extraction burden without a smaller proof surface. If later implementation introduces a mathematically nontrivial canonicalization algebra that Verus cannot express cleanly, or if the owner/orchestrator selects engine YAML-to-IR semantics, State 3 must be amended or regenerated with a tiny theorem kernel before proof writing.

## Waivers

- WAIVED-LEAN-001
  - Clauses: INV-001 through INV-006, POST-001 through POST-007.
  - Owner: State 3 rust-contract, pending independent contract verification review.
  - Reason: Verus is the primary proof layer and is sufficient for the current pure data invariants.
  - Expiry: before proof writing if Verus target discovery finds an invariant requiring theorem-assistant extraction.
  - Compensating evidence: Verus obligations plus Kani/proptest parity and bounds exploration.
