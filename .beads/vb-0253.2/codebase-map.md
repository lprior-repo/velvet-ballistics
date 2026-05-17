# Codebase Map - vb-0253.2

## Bead
**ID**: vb-0253.2  
**Title**: Finish ingress modularization and dedupe

## Scope

### Relevant Crates/Files
| Path | Role | Risk Tags |
|------|------|-----------|
| crates/vb_ipc/src/ingress.rs | MemoryIngress bounded queue implementation | concurrency, persistence |
| crates/vb_ipc/src/lib.rs | IPC module, re-exports IngressFrame, MemoryIngress | public-api |
| crates/vb_ipc/src/bounded.rs | Bounded types for ingress | |
| crates/vb_ipc/src/error.rs | IpcError types including ingress errors | error-handling |
| crates/vb_ipc/src/frame/tests.rs | Ingress frame tests | test |
| crates/vb_ipc/src/tests.rs | MemoryIngress tests | test |
| crates/workspace_tests/benches/velvet_ballastics.rs | Ingress benchmarks | performance |
| crates/velvet_ballastics/tests/cross_crate_adversarial.rs | Cross-crate IPC tests | test |

### Key Symbols
- `MemoryIngress::bounded(capacity)`
- `ingress.try_submit(frame)`
- `ingress.try_recv()`
- `IngressFrame`
- `IpcError::Full`
- `IpcError::Disconnected`

### Public API Surface
- `vb_ipc::IngressFrame`
- `vb_ipc::MemoryIngress`
- `vb_ipc::IpcError`

### Risk Tags
- **concurrency**: Multi-producer, single-consumer queue
- **persistence**: Ingress queue state across process boundaries
- **error-handling**: Full queue, disconnected sender error cases
- **performance**: Benchmarks exist for ingress throughput

### Open Questions
- What modularization remains to be done?
- What deduplication is needed?
- Are there duplicate implementations of similar bounded queues?

### Excluded Paths (Not in Scope)
- vb_runtime shard command queue (separate bead vb-0253.1)
- vb_proof_kernels StepState (separate bead vb-0253.5)

### Downstream Owners
- rust-contract: define ingress modularization contract
- proof-planner: Kani proofs for bounded queue properties
- test-planner: property-based tests for ingress
