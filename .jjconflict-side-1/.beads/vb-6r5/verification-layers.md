bead_id: vb-6r5
phase: 3
updated_at: 2026-05-18T02:00:00Z

# Verification Layers

## Defense-in-Depth Assignment

| Contract Clause | Primary Verifier | Secondary Verifier | Rationale |
|---|---|---|---|
| CLI commands | Unit tests | — | clap derive handles parsing correctness |
| Profile lane selection | Unit tests | — | Static enum matching, trivially verified |
| DAG topological order | Property tests (proptest) | — | Random graph generation covers edge cases |
| Dependency ordering | Property tests (proptest) | — | Same as above |
| Bounded parallelism | Unit tests | — | Deterministic scheduler simulation |
| Structured logging | Unit tests | — | Serialization correctness via serde |
| Workspace discovery | Unit tests | — | cargo metadata parsing |
| CLI flags | Unit tests | — | clap derive + validation |
| Exit code behavior | Integration tests | — | Process exit code verification |

## Waiver Justification
- Kani/Miri: No unsafe code, no raw pointers, no arithmetic requiring bounded model checking
- TLA+: Single-process CLI, not a distributed protocol
- Verus: No safety-critical invariants requiring deductive proof
- Fuzz: CLI input is structured (clap handles), no untrusted binary input
