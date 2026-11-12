bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Verification Layers

## Layer Assignment by Contract Clause

| Clause | Unit Tests | Integration Tests | Property Tests | Kani | Miri | Fuzz | Coverage |
|--------|-----------|-------------------|----------------|------|------|------|----------|
| PRE-001 (db path valid) | X | X | | | | | X |
| PRE-002 (run_id parseable) | X | | X | | | X | X |
| PRE-003 (reason length) | X | | X | X | | | X |
| POST-001 (journal persisted) | | X | | | | | X |
| POST-002 (run removed) | X | X | | | | | X |
| POST-003 (trace pushed) | X | X | | | | | X |
| POST-004 (counter once) | X | X | X | | | | X |
| POST-005 (no-op for missing) | X | X | X | | | | X |
| POST-006 (structured output) | X | X | | | | | X |
| POST-007 (human output) | X | | | | | | X |
| INV-001 (any non-terminal) | X | X | | | | | X |
| INV-002 (terminal no-op) | X | X | X | | | | X |
| INV-003 (no duplicate journal) | | X | X | | | | X |
| INV-004 (reason preserved) | X | X | | | | | X |
| INV-005 (idempotent at all layers) | X | X | X | | | | X |

## Verification Layer Definitions

### Unit Tests
- Target: Individual functions (parsing, event encoding, shard handle_cancel).
- Tool: `cargo test` within each crate.
- Owner: test-writer sub-agent.

### Integration Tests
- Target: End-to-end CLI invocation through runtime to journal read-back.
- Tool: `vb_runtime/tests/durability_matrix_integration.rs`, CLI inline tests.
- Owner: test-writer sub-agent.

### Property Tests
- Target: Idempotency, reason length boundaries, run_id parsing.
- Tool: `proptest` where applicable.
- Owner: test-writer sub-agent.

### Kani
- Target: Pure functions (reason length validation, run_id parsing, event encoding).
- Tool: `cargo kani`.
- Owner: formal-verifier sub-agent.
- **Waiver**: Kani does not apply to CLI I/O or Fjall storage operations.

### Miri
- Target: Not required for this bead (no new unsafe code, no complex pointer manipulation).
- Tool: `cargo miri test`.
- Owner: formal-verifier sub-agent.
- **Waiver**: Not applicable; no unsafe code introduced.

### Fuzz
- Target: IPC payload decoding for `CancelRun` with optional reason.
- Tool: `cargo fuzz` or Bolero if configured.
- Owner: formal-verifier sub-agent.
- **Waiver**: Fuzz target not yet scaffolded in repo; defer to existing IPC adversarial tests.

### Coverage
- Target: Line coverage for all modified files.
- Tool: `llvm-cov` via `cargo tarpaulin` or `moon run :coverage`.
- Owner: qa-enforcer sub-agent.

## Defense-in-Depth Summary

1. **Parse-time validation** (CLI): Rejects invalid run_id and oversized reason before touching runtime.
2. **Runtime queue bounds**: Cancel command enqueue respects shard queue capacity.
3. **Shard idempotency**: Duplicate cancels are silently dropped without journal duplication.
4. **Journal durability**: Fjall WriteBatch ensures atomic persistence of cancel event.
5. **Structured output**: JSON/JSONL output enables automated verification of cancel success.
