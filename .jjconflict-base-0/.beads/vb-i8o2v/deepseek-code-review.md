# Code Review — scripts/check-test-integrity.rs (vb-i8o2v corrected)

**Bead:** vb-i8o2v (corrected independent semantic review)
**Reviewer:** DeepSeek (local)
**Artifact under review:** checker at blob `d2a64ab1a4d8ded0bf005dc0704a8dbe815a2721`
**Base:** `da633fe0bbde9e414173d5f7aabe314a37f8752d`
**Scope:** metadata repair + independent review of the checker source. No production/checker/test source edits were made. No VCS/DB mutations; publication is root-owned.

## Corrections to the prior 3090 report (vb-bxzxu)

The prior report claimed "58 assertions passed (40 main + 18 bypass/lifecycle)" and "35 findings". Both are unsupported by the raw logs.

Actual counts (fresh run via official runner, evidence `.beads/vb-i8o2v/evidence/self-test.stdout`):

| Metric | Prior claim | Actual |
|---|---|---|
| Named self-test PASS (`^SelfTest:.*: PASS`) | 58 | **33** |
| FAIL | — | **0** |
| Total `PASS` markers | 40+18 | **40** (33 test-case + 7 inner-checker) |
| Total `SelfTest:` diagnostic lines | — | **50** (lines, not tests) |
| Raw base findings | 35 | **33** |
| Unique base findings | — | **32** (one duplicate `return;` skip in chunk_002) |

Finding breakdown (actual, matches `.beads/vb-bxzxu/coordinator-evidence-audit.json`):
`WeakenedAssertion 19, IgnoredOrSkippedTest 6, DeletedTestDeclaration 6, DeletedTestFile 2` (= 33 raw).

## Hash-binding probes

The prior probes (`hash-probe-bypass-current.py`, `hash-probe-comprehensive-current.py`) were invalid: they used literal `baseblob`/`currentblob` placeholders and the unsupported `--root` flag, so their rejections could not establish sound hash binding.

Replaced with `.beads/vb-i8o2v/evidence/hash-probe-valid.py`, which uses real 40-hex blobs, `/usr/bin/git`, temp repos, and `--base` only. Each variation is preceded by its baseline (correct record → exit 0).

Four behaviors verified against the current checker:

| Case | Expected | Actual |
|---|---|---|
| unrelated `new_path` | exit 1 | exit 1 |
| `new_path=ABSENT` while old file exists | exit 1 | exit 1 |
| exact same path (correct record) | exit 0 | exit 0 |
| subsequently changed source | exit 1 | exit 1 |

Evidence: `.beads/vb-i8o2v/evidence/hash-probe.stdout` — `hash-probe: ALL PASS`, exit 0.

## Metadata edits (this bead)

`scripts/test-integrity-justifications.tsv` — two replacement-path hash bindings corrected (details in test-suite review): flux_sequence → split `tests.rs`; qi37 → adds ko29 test + fixture. Re-ran the base gate: exit 0.

## Verdict

The checker's hash-binding logic (`JustificationRecord::hashes_valid`) correctly enforces "current finding path must bind itself, not ABSENT, when it exists", rejects unrelated `new_path`, and re-flags a subsequently changed source. The 32 records bind real hashes after correction.

**APPROVE** (metadata-only). No checker source defect found.
