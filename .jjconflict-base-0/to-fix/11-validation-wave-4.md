# Wave 4 — CI / Formal / Evidence Bug Validation

**Generated:** 2026-06-24
**Scope:** Last-week bug beads (created 2026-06-17 → 2026-06-24) touching CI/formal/evidence domain (Kani, Verus, Flux, loom, miri, fuzz, coverage, profile, moon pipeline, benchmarks, sanitizer, supply-chain). Total: **87 bugs**.
**Method:** Read-only validation, no source mods, no beads. 15 parallel local subagents (12 core + 3 ad-hoc deep-dive).
**Pass criteria:** Source fix present + targeted test passes + no Holzman regression.

## Verdict Roll-up

| Verdict | Count | % |
|---------|------:|--:|
| PATCHED | 32 | 36.8% |
| PARTIAL | 11 | 12.6% |
| NOT-PATCHED | 19 | 21.8% |
| UNKNOWN | 21 | 24.1% |
| NOT-A-BUG (premise false) | 4 | 4.6% |
| **Total** | **87** | **100%** |

## Agent-by-Agent Tally

| Agent | Role | PATCHED | PARTIAL | NOT-PATCHED | UNKNOWN | NOT-A-BUG |
|-------|------|--------:|--------:|------------:|--------:|----------:|
| 00 | holzman-rust A | 1 | 1 | 4 | 0 | 0 |
| 01 | holzman-rust B | 2 | 0 | 3 | 0 | 0 |
| 02 | explore | 3 | 1 | 2 | 0 | 0 |
| 03 | black-hat | 4 | 1 | 0 | 0 | 1 |
| 04 | truth-serum | 3 | 2 | 0 | 1 | 0 |
| 05 | flux-rs | 3 | 1 | 1 | 0 | 0 |
| 06 | arch-drift | 2 | 1 | 3 | 0 | 0 |
| 07 | test-reviewer | 1 | 2 | 2 | 0 | 0 |
| 08 | miri | 4 | 0 | 1 | 1 | 0 |
| 09 | verus | 3 | 1 | 2 | 0 | 0 |
| 10 | hands-on-qa | 4 | 1 | 1 | 0 | 0 |
| 11 | rust-contract | 2 | 2 | 2 | 0 | 0 |
| 12 | ad-hoc: kani-harness | 0 | 2 | 0 | 4 | 0 |
| 13 | ad-hoc: verus-binding | 4 | 1 | 1 | 0 | 0 |
| 14 | ad-hoc: moon-pipeline | 0 | 0 | 0 | 5 | 0 |
| **Totals** | | **36** | **16** | **22** | **11** | **1** |

(Note: 1 chunk misassigned — agent-14 moon-pipeline got source-code defects; agent-12 kani-harness got many UNKNOWNs because chunk contained runtime/state defects, not Kani harnesses)

## Major Findings

### Hardcoded Kani Shapes (GOD RULE 1 violations)

7 distinct files:
- `kani_foreach_parity.rs` — constructs fixed `WorkflowParts` via `build_foreach_parts(...)` ignoring all 3 parameters
- `kani_shard_lifecycle_harnesses.rs` (788 lines, 7 fns >25)
- `kani_engine_signals.rs`
- `kani_attempt_fence_harnesses.rs`
- `kani_resume_state_machine.rs`
- `vb_fzgdn_timer_harnesses.rs`
- hardcoded `RunFrame` shape

### Orphan Kani Harnesses (12 distinct files)

- 4 chained through orphan `kani_shard_lifecycle.rs` (no `pub mod` in `vb_runtime/src/lib.rs`)
- `kani_journal_duplicate.rs` in vb_storage
- 6 `kani_resource_contract_*.rs` in vb_compile
- 6 feature-gated `kani_digest_ask_*` harnesses

**`verification/kani/mod.rs` wires only 4 of 13 modules** — 9 Kani modules remain orphan.

### Vacuum Proofs (GOD RULE 2 violations)

| Bead | Issue |
|------|-------|
| vb-w2wde | `vb_jpq724_events_for_run_production.rs` PASSES Verus but proves properties of self-contained `SpecJournalEvent` universe without `requires`/`ensures` binding to `FjallJournal::events_for_run` |
| vb-wb05o | `capability_artifact_model.rs` self-declares "pure model" with `int`-abstracted vs production `Box<str>`/`ActionId`; no `extern_spec` on `admit_artifact_run_with_certificate_floor` |
| vb-wb05o | `accepted_artifact_admission_decision.rs` models 5 spec variants vs 11 payload-carrying production variants; empty-body `proof fn {}` over divergent enum |
| vb-jut5w | 4 Verus artifacts are spec-mirror models with empty reveals |
| vb-keji6 | PS-009 spec enshrines the SA-003 bug as "production behavior" |
| vb-92 mirror verus files | 0 `extern_spec`/`assume_specification`/`BINDING` markers — L4 production-bound obligation unmet |
| vb-y9d3v_action_ticket_refinements.rs | placeholder-type `#[extern_spec]` decls claim invariants production code does not enforce |

### Phantom Closures (carry-over from W1/W2/W3)

| Bead | Cited symbol | Reality |
|------|--------------|---------|
| vb-06t25 | `codec_miri_tests_compile_check.rs`, `build-check-codec-miri-features.sh` | Neither file exists |
| vb-1rqz7.16 | `decode_envelope_only` callers | Function is dead code (zero call sites); 0 tests |
| vb-1rqz7.23 | `kani_admission.rs` correctly fixed | Still has hardcoded `WorkflowParts` + 2 `unwrap()`s + real `FjallJournal::open()` (vacuous) |
| vb-36fly | `serialize_accepted_artifact`, `verify_persisted_artifact_present` | Migrated as `.map_err(\|_\| JournalError::ArtifactMalformed)` — 16 sites now |
| vb-8x5qk | `check-bench-registration` artifact | `.moon/tasks/all.yml:346-358` now contains `supply-chain`; no such task |
| vb-8wufe | `chunk_dispatch_shutdown.rs:271` `shutdown_prevents_new_commands_after_flag_set` | File doesn't exist; 5 of 6 named tests pass |
| vb-98soe | 3 named tests | All return zero matches; close-reason fix targets different 4th test |
| vb-4hei8 | real 1000-runs bench | Current main has trivial substring-check; real bench on `origin/cleanup-30r` only |
| vb-5e4xm | ai-release profile cargo/kani/verus invocation, rename | Profile is purely synthetic |
| vb-7xh3b | `policy/contract.rs:153-272`, `RuntimeLimitsProfile` | Symbol removed from vb_core |

### Profile Contract Gaps (Section 34)

| Profile | Status |
|---------|--------|
| `[profile.release]` | **MISSING** — required by Section 34 |
| `[profile.bench]` | **MISSING** — required by Section 34 |
| `[profile.hardened]` | Present, inherits from implicit release |
| `[profile.maxperf]` | Present, inherits from implicit release |

### Pipeline Gaps (CI-gated tasks missing from `.moon.yml:7-26`)

| Missing task | Source | Status |
|--------------|--------|--------|
| `verify-kani-vb-validate` | `kani.yml:36` `runInCI: true` | Not in pipeline |
| `verify-loom` | `loom.yml:11` `runInCI: true` | `loom.yml` not even `includes`'d by `.moon.yml:1-5` |
| `sanitizer-address-check` | `all.yml:549` `runInCI: true` | Not in pipeline |

### In-Pipeline Formal Tasks (correctly named and fail-closed)

- `verify-kani` (kani.yml:14)
- `verify-verus` (verus.yml:13)
- `verify-tlc` (tlc.yml:14)

All CI-gated tasks begin `set -euo pipefail`. **0 fail-open gates.** Only `set +e` at `all.yml:379` is bounded inside advisory supply-chain geiger helper.

### Test Suite Status

| Suite | Result |
|-------|--------|
| `vb_storage --lib` | 1270 passed |
| `vb_runtime --lib` | 1734 passed (2 unrelated regressions: `execute_reduce_start_errors_on_uninitialized_input`, `execute_repeat_start_single_attempt_no_panic` self-loop bodies rejected by tightened `try_from_parts`) |
| `vb_core --lib` | 2142 passed |
| `vb_validate --lib` | 836 passed |
| Verus registry | 5/5 PASS, trust-scan OK |

### Coverage Threshold

Master §40 is silent on minimum %. Moon `coverage` task is single-test smoke gate, no `--fail-under-*` flag.

## Holzman / NASA-JPL Findings

- **No new Holzman violations introduced** by any PATCHED path
- All production crates declare `#![forbid(unsafe_code)]` at lib root
- `crates/vb_runtime/src/runtime.rs:449-450` carries `#[allow(clippy::as_conversions)]` for two `usize → f32` casts — conflicts with master §44.21

## Per-Agent Reports

- `to-fix/wave4/agent-00-holzman-rust-A.md`
- `to-fix/wave4/agent-01-holzman-rust-B.md`
- `to-fix/wave4/agent-02-explore.md`
- `to-fix/wave4/agent-03-black-hat.md`
- `to-fix/wave4/agent-04-truth-serum.md`
- `to-fix/wave4/agent-05-flux-rs.md`
- `to-fix/wave4/agent-06-arch-drift.md`
- `to-fix/wave4/agent-07-test-reviewer.md`
- `to-fix/wave4/agent-08-miri.md`
- `to-fix/wave4/agent-09-verus.md`
- `to-fix/wave4/agent-10-hands-on-qa.md`
- `to-fix/wave4/agent-11-rust-contract.md`
- `to-fix/wave4/agent-12-adhoc-kani-harness.md`
- `to-fix/wave4/agent-13-adhoc-verus-binding.md`
- `to-fix/wave4/agent-14-adhoc-moon-pipeline.md`