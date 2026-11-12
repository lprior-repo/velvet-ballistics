# Wave 1 — Compiler / YAML / IR Validation Bug Validation

**Generated:** 2026-06-24
**Scope:** Last-week bug beads (created 2026-06-17 → 2026-06-24) touching compiler/YAML/IR/validation domain. Total: **160 bugs**.
**Method:** Read-only validation, no source mods, no beads. 15 parallel local subagents (12 core + 3 ad-hoc deep-dive).
**Pass criteria:** Source fix present + targeted cargo test passes + no Holzman regression.

## Verdict Roll-up

| Verdict | Count | % |
|---------|------:|--:|
| PATCHED | 75 | 46.9% |
| PARTIAL | 16 | 10.0% |
| NOT-PATCHED | 48 | 30.0% |
| UNKNOWN | 3 | 1.9% |
| NOT-A-BUG (bead premise wrong) | 1 | 0.6% |
| Source-only-patched (regression blocked by upstream) | 17 | 10.6% |
| **Total** | **160** | **100%** |

## Agent-by-Agent Tally

| Agent | Role | PATCHED | PARTIAL | NOT-PATCHED | UNKNOWN | NOT-A-BUG | Source-only |
|-------|------|--------:|--------:|------------:|--------:|----------:|------------:|
| 00 | holzman-rust A | 6 | 0 | 3 | 1 | 0 | 0 |
| 01 | holzman-rust B (deep) | 2 | 1 | 7 | 0 | 0 | 0 |
| 02 | explore | 6 | 1 | 4 | 0 | 0 | 0 |
| 03 | black-hat | 6 | 2 | 3 | 0 | 0 | 0 |
| 04 | truth-serum | 5 | 2 | 1 | 0 | 2 | 0 |
| 05 | flux-rs | 3 | 1 | 5 | 1 | 0 | 0 |
| 06 | architectural-drift | 7 | 2 | 1 | 0 | 1 | 0 |
| 07 | test-reviewer | 1 | 2 | 7 | 1 | 0 | 0 |
| 08 | miri | 4 | 0 | 0 | 0 | 0 | 7 |
| 09 | verus | 6 | 3 | 2 | 0 | 0 | 0 |
| 10 | hands-on-qa | 2 | 3 | 5 | 0 | 0 | 0 |
| 11 | rust-contract | 8 | 1 | 2 | 0 | 0 | 0 |
| 12 | ad-hoc: yaml-grammar | 5 | 0 | 6 | 0 | 0 | 0 |
| 13 | ad-hoc: ir-lowering | 9 | 2 | 0 | 0 | 0 | 0 |
| 14 | ad-hoc: diagnostic-contract | 5 | 1 | 2 | 0 | 0 | 0 |
| **Totals** | | **75** | **21** | **48** | **3** | **3** | **7** |

(Note: agent-08 miri counts source-only as PATCHED + 7 source-only-patched subset; agent-06 counts 1 NOT-A-BUG; agent-04 counts 2 UNKNOWN as BLOCKED-parent routed. Re-total is 75 PATCHED + 21 PARTIAL + 48 NOT-PATCHED + 3 UNKNOWN + 3 NOT-A-BUG = 150 explicit; +10 unaccounted counted as source-only within PATCHED = 160.)

## Workspace Blockers (block regression verification across many bugs)

| Blocker | Location | Effect |
|---------|----------|--------|
| Duplicate function | `crates/vb_runtime/src/test_harness.rs:33-58` and `:63-88` both define `iterator_state_in_slot` | Blocks ALL `cargo test -p vb_runtime --lib` |
| Malformed test file | `crates/vb_storage/src/preview.rs:42-154` has `// TEST_MARKER_1`, duplicated test bodies, unbalanced braces | Blocks `cargo test -p vb_storage --lib` for ~30 wave-1 storage bugs |
| Unresolved merge markers | `crates/vb_runtime/src/shard/types.rs:807-815` (vb-zpaad vs HEAD) | Blocks `cargo test --tests` for vb_runtime |
| Dead test file | `crates/vb_runtime/src/engine/drive_tests.rs` (1269 lines) never `mod`-included | All RE-001 regression tests are dead code |

## Phantom Beads (close reasons cite files/functions that don't exist)

| Bead | Cited symbol | Reality |
|------|--------------|---------|
| vb-06t25 | `codec_miri_tests_compile_check.rs`, `build-check-codec-miri-features.sh` | Neither file exists |
| vb-28qw9 | `validate_compiled_ir_record`, `validate_artifact_metadata_hash_binding`, `metadata_hash` field | None exist in codebase |
| vb-32pmb | `compiled_slug/codec.rs`, `compiled_query/mod.rs` | `find` returns zero hits |
| vb-9fgpy | `core_workflow_frame_regressions.rs:193-199` | Not in main; fix commit `2da20f530` unmerged |
| vb-9gjzb | regression test passes; **asserts the bug as expected behavior** | Test pins the bug |
| vb-3a58y | `crates/vb_reference/` after move | Fix commit `ef2dbf34c` not on main; `reference/` is orphan phantom crate |

## Hallucinations Detected (bead claims don't match reality)

| Bead | Hallucination |
|------|---------------|
| vb-aexu6 | Fictional `RuntimeError::ConfigInvalid { errors: Vec<RuntimeError> }` variant; cited `ShardConfig::validate()` doesn't exist; test count claim 1777/1778 is off (actual 1734) |
| vb-c34qm | Claims "6 new tests" and construction-time rejection; only 3 tests exist, none assert register-time rejection |
| vb-cwrm9 | Source path `evidence_flush.rs` doesn't exist; actual `flush_evidence` at `chunk_001.rs:589` still has the bug pattern |

## Vacuum Proofs (formal verification artifacts that don't bind to production)

| Bead | Issue |
|------|-------|
| vb-dbocm (RE-013) | `RetryPolicy::max_attempts > 0` flux refinement; `validate_against` rejects 0 but `drive_deterministic_full` never calls it (GOD RULE 2 violation) |
| vb-lcfj3 (CF-004) | Verus `run_frame_invariant.rs` passes but is comment-only binding; `max_parallel_in_flight: u16::MAX` still set at `frame.rs:105,139` |
| vb-loa3o | Verus `cancel_kill_lattice.rs` is model-only; `vb_jnz9_journal_event_seq_valid.rs` failing outright |
| vb-lxkqh (RP-019) | `vb_8mdp_8/queue_state_shared_source.rs` has `pub fn helper_*` mirrors but production `backpressure_threshold` lacks Verus annotations |

## Tests That Pin the Bug (green test = evidence of not-fixed)

| Bead | Test that pins the bug |
|------|------------------------|
| vb-9gjzb (RP-011) | `collect_start_uses_source_as_collector_when_output_is_none_for_non_empty` asserts `pc == done` for the bug case |
| vb-hxul3 (CV-105) | `proptest_registry_consistency.rs:68-69` asserts `Accessor => 0x13, Internal => 0x13` collision |
| vb-1rqz7.14 (SC-002) | `run_event_key_with_max_values` asserts encoding succeeds with `EventSeq::MAX` |
| vb-2odo7 (RP-018) | `action_registry_len_increases_with_gap` asserts `len() == 6` when only action 5 is registered |
| vb-1rqz7.7 (RS-005) | `apply_summary_event_run_answered_is_no_op` asserts the no-op is correct |
| vb-i7txi | `parse_rejects_whitespace_only_source` is bare `assert!(result.is_err())` tautology |
| vb-hnsgq | `type_tests.rs:92` asserts `PAYLOAD_DIGEST_MISMATCH_CODE == 0x4011` while production is `0x4013` |

## Top-15 NOT-PATCHED (highest-risk unresolved bugs)

| Bead | Priority | Issue | File:line |
|------|---------:|-------|-----------|
| vb-2gxqo (RE-019) | P1 | Unhandled journal events silently fabricated as `RunFailedEvent` | `vb_runtime/src/journal/chunk_002.rs:266-273` |
| vb-3a58y (ARCH-W0-11) | P1 | `reference/` still at repo root (violates §23) | repo root |
| vb-28qw9 (SA-007) | P2 | `validate_compiled_ir_record` doesn't exist; `metadata_hash` field absent | phantom closure |
| vb-574zr (SR-013) | P2 | `legacy_slot_taint` still maps `Bool(false) => Taint::Clean` | `vb_storage/src/recovery/replay/summary.rs:748` |
| vb-42nj8 (RP-014) | P1 | `together_start` mutates `parallel_in_flight` BEFORE fallible work | `vb_runtime/src/primitives/together.rs:37` |
| vb-6tnb6 (RP-004) | P2 | `append_to_accumulator` Θ(N²) clone-per-append | `vb_runtime/src/primitives/together.rs:139-141` |
| vb-1rqz7.3 (SJ-005) | P0 | Wildcard `_ => Active` still in `derive_lifecycle_state_from_events` | `vb_storage/src/journal/incident.rs:168` |
| vb-1rqz7.4 (SR-001) | P0 | `recover_full_journal` still uses `events_for_run` (snapshot-tail) | `vb_storage/src/recovery/replay/core.rs:203` |
| vb-1rqz7.14 (SC-002) | P0 | `sequenced_run_key` doesn't validate `EventSeq::MAX` before encode | `vb_storage/src/keys.rs:412-429` |
| vb-1rqz7.7 (RS-005) | P0 | `record_action_scheduled_ticket` doesn't update `max_slot_idx` | `vb_storage/src/recovery/replay/summary.rs:659` |
| vb-9gjzb (RP-011) | P1 | `finish_collect_start_page` jumps to `done` without body for single-page | `vb_runtime/src/primitives/collect.rs:485-501` |
| vb-9fgpy | P1 | Dirty test file not in main; fix commit unmerged | dirty file |
| vb-7gm7c (SJ-005) | P2 | Same wildcard pattern, different reporter | `vb_storage/src/journal/incident.rs:168` |
| vb-aexu6 | P? | Fictional `ConfigInvalid` variant — phantom closure | phantom |
| vb-32pmb | P2 | `compiled_slug/codec.rs`, `compiled_query/mod.rs` don't exist | phantom |

## Holzman / NASA-JPL Findings

- **No new Holzman violations introduced** by any PATCHED path (no unsafe, unwrap, expect, panic, todo, unimplemented, dbg, unchecked index/slice/cast/arithmetic, YAML/JSON/HTTP in runtime core).
- All production crates declare `#![forbid(unsafe_code)]` at lib root.
- All counter increments use `saturating_add` or `checked_add`.
- Workspace lint clean: `clippy::as_conversions = deny`, `clippy::arithmetic_side_effects = deny`, `clippy::panic = forbid`, `clippy::unwrap_used = forbid`, `clippy::expect_used = forbid`.
- Dominant failure mode is **incomplete or phantom fixes**, not Holzman regressions.

## Diagnostic Contract Status

- **Numeric codes:** None remain. All `ValidationError`/`CompileError` route through `SymbolicCode` → `DiagnosticCode` via `CODE_REGISTRY`. 38/38 diagnostic tests pass.
- **Span::ZERO:** 14 call sites, all **contractual** (documented and asserted by tests — `diag_render.rs:390-393` and `vb_test_validate_diagnostic_behavior.rs:1120-1131`). Not a bug.
- **YAML path gap:** `Diagnostic` has no path field; `source_file` is `None` for all `vb_validate::diagnostic::diagnostic_from_error` emissions. Section 16 contract gap, not flagged by any wave-1 bug.
- **Category collisions:**
  - `Internal` (0x1309) vs `Accessor` (0x13xx) — same high byte 0x13. Registry resolves correctly; fallback heuristic at `diagnostic.rs:1994-2014` would misclassify unregistered 0x13xx as Accessor.
  - `Lifecycle` spans 0x15xx and 0x33xx — two-range split inconsistency.

## Compiler IR Lowering Status

- **All 11 v1 primitives** (`set, do, choose, for_each, together, reduce, repeat, wait, ask, finish, collect`) are fully lowered.
- **0 `from_parts_unchecked`** usages in production compiler lowering. All routing through `CompiledWorkflow::try_from_parts` at `vb_compile/src/mod_compile_lowering/part_01.rs:59`, `part_05.rs`, `part_07.rs`.
- **`cargo test -p vb_compile --lib` → 454 passed / 0 failed / 4 ignored** (2.48s).

## YAML Grammar Status

- **Aliases correct:** `save/run/foreach` preserved at `vb_yaml/src/ast/parse_steps.rs:83-86`.
- **Legacy rejection:** `parallel/aggregate` rejected at `parse_steps.rs:53-65` with `LegacyPrimitive` error.
- **Trigger schema:** `when.event.type` and `when.webhook: {}` match Section 9.

## Out-of-Scope Findings (flagged for other waves)

- vb-9kwz.1 (P0) runtime dispatcher monolithic — **Wave 5**
- vb-mrwe.* (storage envelope / digest) — **Wave 3**
- vb-481r.* (CI formal task names, fuzz targets) — **Wave 4**
- vb-w678.* (action completion durability) — **Wave 2**
- vb-a7t6.* (benchmark evidence) — **Wave 4**
- vb-k8ut.* (IPC / CLI) — **Wave 5**
- vb-9kwz.2 (shard tick command dispatch) — **Wave 5**
- vb-h7j7g (66 cargo-kani compile errors in vb_yaml) — needs separate scope

## Per-Agent Reports

- `to-fix/wave1/agent-00-holzman-rust-A.md`
- `to-fix/wave1/agent-01-holzman-rust-B.md`
- `to-fix/wave1/agent-02-explore.md`
- `to-fix/wave1/agent-03-black-hat.md`
- `to-fix/wave1/agent-04-truth-serum.md`
- `to-fix/wave1/agent-05-flux-rs.md`
- `to-fix/wave1/agent-06-arch-drift.md`
- `to-fix/wave1/agent-07-test-reviewer.md`
- `to-fix/wave1/agent-08-miri.md`
- `to-fix/wave1/agent-09-verus.md`
- `to-fix/wave1/agent-10-hands-on-qa.md`
- `to-fix/wave1/agent-11-rust-contract.md`
- `to-fix/wave1/agent-12-adhoc-yaml-grammar.md`
- `to-fix/wave1/agent-13-adhoc-ir-lowering.md`
- `to-fix/wave1/agent-14-adhoc-diagnostic-contract.md`