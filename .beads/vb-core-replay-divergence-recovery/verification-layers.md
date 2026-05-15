# Verification Layers — vb-core-replay-divergence-recovery

## Boundary

- Verus-owned kernel: Typed error exhaustive mapping, seq ordering invariants, Postcard round-trip invariants, ActionReplayTracker blocking logic
- TLA+ temporal model: None (single-writer deterministic sequential replay — see tla-spec.md)
- Theorem projection: None
- Runtime shell: Fjall journal read path, frame hydration orchestration, digest verification orchestration
- External systems: Fjall storage backend, CompiledWorkflow artifact store

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Waiver |
|---|---|---|---|
| CC-001 (No YAML in recovery) | miri | static-scan (grep yaml) | None |
| CC-002 (Snapshot+tail hydration) | miri | integration tests | None |
| CC-003 (Typed digest errors) | miri | integration tests | None |
| CC-004 (Typed replay divergence) | miri | integration tests | None |
| CC-005 (Fail-closed corrupt/incomplete) | miri | integration tests | None |
| CC-006 (Object/list unsupported) | miri | integration tests | None |
| CC-007 (Events-only hydration) | miri | proptest | None |
| CC-008 (Frame seed round-trip) | miri | proptest | None |

## miri Scope

**Target crates**: vb_storage, vb_runtime

**Coverage**:
- All recovery functions listed in delivery-scope.jsonl run under miri via existing integration tests
- recovery_integration.rs: 13 test cases covering full/partial write, action replay, digest mismatch detection
- replay_resume.rs: 3 test cases covering tail replay determinism and sequence gap rejection
- vb_runtime/src/recovery.rs unit tests: 8 test cases covering boundary traits and factory functions
- vb_qi37_1_1_red_recovery_contract_test.rs: 14 test cases + 3 proptest cases covering taint preservation, slot recovery, corrupt/missing slots

**Evidence command**: `cargo miri test --package vb_storage --test recovery_integration --test replay_resume --package vb_runtime -- --include-ignored` and `cargo miri test --package workspace_tests --test vb_qi37_1_1_red_recovery_contract_test`

**Trusted boundary**: Fjall journal is treated as external oracle; miri checks memory safety within the recovery code paths.

**Shell exclusions**: Fjall storage I/O (journal read path), wall-clock time, non-determinism from OS scheduler

## static-scan (No YAML Verification)

**Coverage**: Grep scan of vb_storage/src/recovery/ for yaml, serde_yaml, QuickYaml, Yaml decoding imports

**Evidence command**: `rg -i 'yaml|serde_yaml|quick_yaml' crates/vb_storage/src/recovery/ --files-with-matches`

**Expected**: Zero matches

## proptest Scope

**Target**: SlotValue taint preservation, valid slot events hydration, no-output step recovery invariants

**Coverage**: vb_qi37_1_1_red_recovery_contract_test.rs proptest cases

**Evidence command**: `cargo test --package workspace_tests --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture`

**Expected**: All proptest cases pass; no fabrication of slot zero dimensions or missing taints

## Waivers

| Clause | Reason | Compensating Evidence |
|---|---|---|
| No TLA+ model | Single-writer deterministic sequential replay; no temporal liveness properties | miri on all integration tests + proptest |
| No Lean theorem | No algebraic theorem kernel; all properties covered by Verus match exhaustiveness + miri | Verus exhaustiveness + miri integration |
| No Kani | No numeric/indexing/arithmetic proof targets; all bounds are covered by miri + integration tests | miri integration coverage |
