# Proof Plan Review Input: vb-qi37.12.4

## Review Request

Review State 4 proof planning after repaired State 3. This plan intentionally keeps the canonical `GATE-*` ID namespace from `proof-obligations.jsonl` so State 5/6 can trace exact obligations to raw evidence.

## Files To Review

- `.beads/vb-qi37.12.4/proof-strategy.md`
- `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.12.4/contract.md`
- `.beads/vb-qi37.12.4/proof-obligations.jsonl`
- `.beads/vb-qi37.12.4/traceability-matrix.jsonl`
- `.beads/vb-qi37.12.4/verification-layers.md`
- `.beads/vb-qi37.12.4/tla-spec.md`
- `.beads/vb-qi37.12.4/lean-contract.md`
- State 6 rejection artifacts: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`

## Must Check

- Every required executable clause from repaired State 3 appears in planned obligations with the same canonical `GATE-*` ID.
- Every row has `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, and `waiver`.
- Planned rows do not claim pass results.
- Waived and not-applicable rows include explicit rationale and expiry or trigger where relevant.
- Verus waiver is acceptable only while no Rust-local classifier/parser/exception-validator exists.
- Later proof evidence must disposition `GATE-*` IDs directly, not only `PO-*` aliases.

## Known Blockers For Later States

- `scripts/check-ignored-fallible-results.sh` is not currently executable in this workspace; State 5 previously observed `test -x "scripts/check-ignored-fallible-results.sh"` exit 1. This State 4 plan does not mark any executable gate obligation as passed.
- Negative fixture commands cannot produce approval evidence until the direct gate and fixture mechanism exist.

## Discovery Commands Run

- `pwd -P`
- `test -s ".beads/vb-qi37.12.4/contract.md" && test -s ".beads/vb-qi37.12.4/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.12.4/delivery-scope.jsonl"`
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates xtask/src .moon/tasks/all.yml Cargo.toml scripts`
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates xtask/src .moon/tasks/all.yml Cargo.toml scripts`

No discovery command was blocked.
