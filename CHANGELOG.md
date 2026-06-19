# Changelog

## [Unreleased]

### Tier A v0.1.0 — BATTLE TESTING (NOT RELEASED)

Status: in progress, NOT YET released. v0.1.0 tag deleted on 2026-06-19
after independent battle-test revealed Kani toolchain incompatibility and
uncommitted implementation drift.

**Implemented (claimed at bead close)**

- 22 Tier A beads created (tier-a-0-001..022)
- Master §78 amendment (3cdbca26b)
- Kani harness cleanup + global ASM isolation
- 22 residue beads nuked (UI/codegen)
- 17 P4 beads deleted
- IPC: chmod 0o600, CallerCapabilities envelope, peer-credentials
- Runtime: TOCTOU shutdown CAS, terminal-runs LRU + TTL
- Compiler: proptest roundtrip, WholeWorkflowBudget analyzer
- Verus: 5 spec fns + 6 lemmas in vb_expr (file: crates/vb_expr/src/eval/verus.rs)
- Verus: 1 lemma in resource_budget (file: crates/vb_proof_kernels/src/resource_budget/spec.rs)
- Verus: 1 exec fn + 1 lemma in runtime_facade_api (file: crates/vb_runtime/src/verification/verus/runtime_facade_api.rs)
- Verus: 7 dual-mode proof kernels registered
- Verus: 14 inline `#[cfg(verus)]` blocks audited

**Battle test results** (this run, 2026-06-19)

`moon ci` (5m 31s wall-clock, NOT 1800s timeout):
- 29 completed, 18 FAILED, 4 skipped
- 11 Kani buckets failed: `kani::proof_for` does not exist in installed cargo-kani 0.67.0
  (tier-a-9-017 claim is NOT verifiable on this toolchain)
- 4 vb_runtime shard tests FAILED in `velvet-ballistics:test`:
  - `shard_config_new_accepts_valid_parameters`
  - `shard_config_new_at_max_capacity_boundary`
  - `shard_config_new_accepts_max_step_budget`
  - `shard_config_new_at_minimum_capacity`
  Root cause: tier-a-6-014 sets `DEFAULT_MAX_TERMINAL_RUNS = 100_000`
  via `lru_ring.rs`, but tests assert `max_terminal_runs: 16`.
- 4 same shard tests FAILED in `velvet-ballistics:sanitizer-address-check`
- `velvet-ballistics:lint-src` FAILED: arithmetic_side_effects, as_conversions
  on vb_runtime (10 errors)
- `velvet-ballistics:fmt` FAILED: formatting drift on budget_analyzer.rs,
  proptest_compile_ir_roundtrip.rs, capabilities.rs, and others
- `velvet-ballistics:test-integrity` FAILED: ignored/skip without justification
- `velvet-ballistics:test-determinism` FAILED: new distinct labels exceed baseline

Per-bead test verification:
- tier-a-6-011 caller_capabilities: 7 from_wire tests PASS; capability tests PASS
- tier-a-6-011 peer_credentials: 7 tests PASS
- tier-a-6-011 permission: 2 permission_denied tests PASS
- tier-a-6-012 bind_sets_socket_mode_to_0o600: 1 test PASS
- tier-a-6-013 shutdown_cas: 9 tests PASS
- tier-a-6-014 terminal_runs_lru: 3 tests PASS (filter)
- tier-a-6-014 lru_ring: 3 tests PASS (filter)
  WARNING: full vb_runtime test suite shows 4 SHARD_CONFIG failures
  because `max_terminal_runs` default 100000 ≠ expected 16
- tier-a-3-008 proptest_compile_ir_roundtrip: 2 tests PASS
- tier-a-3-009 docs/ir-primitive-coverage.md: EXISTS (82 lines)
- tier-a-7-016 budget_analyzer: 2 tests PASS
- tier-a-9-017 kani_workflow_arbitrary: NOT TESTABLE — kani 0.67.0 lacks `proof_for`
  and module is gated `#[cfg(all(kani, ...))]`

Kani toolchain:
- cargo-kani 0.67.0 INSTALLED
- `cargo kani -p vb_core --harness kani_workflow_parts_arbitrary` FAILED:
  `error[E0433]: failed to resolve: could not find proof_for in kani`
  at crates/vb_core/src/kani_workflow_arbitrary.rs:667:9
- Conclusion: tier-a-9-017 cannot be battle-tested until kani is upgraded

Verus toolchain:
- verus 0.2026.05.05 INSTALLED
- `bash scripts/verify-verus.sh` (registry-driven) PASSED: 19/19 obligations
  verified, trust-scan OK, evidence at .evidence/verus/
- `verus --crate-type=lib crates/vb_runtime/src/verification/verus/runtime_facade_api.rs`
  FAILED: `cannot call function runtime_facade_api::exec_shard_index_runtime
  with mode exec` at runtime_facade_api.rs:147:13
  → tier-a-6-015 / vb-puvkn claim is FALSE
- `verus --crate-type=lib crates/vb_expr/src/eval/verus.rs` FAILED:
  unresolved `vb_core` import, missing `verus!` macro
  → file is not a standalone Verus spec; cannot be evaluated
- `verus --crate-type=lib crates/vb_proof_kernels/src/resource_budget/spec.rs`
  FAILED: `too many leading super keywords` at spec.rs:8:5
  → tier-a-7 spec claim is FALSE

vb_storage build break:
- moon ci reported `event_replay` file not found
- After re-test: vb_storage compiles cleanly, 1584 tests PASS in 23.77s
- Conclusion: the moon-ci error was a transient parallel-build race condition,
  NOT a real source break. The directory `recovery/event_replay/` with mod.rs
  exists and works.

miri on lru_ring:
- cargo-miri INSTALLED
- `cargo +nightly miri test -p vb_runtime --lib --all-features lru_ring` PASSED:
  3 tests OK in 10.05s
  - test_terminal_runs_lru_bounded_under_load
  - test_terminal_runs_lru_evicts_oldest_after_capacity
  - test_terminal_runs_lru_respects_ttl_seconds

Uncommitted files (this run):
- 22 modified tracked files (all related to Tier A work in flight)
- 4 new .bead-progress/ directories
- 1 new workspace test file
- These should be reviewed before v0.1.0 release; not "17 unrelated files"

**v0.1.0 release criteria (still NOT met)**

- All tier-a beads closed (claimed closed; battle-test reveals tests broken)
- `moon ci` PASS at green budget (FAILED: 18 tasks, 5m 31s)
- All 4 PARTIAL Verus closures become DONE (FAILED: 3 of 3 files FAILED verus)
- Pre-existing vb_storage build break fixed (RESOLVED: was never broken)
- v0.1.0 tag signed with GPG key (DELETED pending re-battle-test)

Track: master §78 Tier A v0.1.0
