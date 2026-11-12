# vb-ndx8b handoff — precise rename diffing + hash-binding + exact-base lifecycle

## Scope delivered

Fixed `scripts/check-test-integrity.rs` (only source file changed) for the three
blockers, plus actual-runner evidence. No integration `test-integrity-justifications.tsv`
populated yet (final review + Ask fix vb-b24vj pending, per coordinator).

## Actual verification (official runner, no fabricated JSON)

All via `python3 /tmp/vb-run-gate.py 180 <prefix> <cmd> <args>`, from workspace root.
Evidence JSON files are machine-written by the runner (`exit_code`, `coordinator_message`, timing).

| Case | Command | Exit | Seconds |
|---|---|---|---|
| self-test | `bash scripts/check-test-integrity.sh --self-test` | **0** | 2.4 |
| real base | `bash scripts/check-test-integrity.sh --base da633fe0bbde9e414173d5f7aabe314a37f8752d` | **1** | 8.2 |
| source-length | `bash scripts/check-source-length.sh` | 1 (pre-existing) | 3.7 |

Evidence: `.beads/vb-ndx8b/evidence/deepseek-final-v3-{selftest,base,sourcelen}.{json,stdout,stderr}`.

Self-test: 40 `PASS` assertions, **0 `SelfTest:…FAIL`**. Exact finding kind+path asserted
(not bare exit code), setup errors propagate.

## Real-base findings — 35 total, honest raw output

`--base da633fe0bbde9e414173d5f7aabe314a37f8752d` → exit 1. Breakdown:
`DeletedTestFile` 2 · `DeletedTestDeclaration` 7 · `WeakenedAssertion` 20 · `IgnoredOrSkippedTest` 6.

```
IgnoredOrSkippedTest|crates/velvet_ballistics/tests/admission_evidence_integration/chunk_001.rs|return;
IgnoredOrSkippedTest|crates/velvet_ballistics/tests/admission_evidence_integration/chunk_002.rs|// Relaxed admission skips artifact loading, but submit_artifact still
IgnoredOrSkippedTest|crates/velvet_ballistics/tests/admission_evidence_integration/chunk_002.rs|// (typed evidence contract supersedes "relaxed skips checksum").
IgnoredOrSkippedTest|crates/velvet_ballistics/tests/admission_evidence_integration/chunk_002.rs|return;
IgnoredOrSkippedTest|crates/velvet_ballistics/tests/admission_evidence_integration/chunk_002.rs|return;
IgnoredOrSkippedTest|crates/velvet_ballistics/tests/admission_evidence_integration/chunk_003.rs|return;
WeakenedAssertion|crates/vb_cli/tests/admission_evidence_integration/chunk_003.rs|removed_exact=1 added_exact=4 added_weak=1
DeletedTestFile|crates/workspace_tests/tests/vb_nf2u_ui_release_acceptance.rs|deleted file contained tests or test assertions
DeletedTestDeclaration|crates/workspace_tests/tests/vb_nf2u_ui_release_acceptance.rs|removed_declarations=8 added_declarations=0
WeakenedAssertion|crates/workspace_tests/tests/vb_nf2u_ui_release_acceptance.rs|removed_exact=38 added_exact=0 added_weak=0
DeletedTestDeclaration|crates/vb_compile/tests/digest_duplicate_parity.rs|removed_declarations=4 added_declarations=0
WeakenedAssertion|crates/vb_compile/tests/digest_duplicate_parity.rs|removed_exact=4 added_exact=0 added_weak=0
DeletedTestFile|crates/vb_compile/tests/digest_duplicate_parity.rs|deleted file contained tests or test assertions
WeakenedAssertion|crates/vb_compile/tests/v1_primitive_lowering.rs|removed_exact=1 added_exact=0 added_weak=0
WeakenedAssertion|crates/vb_expr/src/expr_bytecode/tests/expr_bytecode_tests.rs|removed_exact=0 added_exact=49 added_weak=2
WeakenedAssertion|crates/vb_runtime/src/admission/tests.rs|removed_exact=0 added_exact=1 added_weak=1
DeletedTestDeclaration|crates/vb_runtime/src/shard/lifecycle/chunk_001.rs|removed_declarations=5 added_declarations=0
WeakenedAssertion|crates/vb_runtime/src/shard/lifecycle_tests/chunk_009.rs|removed_exact=80 added_exact=80 added_weak=14
WeakenedAssertion|crates/vb_runtime/src/shard/tests/chunk_015.rs|removed_exact=2 added_exact=1 added_weak=0
WeakenedAssertion|crates/vb_runtime/src/shard/tests/submission_parity.rs|removed_exact=0 added_exact=13 added_weak=3
WeakenedAssertion|crates/vb_runtime/tests/durable_retry_run_attempt_authority.rs|removed_exact=0 added_exact=12 added_weak=1
WeakenedAssertion|crates/vb_runtime/tests/vb_4qog3_independent_review.rs|removed_exact=0 added_exact=4 added_weak=3
DeletedTestDeclaration|crates/vb_runtime/tests/vb_wave16_mqa.rs|removed_declarations=1 added_declarations=0
WeakenedAssertion|crates/vb_storage/src/inputs/tests.rs|removed_exact=0 added_exact=5 added_weak=1
DeletedTestDeclaration|crates/vb_storage/src/types/flux_sequence.rs|removed_declarations=15 added_declarations=0
WeakenedAssertion|crates/velvet_ballistics/tests/input_mapping_boundary.rs|removed_exact=0 added_exact=4 added_weak=1
WeakenedAssertion|crates/workspace_tests/tests/artifact_evidence_support/schema3_cases.rs|removed_exact=0 added_exact=8 added_weak=1
WeakenedAssertion|crates/workspace_tests/tests/cli/submit_input_boundary.rs|removed_exact=0 added_exact=29 added_weak=2
WeakenedAssertion|crates/workspace_tests/tests/cli/submit_retained_inputs.rs|removed_exact=0 added_exact=7 added_weak=3
WeakenedAssertion|crates/workspace_tests/tests/cli/verify_budget_profiles.rs|removed_exact=0 added_exact=8 added_weak=2
DeletedTestDeclaration|crates/workspace_tests/tests/runtime_version_barrier_tests.rs|removed_declarations=12 added_declarations=3
DeletedTestDeclaration|crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs|removed_declarations=3 added_declarations=0
WeakenedAssertion|crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs|removed_exact=18 added_exact=5 added_weak=0
WeakenedAssertion|fuzz/fuzz_targets/kind_validation.rs|removed_exact=2 added_exact=0 added_weak=0
WeakenedAssertion|scripts/check-test-integrity.rs|removed_exact=5 added_exact=7 added_weak=5
```

This is the precise raw result (down from ~373 false positives in the parent draft). It is **not**
the fabricated `real-origin-base-v2.txt` 17-finding list; the parent still lacks the Ask/ko29
restorations, so `vb_ko29_5_public_idempotency.rs` and the archive move
`vb_nf2u_ui_release_acceptance.rs` surface as genuine deletions.

## What changed in the checker

1. **Sound rename mapping (A).** `git diff --name-status -z --find-renames` enumerates `R<score> old new`
   in raw bytes; each pair is handled once. Test-to-test rename scans the actual per-pair
   `git diff --find-renames --text --no-ext-diff --no-textconv --unified=0 BASE -- OLD NEW` hunks
   (fail-closed if git does not resolve the pair as a rename). Exact byte move = no findings;
   modified move only diffs real hunks (no whole-file add). Rename to non-test/archive always
   reports deleted originals. Strict UTF-8/NUL validated **before** any relocation skip, so an
   identical invalid-Rust move still fails. No `news[0]`/unchecked indexing; no D×A subprocess loop.
2. **Hash binding (B).** `hashes_valid` now binds the actual current finding path: if `old_path`
   still exists, `new_path` must equal `old_path` with its exact hash; a different `new_path` or
   `ABSENT` is only valid when the original is actually absent (real reviewed relocation). Both
   LOCAL5090 bypasses (unrelated `new_path`, `ABSENT/ABSENT`) are closed.
3. **Exact-base lifecycle (C).** Base resolves to its full commit hash; only records whose exact
   `base_commit` equals the resolved base are applied. Inactive other-base records are ignored
   (cannot waive, cannot error). Active-base records matching no finding remain stale errors.
   Malformed/duplicate still reject at load.

## Self-test regressions added

exact move PASS · modified-move-retaining-coverage PASS · renamed-weakened FAIL · archive-move FAIL ·
spaces rename · identical-invalid-UTF8 rename FAIL · unrelated-new-path FAIL · ABSENT-while-exists FAIL ·
subsequent-test-change FAIL · changed-base-no-findings PASS · wrong-base-real-finding FAIL.
Existing valid correct-path justification PASS preserved.

## Limits / notes

- **source-length exits 1 for pre-existing reasons unrelated to this bead** (no new exceptions added):
  stale ledger row `crates/workspace_tests/tests/vb_nf2u_ui_release_acceptance.rs` (line 216, file was
  renamed to `docs/history/...`), and two verus files over 800 lines
  (`verification/verus/extern_try_from_parts.rs` 828, `.../vb_xi2f_compile_source_production.rs` 805).
  `scripts/check-test-integrity.rs` itself is covered by a pre-existing exception row; `production`
  scanned 943 with `over_limit=0`.
- `WeakenedAssertion` rows with `removed_exact=0 added_weak>0` come from the pre-existing `added_weak>0`
  heuristic (new files adding weak assertions), not from this repair; same for `scripts/check-test-integrity.rs`
  self-flagging (its own self-test fixture contains `assert_eq!`/`#[test]` substrings).
- Rename findings are attributed: `DeletedTestDeclaration`/`WeakenedAssertion` → old path,
  `IgnoredOrSkippedTest`/`CompileOnlyReplacement` → new path.
- JJ rename per-pair diff is fail-closed (`Err`), never silently dropped.
- No integration justifications written; no broad MoonCI/Kani run (out of scope per coordinator).

## Git/JJ state

- Roots verified: Git toplevel and `jj root` both resolve to
  `…/velvet-ballistics-vb-ndx8b-deepseek-filter-repair`.
- Only `scripts/check-test-integrity.rs` modified (working copy); no commits/bookmarks/push performed
  (root owns publication).
