bead_id: vb-6r5
phase: 4
updated_at: 2026-05-18T02:00:00Z

# Proof Strategy - State 4

## Verifier Lane Strategy

This is a CLI tooling bead (xtask orchestrator), not safety-critical runtime code. Proof obligations are limited to algorithmic correctness properties best verified through unit tests and property tests rather than formal verification.

### Risk Tags -> Verifier Mapping
- `MEDIUM:parallel_execution` -> Unit tests + property tests (proptest for DAG scheduling)
- `LOW:cli_parsing` -> Unit tests (clap derive handles most correctness)
- `LOW:jsonl_logging` -> Unit tests
- `MEDIUM:tool_availability` -> Unit tests (mock tool detection)

### Verifier Lanes
1. **Unit tests** (required): CLI parsing, DAG scheduling, lane selection, profile filtering
2. **Property tests** (required): DAG topological sort correctness, bounded parallelism invariants
3. **Kani** (deferred): Not applicable — no unsafe code, no arithmetic requiring bounded model checking
4. **Miri** (deferred): Not applicable — no raw pointers, no unsafe code
5. **TLA+** (deferred): Not applicable — scheduler is single-process, not distributed
6. **Verus** (deferred): Not applicable — no safety-critical invariants requiring deductive proof

### Waiver Candidates
- Kani, Miri, TLA+, Verus: Tooling bead with no unsafe code, no distributed protocols, no arithmetic overflow risk beyond standard Rust checks.

## Proof Plan
- P1: DAG scheduler produces valid topological order (property test)
- P2: No crate scheduled before its dependencies (property test)
- P3: Parallel job count never exceeds --jobs bound (unit test)
- P4: CLI rejects invalid --jobs values (unit test)
- P5: Profile lane selection is monotonic (fast ⊆ standard ⊆ deep ⊆ all) (unit test)
