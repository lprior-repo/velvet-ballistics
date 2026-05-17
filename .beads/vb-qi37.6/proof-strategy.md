# vb-qi37.6 Proof Strategy

State: 4 proof-planner attempt 3  
Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6`  
Source checkout writes: forbidden and not used.

## Planning Basis

- Repaired State 3 changed `proof-obligations.jsonl` for `INTEG-011`..`INTEG-014` from blocked placeholders to executable commands.
- State 6 rejection requires Kani/fuzz evidence repair, integration evidence execution, release gauntlet classification, and ledger ID normalization.
- This plan normalizes planned IDs to the primary State 3 ledger IDs: `VERUS-CAP-001`..`GATE-016`.
- No proof pass is claimed by this plan; all rows remain planned unless explicitly marked not required.

## Discovery Evidence

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6`.
- `test -s ".beads/vb-qi37.6/contract.md" && test -s ".beads/vb-qi37.6/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.6/delivery-scope.jsonl"` exited 0.
- Scoped risk discovery command ran over delivery-scope paths: `rg -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" <scope-paths>`.
- Scoped verifier discovery command ran over delivery-scope paths: `rg -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" <scope-paths>`.
- No discovery command was blocked.

## Risk-To-Verifier Routing

- Exact capability identity, profile cardinality, and accepted certificate preservation: Verus pure model plus Kani bounded implementation harnesses.
- Prefix, partial-prefix, action mismatch, empty name, and panic-freedom risks: Kani harnesses and schema fuzz lanes.
- Strict/Journaled admission lifecycle, denied allocation, no-contract dispatch, gate mismatch, and legacy bypass: TLA+ finite safety model.
- Storage/runtime/API/shard realization: exact integration commands from repaired `INTEG-011`..`INTEG-014`.
- UI projection: optional API compatibility test/review because UI is not release-critical for this bead.
- Release regression: `moon ci`, or formal classification of unrelated pre-existing global debt.

## Known Blockers For Later States

- Prior Kani commands timed out; proof-writer must either make them tractable or split focused harness obligations with explicit mapping before claiming PASS.
- Prior cargo-fuzz commands failed on sanitizer/static-libc conflict; proof-writer or implementation state must repair tooling/config or record an approved waiver before claiming PASS.
- `INTEG-011`..`INTEG-014` now have executable commands but are not run by State 4.
- `GATE-016` requires `moon ci` or formal verifier debt classification; State 4 does not execute it.
