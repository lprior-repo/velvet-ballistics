# Proof Strategy: vb-qi37.12.4

## Status

- State: 4 proof-planner repair, attempt 3.
- Scope: planning only. No production code, tests, proof/model/harness/specs, dependencies, or CI config were edited.
- Basis: repaired State 3 contract bundle after State 6 rejection.
- ID policy: use canonical `GATE-*` IDs from repaired `proof-obligations.jsonl`; do not emit a separate unmapped `PO-*` namespace.

## Inputs Read

- `.beads/vb-qi37.12.4/contract.md`
- `.beads/vb-qi37.12.4/proof-obligations.jsonl`
- `.beads/vb-qi37.12.4/traceability-matrix.jsonl`
- `.beads/vb-qi37.12.4/verification-layers.md`
- `.beads/vb-qi37.12.4/tla-spec.md`
- `.beads/vb-qi37.12.4/lean-contract.md`
- `.beads/vb-qi37.12.4/delivery-scope.jsonl`
- `.beads/vb-qi37.12.4/codebase-map.md`
- State 6 rejection artifacts: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`
- Prior proof context: `proof-evidence.md`, `proof-writer-report.md`, `formal-verification-report.md`

## Discovery Evidence

- `pwd -P` -> exit 0; output `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4`.
- `test -s ".beads/vb-qi37.12.4/contract.md" && test -s ".beads/vb-qi37.12.4/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.12.4/delivery-scope.jsonl"` -> exit 0.
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates xtask/src .moon/tasks/all.yml Cargo.toml scripts` -> exit 0; found broad existing workspace risk terms, including `unsafe_code = "forbid"`, generated/test assertions, Kani/proptest/test references, retry/queue code outside this bead's gate contract, and no new State 4 proof target.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates xtask/src .moon/tasks/all.yml Cargo.toml scripts` -> exit 0; found existing verifier/proof terms in workspace code, but no bead-local Rust classifier, parser, exception-validator, temporal model, theorem kernel, concurrency primitive, unsafe/FFI target, or dependency change introduced by State 3.

No discovery command was blocked.

## Risk Classification

- Primary risk: static quality gate can silently miss ignored fallible results or accept overbroad exceptions.
- Required executable evidence: direct gate command, DISCARD-001 through DISCARD-006 negative evidence, scan-domain evidence, exception-validation evidence, deterministic rerun evidence, fail-closed evidence, canonical Moon path, and lint hard-deny path.
- Formal waiver posture: TLA+, Verus, Lean/Aeneas/Hax, Kani, Flux, Loom, Miri, proptest, fuzz, and supply-chain lanes are not current proof-writing targets unless later implementation introduces their trigger conditions.
- Verus trigger: if State 8/11 introduces Rust-local deterministic classifier or exception-validation logic, contract repair must add Verus-first obligations before approval.

## Planned Verifier Lanes

- `manual-qa`/`static-scan`: required for direct gate behavior and fixture evidence.
- `moon`: required for `moon run :verify-standard` propagation.
- `clippy`: required for `moon run :lint-src` hard denial of ignored must-use values.
- `tla-plus`: waived for current scope because there is no temporal behavior.
- `verus`: waived for current scope because there is no Rust-local classifier/validator artifact; expiry is explicit.
- `lean/aeneas/hax`: waived for current scope because there is no theorem-critical kernel.
- `kani`, `flux`, `loom`, `miri`, `proptest`, `fuzz`: not applicable or waived until implementation adds corresponding risk surfaces.

## Evidence Required Later

- `machine-gate-report.md` must include raw command output, exit statuses, and fixture paths for every required canonical `GATE-*` obligation.
- `proof-evidence.md` must disposition each canonical `GATE-*` ID exactly; `PO-*` aliases are not sufficient.
- No PASS may be claimed without raw executable evidence.
