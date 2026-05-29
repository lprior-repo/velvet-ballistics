# Workflow Model — Fuzz Hardening (vb-hbav)

## State Machine: Fuzz Target Lifecycle

```
                               ┌─────────────────┐
                               │   UNDECLARED    │  File exists, no [[bin]] entry
                               └────────┬────────┘
                                        │ DeclareTarget
                                        ▼
                               ┌─────────────────┐
                               │    DECLARED     │  [[bin]] entry present in Cargo.toml
                               └────────┬────────┘
                                        │ BuildTargets (cargo fuzz build)
                                        ▼
                               ┌─────────────────┐
                      ┌───────│    COMPILABLE    │───────┐
                      │ FAIL  └────────┬────────┘ FAIL  │
                      ▼                │                ▼
              ┌────────────┐           │       ┌────────────┐
              │UNCOMPILABLE │        SUCCESS    │UNCOMPILABLE │
              │  (fix)      │           │       │  (skip)    │
              └──────┬──────┘           │       └────────────┘
                     │                  ▼
                     └──────► ┌─────────────────┐
                              │   INSTRUMENTED  │  libfuzzer: nm | grep LLVMFuzzer
                              │                 │  Stdin: feature="fuzz" main()
                              └────────┬────────┘
                                       │ Smoke test (10s, no sanitizers)
                                       │ Gate: zero crashes
                                       ▼
                              ┌─────────────────┐
                              │     SMOKED      │  Panic-freedom verified
                              └────────┬────────┘
                                       │ HardenHarness (add assertions)
                                       ▼
                              ┌─────────────────┐
                              │    HARDENED     │  ≥ TypedError assertion strength
                              └────────┬────────┘
                                       │ ASAN smoke test (10s)
                                       │ Gate: zero crashes, zero leaks
                                       ▼
                              ┌─────────────────┐
                              │   ASAN-SMOKED   │  Sanitizer-clean baseline
                              └────────┬────────┘
                                       │ CreateSeedCorpus (≥ 1 seed)
                                       ▼
                              ┌─────────────────┐
                              │     SEEDED      │  Corpus directory populated
                              └────────┬────────┘
                                       │ RunDeepCampaign (1hr, ASAN)
                                       │ Gate: zero crashes, corpus growth > 0
                                       ▼
                          ┌───────────────────────┐
                          │   CAMPAIGNED (PASS)   │  Target is hardened and proven
                          └───────────────────────┘
```

### State Transition Table

| From | Guard | Command | To | Error Outcome |
|------|-------|---------|-----|---------------|
| `Undeclared` | File exists | `DeclareTarget` | `Declared` | InvalidTargetName |
| `Declared` | Cargo.toml valid | `BuildTargets` | `Compilable` | `BuildFailed` → `Uncompilable` |
| `Uncompilable` | Error fixed | `BuildTargets` | `Compilable` | `BuildFailed` (retry) |
| `Compilable` | `nm \| grep LLVMFuzzer` passes | `VerifyInstrumentation` | `Instrumented` | `Uninstrumented` |
| `Instrumented` | Zero crashes in 10s | `SmokeTest` | `Smoked` | `SmokeFailed` |
| `Smoked` | Assertions added, strength ≥ TypedError | `HardenHarness` | `Hardened` | `InsufficientAssertions` |
| `Hardened` | Zero ASAN crashes in 10s | `AsanSmoke` | `AsanSmoked` | `AsanCrashFound` |
| `AsanSmoked` | ≥ 1 seed file created | `CreateCorpus` | `Seeded` | `CorpusEmpty` |
| `Seeded` | Zero crashes, growth > 0 in 1hr | `RunDeepCampaign` | `CampaignedPass` | `CampaignCrashFound`, `CampaignNoGrowth` |

## State Machine: 21-Target Hardening Workflow

```
START
  │
  ├── 1. fuzz_yaml_events
  │     Current: CoverageOnly (let _profile, let _events, let _source_map)
  │     Target:  Structural (assert !events.is_empty() for non-empty input,
  │              source_map has entries, match YamlError variants)
  │
  ├── 2. fuzz_replay_events
  │     Current: CoverageOnly (let _result)
  │     Target:  Structural (assert replayed.len() <= events.len(),
  │              ActionReplayTracker invariants)
  │
  ├── 3. fuzz_extract_terminal
  │     Current: CoverageOnly (let _terminal)
  │     Target:  Structural (assert terminal.children().is_empty(),
  │              terminal is a valid node kind)
  │
  ├── 4. fuzz_action_tracker
  │     Current: CoverageOnly (no assertions on tracker state)
  │     Target:  Structural (assert is_resolved is deterministic,
  │              mark_completed and mark_failed transition correctly)
  │
  ├── 5. fuzz_accepted_artifact_envelope_qi37_4_2
  │     Current: CoverageOnly (let _ = artifact.field)
  │     Target:  Structural (assert gate_count > 0, accepted_at_seq >= 1,
  │              required_capabilities.len() matches contract)
  │
  ├── 6. fuzz_expr_bytecode
  │     Current: CoverageOnly (let _result)
  │     Target:  Structural (assert result type_name is known type,
  │              stack depth ≤ max, no silent Null on success)
  │
  ├── 7. fuzz_verifier_gates
  │     Current: CoverageOnly (drop(result) on each gate)
  │     Target:  TypedError (match ValidationError per gate,
  │              assert gate-specific invariants)
  │
  ├── 8. fuzz_budget_compute
  │     Current: CoverageOnly (let _ = budget.max_total_steps)
  │     Target:  Structural (assert max_total_steps > 0 for non-empty,
  │              all budget components are non-negative, fanout ≤ max)
  │
  ├── 9. fuzz_admission_flow
  │     Current: CoverageOnly (drop(submit_artifact(...)))
  │     Target:  TypedError (match AdmissionError variants,
  │              assert artifact exists in store after submit)
  │
  ├── 10. fuzz_expr_eval
  │     Current:  Structural (already has assertions)
  │     Target:   Verify assertions are mutation-resistant
  │
  ├── 11. fuzz_accessor_traversal
  │     Current:  CoverageOnly (drop(eval_accessor_with_store(...)))
  │     Target:   Structural (assert path depth ≤ FUZZ_MAX_ACCESSOR_DEPTH,
  │               slot reference validity on success)
  │
  ├── 12. fuzz_admission_fuzz
  │     Current:  CoverageOnly (let _result)
  │     Target:   Structural (assert parts has ≥ 1 node on success,
  │               match AdmissionError variants)
  │
  ├── 13. fuzz_digest_coherence
  │     Current:  CoverageOnly (let _result)
  │     Target:   Equivalence (blake3::hash(data) == compute_digest(data)
  │               when both paths succeed)
  │
  ├── 14. fuzz_admission_input_surface
  │     Current:  CoverageOnly (let _strict, let _relaxed)
  │     Target:   Equivalence (strict and relaxed paths agree on success/failure
  │               for identical inputs)
  │
  ├── 15. fuzz_readback_family_set
  │     Current:  CoverageOnly (let _classification)
  │     Target:   Structural (classification ∈ {Full, Partial, Absent, Unreadable},
  │               no Unreadable when all families are readable)
  │
  ├── 16. fuzz_accepted_artifact_decode
  │     Current:  CoverageOnly (let _result)
  │     Target:   Structural (assert decoded artifact has accepted_at_seq > 0,
  │               gate_count matches verification claims)
  │
  ├── 17. fuzz_recovery_decode
  │     Current:  CoverageOnly (let _summary, let _seed)
  │     Target:   Structural (assert seed has non-zero fields when events non-empty,
  │               match RecoveryError variants)
  │
  ├── 18. C.25 fuzz_collect_page_pagination
  │     Current:  MISSING (function does not exist in lib.rs)
  │     Target:   Structural (assert page count = ceil(list_len / page_size),
  │               each page item count ≤ page_size, page_size=0 → error,
  │               empty list → empty result, non-list slot → typed error)
  │
  ├── 19. fuzz_action_tracker (src/bin duplicates #4)
  │     Target:  Remove duplicate, reference shared fuzz_lib::fuzz_action_tracker
  │
  ├── 20. decode_record (fuzz_targets)
  │     Current:  CoverageOnly (all .ok() suppressed)
  │     Target:   TypedError (match JournalError variants exhaustively,
  │               assert is_valid() on Ok, roundtrip on valid)
  │
  └── 21. expr_eval (fuzz_targets duplicates #10)
        Target:  Verify delegates to shared fuzz_lib::fuzz_expr_eval
```

## Stdin Boilerplate Refactor Workflow

```
START: 38 files each contain identical run_with_stdin/write_stderr/main
  │
  ├── Create fuzz/src/bin_common.rs:
  │     pub fn run_with_stdin(target: fn(&[u8])) -> ExitCode
  │     fn write_stderr(error: io::Error) -> ExitCode
  │
  ├── For each of 38 src/bin/*.rs files:
  │     Replace: entire file → #[cfg(feature="fuzz")] fn main() -> ExitCode {
  │                 run_with_stdin(fuzz_lib::fuzz_TARGET)
  │               }
  │     Remove:  duplicate run_with_stdin, write_stderr definitions
  │
  └── RESULT: 38 files × ~25 lines = ~950 lines removed → ~3 lines each ≈ 114 lines
              Net savings: ~836 lines
              Exactly one canonical run_with_stdin implementation
```

## C.21-C.24 ASAN Verification Workflow

```
CLAIMED FIXED targets: generated_compare, compiled_ir, ipc_frame, expression
  │
  ├── Gate: Each target must have non-CoverageOnly assertion strength
  │     generated_compare: Equivalence (validation vs construction agree)
  │     compiled_ir:       Structural (slot bounds, digest preservation, node count)
  │     ipc_frame:         Structural+Roundtrip (header re-encode matches, payload variants)
  │     expression:        Structural (type_name non-empty on Ok)
  │
  ├── For each target:
  │     cargo fuzz run TARGET -- -max_total_time=3600 -rss_limit_mb=2048 \
  │         -print_final_stats=1 -detect_leaks=1
  │
  ├── Gate outcomes:
  │     Zero crashes → PASS
  │     Zero leaks   → PASS
  │     Corpus growth > 0 → PASS
  │     Any crash → FAIL (file bead, fix, re-run)
  │
  └── All 4 must PASS before Phase 1 exit gate
```

## C.25 collect_page Implementation Workflow

```
START: collect_page_pagination bin exists, fuzz_collect_page_pagination missing
  │
  ├── Implement in fuzz/src/lib.rs:
  │     pub fn fuzz_collect_page_pagination(data: &[u8])
  │
  ├── Assertions required:
  │     - page_count = ceil(list_len / page_size)
  │     - each page item count ≤ page_size
  │     - page_size = 0 → typed error (not panic)
  │     - empty list → empty result (not error)
  │     - non-list slot → typed error
  │     - RunFrame::new must not panic
  │     - collect_page must never panic
  │
  ├── Build, smoke, ASAN smoke
  │
  └── Must PASS 1-hour ASAN campaign before exit
```

## Seed Corpus Creation Workflow

```
For each target T among 24+ targets lacking seed corpora:
  │
  ├── mkdir -p fuzz/corpus/T/
  │
  ├── Generate seed categories:
  │     1. Empty input (0 bytes)           → seed_empty.bin
  │     2. Single byte 0x00                → seed_single_00.bin
  │     3. Single byte 0xFF                → seed_single_ff.bin
  │     4. Single byte 0x7F                → seed_single_7f.bin
  │     5. Magic bytes if format-aware     → seed_magic.bin
  │     6. Valid happy path from tests     → seed_valid.bin
  │     7. Edge case at boundary           → seed_edge.bin
  │     8. Bit-flipped from valid          → seed_corrupt.bin
  │
  ├── Gate: Every target gets ≥ 1 seed
  │     Structure-aware targets (Parser, Roundtrip, StructureAware) get ≥ 5 seeds
  │
  └── Seeds are committed to repository
```

## Terminal States

| State | Meaning | Exit Condition |
|-------|---------|---------------|
| `CampaignedPass` | Target hardened, ASAN-clean, seeded, campigned, zero crashes | Bead closure valid |
| `Uncompilable` | Target cannot be built (missing crate, API drift) | Requires separate bead for crate fix |
| `CampaignCrashFound` | Crashes discovered in deep campaign | Bead filed for each crash, target re-fuzzed after fix |
| `Skipped` | Target has known pre-existing condition (e.g., vb_5xs4 targets need API analysis) | Deferred to future bead |

## Concurrent Target Processing

The 21 weak targets can be hardened concurrently (independent harnesses in lib.rs), but must be sequentially verified through the build/smoke/ASAN/campaign pipeline. The stdin refactor must complete before any target's bin file is modified.

## Idempotence

- `cargo fuzz build` is idempotent (rebuilds only changed targets)
- `cargo fuzz run TARGET` is idempotent (corpus accumulates across runs)
- Hardening assertions must be idempotent (adding an assertion twice produces same code)
- Seed corpus creation is idempotent (files can be overwritten with same content)
