# Proof Strategy: vb-qi37.5 State 4 Attempt 3

STATUS: PLANNED

## Scope

- Planning refresh after repaired State 3 artifacts and State 6 rejection feedback.
- Planning artifacts only: no production code, tests, proof files, models, harnesses, specs, dependencies, or config were edited.
- Source checkout `/home/lewis/src/velvet-ballistics` was not written.

## Inputs Read

- Repaired State 3: `contract.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `verification-layers.md`, `delivery-scope.jsonl`, `codebase-map.md`.
- State 6 rejection: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`.
- Prior evidence only as context: `proof-writer-report.md`, `proof-evidence.md`.

## Discovery Evidence

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`.
- `test -s ".beads/vb-qi37.5/contract.md" && test -s ".beads/vb-qi37.5/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.5/delivery-scope.jsonl"` exited 0.
- Risk discovery over State 2 delivery paths found retry/replay/state/transition/queue/serialization/assertion surfaces in `vb_core`, `vb_validate`, `vb_compile`, `vb_storage`, and `vb_runtime`.
- Verifier discovery found Kani harnesses in `vb_core`, `vb_validate`, and `vb_compile`; fuzz symbols in `fuzz/fuzz_targets.rs`; no scoped Loom, Flux, or TLA implementation surface in production paths.
- No discovery command was blocked.

## Strategy

- Treat repaired `proof-obligations.jsonl` as the source contract obligation set, but tighten planned evidence to answer State 6 findings.
- TLA+ obligations must require deadlock-enabled TLC evidence and model action ID, run ID, tickets/keys, digests, duplicate attempts, admission evidence, and terminal stutter without `CHECK_DEADLOCK FALSE`.
- Verus parity may not be tautological. It must either model production `vb_compile::check_idempotency_gates` faithfully or document an approved extraction/refinement boundary.
- Kani parity must enumerate all `SideEffect x RetrySafety x Idempotency` combinations and must not use `kani::assume(!excluded)` for known disagreement cases.
- Certificate proof must use action identifiers or finite sets/sequences and prove no over-reporting plus no default-empty omission for qualifying keyed/attested contracts.
- Fuzz obligation is executable after State 3 repair: `cargo fuzz run admission_fuzz -- -runs=1000`.
- POST-006 requires both temporal proof and runtime realization evidence via `TEST-COMPLETION-015`.

## Waivers And Non-Applicable Lanes

- Lean/Aeneas/Hax: waived because State 3 theorem-owned clauses remain empty and Verus/TLA+ are sufficient first-line tools.
- Loom/Shuttle: not applicable unless later implementation introduces shared-memory concurrency in this bead scope.
- Flux: not applicable for this plan because no refinement annotation surface is present and prior tool discovery found `cargo flux` unavailable.
- Dependency/supply-chain: not applicable unless later states edit dependency/config files.

## Handoff

- Proof writer must repair prior TLA+/Verus/Kani artifacts rather than reusing State 5 evidence as proof of repaired obligations.
- Formal verifier must run the commands exactly as planned or record `blocked_tooling` rows without inventing pass results.
