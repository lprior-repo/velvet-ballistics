# Proof Coverage Matrix — vb-y9d3v

Maps every contract clause to proof seeds, planned obligations, and required verifier lanes.

## Contract Clause Coverage

| Clause ID | Clause Summary | Seeds | Required Obligations | Verifier Lanes |
|---|---|---|---|---|
| ACT-001 | External action authority only for live non-terminal run | 001, 004, 006 | PO-001, PO-002, PO-004, PO-006, PO-009, PO-011, PO-013, PO-014, PO-020, PO-024, PO-025 | Verus, Kani, Flux, proptest |
| ACT-002 | Step in bounds, Running, Do node matches ticket action | 001, 004 | PO-001, PO-002, PO-004, PO-009, PO-011, PO-020, PO-022 | Verus, Kani, Flux, proptest |
| ACT-003 | capacity > 0 and 1 <= attempt <= capacity | 001, 002, 003 | PO-001, PO-002, PO-003, PO-012, PO-017, PO-018, PO-023 | Verus, Kani, Flux, proptest |
| ACT-004 | Idempotency key equals canonical key | 004 | PO-004, PO-009, PO-011, PO-019, PO-022 | Verus, Kani, Flux, proptest |
| ACT-005 | Exact attempt equality for external completion/failure | 001, 002 | PO-001, PO-002, PO-004, PO-009, PO-011, PO-017, PO-022, PO-024 | Verus, Kani, Flux, proptest |
| ACT-006 | Future attempt within capacity not retry authority | 002 | PO-002, PO-011, PO-012, PO-018, PO-022, PO-023 | Verus, Kani, Flux, proptest |
| ACT-007 | Invalid authority must not mutate frame/journal/trace | 001-004 | PO-001, PO-002, PO-003, PO-004, PO-009, PO-013, PO-022, PO-024 | Verus, Kani, Flux, proptest |
| ACT-008 | Completion payload checks before ActionCompletedEnvelope | 004, 007 | PO-004, PO-009, PO-011, PO-013, PO-020, PO-024 | Verus, Kani, Flux, proptest |
| ACT-009 | Failure handler validates authority before retry/handler/failure | 004 | PO-004, PO-009, PO-013, PO-024 | Verus, Kani, Flux, proptest |
| ACT-010 | Retry advancement bounded, checked arithmetic | 003, 008 | PO-003, PO-007, PO-008, PO-012, PO-019, PO-023 | Verus, Kani, Flux, proptest |
| ACT-011 | Retry capacity is max bound, not authorization | 002, 003 | PO-002, PO-003, PO-012, PO-018, PO-023 | Verus, Kani, Flux, proptest |
| ACT-012 | Terminal run cleanup fences off later actions | 004, 005, 006 | PO-005, PO-006, PO-009, PO-013, PO-015, PO-020, PO-024, PO-025 | Verus, Kani, Flux, proptest |
| TMR-001 | Timer fire authoritative only at current generation | (via seed 004) | PO-0031, PO-0032, PO-0034, PO-0038, PO-0039 | Verus, Kani, Flux |
| TMR-002 | Timer replacement increments generation with overflow check | (via seed 004) | PO-0031, PO-0032, PO-0038, PO-0039 | Verus, Kani, Flux |
| TMR-003 | Cancelled/replaced timers stale; must not resume | (via seed 004) | PO-0031, PO-0032, PO-0038, PO-0039 | Verus, Kani, Flux |
| VER-001 | Proof artifacts bind to fresh-main production functions | 001-012 (all) | (all obligations) | All required lanes |
| VER-002 | Prior vb-8mdp.5 artifacts context only | meta | (verification quality gate) | All lanes |

## Acceptance Invariant Coverage (from contract.md)

| # | Invariant | Seeds | Obligations |
|---|---|---|---|
| 1 | Hostile public ActionTicket inputs, not only runtime-generated | 001, 002, 003, 010 | PO-001, PO-002, PO-003, PO-022, PO-023, PO-024 |
| 2 | Lower stale, exact current, future within capacity, zero attempt, zero capacity, over-capacity | 001, 002, 003, 010 | PO-001, PO-002, PO-022, PO-023 |
| 3 | Stale/future/invalid key leaves journal/frame/trace/runtime state unchanged | 001, 002, 003, 004 | PO-001, PO-002, PO-003, PO-004, PO-009, PO-013, PO-022, PO-024 |
| 4 | Retryable failure does not authorize n+2 before n+1 scheduled | 003, 008 | PO-003, PO-007, PO-012, PO-023 |
| 5 | Stale timer generation after replacement/cancel | 004, 008 (TLA+ seed 012 is temporal design context only) | PO-0031, PO-0032, PO-0038, PO-0039 |
| 6 | Verifier lanes planned against current fresh-main wiring | VER-001, VER-002 | (all lane decisions reference fresh-main paths) |

## Hazard Coverage

| Hazard ID | Hazard | Seeds | Obligations |
|---|---|---|---|
| H-ACT-001 | Lower stale attempt overwrites newer run state | 001 | PO-001, PO-011, PO-017, PO-022 |
| H-ACT-002 | Future attempt fabricates authority within capacity | 002 | PO-002, PO-011, PO-018, PO-022 |
| H-ACT-003 | Noncanonical key bypasses duplicate/replay tracking | 004 | PO-004, PO-011, PO-019, PO-022 |
| H-ACT-004 | Invalid completion/failure appends journal before rejection | 004 | PO-004, PO-009, PO-013, PO-024 |
| H-ACT-005 | seq or attempt overflows on retry | 003, 008 | PO-003, PO-007, PO-012, PO-019 |
| H-ACT-006 | Retry metadata zero/out-of-range | 003, 008 | PO-003, PO-012, PO-023 |
| H-ACT-007 | Completion downgrades taint or exceeds byte bounds | 004, 007 | PO-004, PO-009, PO-011, PO-024 |
| H-ACT-008 | Completion after run terminal removal | 004, 005, 006 | PO-005, PO-006, PO-009, PO-013, PO-014, PO-024, PO-025 |
| H-TMR-001 | Replaced/cancelled timer fires from old deadline | 004 | PO-0031, PO-0032, PO-0038, PO-0039 |
| H-TMR-002 | Timer generation overflows | 004 | PO-0031, PO-0032, PO-0038, PO-0039 |
| H-VER-001 | Hardcoded Kani shapes or detached Verus/Flux models | VER-001 | PO-001 through PO-028 (all use Arbitrary/generators, production bindings) |
| H-VER-002 | Prior rejected evidence copied as approval | VER-002 | (all lane decisions cite fresh-main only) |

## Obligation-Verifier Distribution

| Verifier | Obligation Count | Obligation IDs |
|---|---|---|
| Kani | 10 | PO-0001, 0005, 0009, 0013, 0017, 0021, 0025, 0029, 0033, 0037 |
| Verus | 10 | PO-0002, 0006, 0010, 0014, 0018, 0022, 0026, 0030, 0034, 0038 |
| Flux-rs | 10 | PO-0003, 0007, 0011, 0015, 0019, 0023, 0027, 0031, 0035, 0039 |
| proptest | 10 | PO-0004, 0008, 0012, 0016, 0020, 0024, 0028, 0032, 0036, 0040 |
| cargo-fuzz | 1 | PO-0041 |
| **Total** | **41** | |

**Note: TLA+ obligations do not exist.** TLA+ has been globally removed from the verifier whitelist. Seed 012 temporal claims remain as design context only; Rust-local invariants are covered by seeds 001-010.

## Uncovered Risk Tags

All risk tags from proof-seeds.jsonl are covered by at least one obligation. No uncovered risk tags exist.

- `stale-attempt`: covered by PO-001, PO-011, PO-017, PO-022
- `future-attempt`: covered by PO-002, PO-011, PO-018, PO-022
- `retry-fence`: covered by PO-003, PO-007, PO-012, PO-019, PO-023
- `stale-authority`: covered by PO-004, PO-005, PO-009, PO-013, PO-020, PO-024, PO-025
- `single-terminal`: covered by PO-005, PO-006, PO-009, PO-013, PO-020
- `typed-error`: covered by PO-006, PO-014, PO-020, PO-025
- `verus`: covered by PO-011 through PO-016
- `kani`: covered by PO-001 through PO-010
- `flux-rs`: covered by PO-017 through PO-021
- `proptest`: covered by PO-022 through PO-026
- `fuzz` / `codec`: covered by PO-010, PO-027
- `temporal` / `tla-plus`: not_applicable (TLA+ globally removed; seed 012 is temporal design context only)
- `type-contract`: covered by PO-017, PO-018, PO-019, PO-020, PO-021
- `rust-local`: covered by all PO-001 through PO-026
- `rust-local-proof`: covered by PO-011, PO-012, PO-013, PO-014, PO-015, PO-016
