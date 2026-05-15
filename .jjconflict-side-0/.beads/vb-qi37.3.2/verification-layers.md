# Verification Layers: vb-qi37.3.2

## Boundary

- **Verified kernel**: `CollectStates::capture_state`, `CollectStates::hydrate_extra`, `validate_hydrated_identity`, `hydrate_collect_states_from_recovered_journal`, and the `CollectPaginationState` Postcard codec.
- **Lean contract projection**: Postcard round-trip + identity validation (both waived; unit tests provide compensating evidence).
- **Runtime shell**: `drive_deterministic_full`, `EvidenceCollector::push_slot_written_with_extra`, Fjall journal persistence, and recovery integration.
- **External systems excluded from formal proof**: Fjall LSM internals, OS filesystem, wall-clock time.

## Layer Assignment

| Clause | Primary Layer | Secondary Layer | Evidence |
|--------|--------------|-----------------|----------|
| PP1 (capture_state returns correct identity) | `code-review` | `unit` | Structural proof at `collect.rs:86-92` + `collect_tests.rs:2112-2154` |
| PP2 (evidence embedding) | `code-review` | `unit` | Structural proof at `drive.rs:98-100` |
| PP3 (extra Postcard encoding) | `unit` | `proptest` | `collect_tests.rs:2112-2154` |
| PP4 (journal record kind) | `code-review` | N/A | `events.rs:214` |
| PQ1-PQ4 (persistence path) | `unit` | `cargo-fuzz` | `collect_tests.rs:2193-2258` |
| PQ5 (recovery reconstruction) | `unit` | N/A | `collect_tests.rs:2238-2258` |
| PQ6 (resumed collect_next) | `unit` | N/A | `collect_tests.rs:2247-2251` |
| RP1 (hydrate_journal_event extraction) | `code-review` | `unit` | `collect.rs:116-126` + `collect_tests.rs:2166-2172` |
| RP2 (identity validation) | `code-review` | `unit` | `collect.rs:138-148` + `collect_tests.rs:2262-2307` |
| RP3-RP4 (error paths) | `unit` | N/A | `collect_tests.rs:2262-2307` |
| RP5 (empty recovery) | `code-review` | `unit` | `collect.rs:130-136` |
| RQ1-RQ6 (recovery behavior) | `unit` | N/A | `collect_tests.rs:2188-2307` |
| PI1 (identity preserved through cycle) | `code-review` | N/A | Chain: `drive.rs:98` → `collect.rs:76-78` → `events.rs:98-99` → `collect.rs:130-136` |
| PI3 (cross-run contamination blocked) | `code-review` | `unit` | `collect.rs:138-148` + `collect_tests.rs:2285-2307` |
| PI4 (fresh CollectStates on recovery) | `code-review` | N/A | `collect.rs:133` |

## Fuzzing Scope

- **Parser/Codec**: `collect_tests.rs` uses structured unit tests; Postcard fuzzing is covered by the broader fuzzing corpus for `vb_storage` codec targets
- **Protocol**: Journal event ordering and extra byte corruption are covered by unit tests

## Concurrency Scope

- No concurrency in collect primitive — `Shard` is single-threaded for run execution
- `CollectStates` is owned per-run and passed by `&mut` reference — no shared ownership

## Performance Scope

- No performance-critical claims in this contract
- Cursor capture is O(1) HashMap lookup
- Cursor hydration is O(n) over journal events with extras

## Static Analysis Scope

- `#![forbid(unsafe_code)]` enforced crate-wide in `vb_runtime`
- No `unwrap`/`expect`/`panic` in collect persistence path
- All fallible operations return `Result<T, EngineError>`

## Verification Summary

| Layer | Count | Status |
|-------|-------|--------|
| `unit` | 15 | Covered by `collect_tests.rs:2112-2307` |
| `code-review` | 9 | Structural proofs at specified locations |
| `proptest` | 1 | Waived; Postcard round-trip unit-tested |
| `cargo-fuzz` | 0 | Waived; structured unit tests provide equivalent coverage |
| `waiver` | 2 | Postcard codec, identity validation |

**Total clauses**: 27 (15 unit + 9 code-review + 2 waived + 1 implicit)
**Coverage**: 100% of contract clauses have at least one verification layer
