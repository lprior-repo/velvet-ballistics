# Proof Strategy: vb-core-cli-accepted-path State 4 Attempt 3

## Scope

- Planning state only: no production, test, verifier model, harness, dependency, or config edits.
- Inputs: repaired State 3 `contract.md`, `verification-layers.md`, `tla-spec.md`, `lean-contract.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`; State 6 rejection artifacts `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`; prior proof evidence as context only.
- Repaired State 3 obligation IDs remain canonical and are mapped to stable State 4 IDs in planned obligations.

## Discovery Evidence

- `pwd -P` -> `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path`.
- `test -s .beads/vb-core-cli-accepted-path/contract.md` -> exit 0.
- `test -s .beads/vb-core-cli-accepted-path/traceability-matrix.jsonl` -> exit 0.
- `test -s .beads/vb-core-cli-accepted-path/delivery-scope.jsonl` -> exit 0.
- `/usr/bin/rg -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" <delivery-scope paths>` -> exit 0; matches include strict runtime state/admission/queue code, parser/codec serialization, tests, Kani harnesses, and forbidden-token occurrences mostly in tests/harnesses.
- `/usr/bin/rg -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" <delivery-scope paths>` -> exit 0; matches include existing Kani harnesses and accepted artifact admission tests.

No discovery command was blocked.

## Risk To Verifier Mapping

- Temporal/persistence/release-critical ordering: `PO-001` TLA+ must prove safety plus configured liveness/deadlock handling. Prior safety-only TLC output is insufficient.
- Digest identity binding: `PO-002` Verus plus later proptest/integration evidence.
- Strict raw compiled bypass and storage-backed witness typing: `PO-003` Verus plus integration/static scan.
- Typed invalid artifact rejection before admission, acknowledgement, or run-state insertion: `PO-004` Verus must be strengthened from the rejected tautological model.
- Bounded malformed decode/admission/bypass state space: `PO-007` remains required, but the current aggregate `moon run :verify-proof` lane is known blocked by tooling until repaired or reviewer-approved waived.
- Runtime shell behavior: integration/manual QA, static scan, API compatibility, fuzz/proptest, and canonical CI remain later-state obligations.
- Lean/Aeneas/Hax theorem kernel: explicitly waived at contract time by State 3 unless accepted artifact format introduces a theorem-level proof lattice.

## State 3 To State 4 Mapping

- `TLA-ACCEPT-001` -> `PO-001`.
- `VERUS-DIGEST-001` -> `PO-002`.
- `VERUS-POLICY-001` -> `PO-003`.
- `VERUS-ADMISSION-001` -> `PO-004`.
- `KANI-ADMISSION-001` -> `PO-007`.

## Reviewer Notes

- The planned ledger does not claim any verifier pass.
- `PO-001`, `PO-004`, and `PO-007` explicitly carry the State 6 repair requirements.
- `PO-007` is marked `blocked_tooling` rather than silently omitted because State 6 rejected the prior bundle for missing required Kani/aggregate evidence.
