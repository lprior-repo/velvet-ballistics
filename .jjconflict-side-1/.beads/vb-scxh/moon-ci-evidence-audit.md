# Moon CI Evidence Audit: vb-scxh

STATUS: APPROVED

## Audited Artifact

- Artifact path: `.beads/vb-gvmt/moon-ci-or-static-scan-report.md`
- Ledger path: `.beads/vb-gvmt/verification-ledger.jsonl`

## Required Markers

| marker | observed raw evidence |
|---|---|
| command `moon ci` | PRESENT in report lines 5-7 and ledger row `kind=moon-ci` |
| PASS | PRESENT: `Status: PASS` |
| 19 tasks | PRESENT: `Tasks: 19 completed (1 cached)` |
| 8276/8276 tests | PRESENT: `nextest 8276 tests run: 8276 passed, 0 skipped` and ledger `8276 tests passed` |
| runtime | PRESENT: `Time: 1m 37s 538ms` |
| artifact path | PRESENT by audit path only; report does not self-identify its own path |
| fresh rerun marker | PRESENT and PASS: `TMPDIR=/home/lewis/src/vb-scxh/target/tmp RUSTC_WRAPPER= moon ci --force --summary normal` exited 0 on 2026-05-14 from `/home/lewis/src/vb-scxh` |

Validation on 2026-05-14: required artifacts existed and JSONL ledgers parsed with `jq -c .`; this validates syntax/presence only, not CI success.

## State 11 Fresh Rerun Evidence

Fresh rerun attempt 6, post Moon CI source repair forced freshness probe:

```text
TMPDIR=/home/lewis/src/vb-scxh/target/tmp RUSTC_WRAPPER= moon ci --force --summary normal
```

Result: PASS, exit `0`. Summary markers:

```text
pass RunTask(velvet-ballastics:agent-cli-contract) (62ms, 2c7f7221)
pass RunTask(velvet-ballastics:fuzz-smoke) (170ms, c7842f37)
pass RunTask(velvet-ballastics:lint-src) (1s 138ms, 99c2dfcf)
pass RunTask(velvet-ballastics:source-length) (1s 402ms, 002f7ee9)
pass RunTask(velvet-ballastics:fmt) (1s 846ms, 2a367c99)
pass RunTask(velvet-ballastics:nightly-feature-gate) (3s 963ms, 096b388d)
pass RunTask(velvet-ballastics:check) (163ms, 937c9cba)
pass RunTask(velvet-ballastics:nightly-feature-cargo-probe) (24ms, 68327fa4)
pass RunTask(velvet-ballastics:feature-powerset) (4s 457ms, f73e9329)
pass RunTask(velvet-ballastics:coverage) (7s 866ms, aae27c63)
pass RunTask(velvet-ballastics:bench-build) (7s 908ms, cc71bd49)
pass RunTask(velvet-ballastics:mutants-smoke) (13s 153ms, af1a6180)
pass RunTask(velvet-ballastics:miri) (19s 877ms, 98d4ae4b)
pass RunTask(velvet-ballastics:test) (27s 964ms, 77014bd2)
pass RunTask(velvet-ballastics:hardened-build) (205ms, 7537c1cb)
pass RunTask(velvet-ballastics:doc-test) (1s 383ms, 0c8572dc)
pass RunTask(velvet-ballastics:doc) (2s 389ms, 2d21c395)
pass RunTask(velvet-ballastics:maxperf) (2s 743ms, aee33bed)
pass RunTask(velvet-ballastics:maxperf-native) (2s 745ms, 1de9bd78)
Actions: 21 completed
Time: 34s 838ms
```

Test lane marker:

```text
Nextest run ID 084a71cb-efd5-4dd3-9c50-13d96a71a9fc with nextest profile: default
Starting 8185 tests across 95 binaries (6 tests skipped)
Summary [  26.466s] 8185 tests run: 8185 passed, 6 skipped
```

Artifact-path marker for this lane: `.beads/vb-scxh/moon-ci-evidence-audit.md` is the current State 11 CI evidence report. Historical reference artifact remains `.beads/vb-gvmt/moon-ci-or-static-scan-report.md`; it is no longer the only CI evidence because the fresh forced rerun above passed.

Fresh rerun attempt 1:

```text
moon ci
```

Result: no usable PASS evidence. Moon reported `No tasks affected by changed files. Unable to execute action pipeline.`, `Requested targets: 30`, `Resolved targets: 0`.

Fresh rerun attempt 2:

```text
moon ci --force --summary normal
```

Result: FAIL. Summary markers:

```text
fail RunTask(velvet-ballastics:fmt) (1s 676ms, 8c38de8d)
fail RunTask(velvet-ballastics:lint-src) (2s 322ms, c69bee08)
fail RunTask(velvet-ballastics:fuzz-smoke) (2s 643ms, 8f34d24a)
fail RunTask(velvet-ballastics:check) (1s 137ms, 42df0c1e)
Actions: 6 completed, 4 failed, 11 skipped
Time: 26s 923ms
```

Observed blockers included rustfmt diffs in `crates/vb_compile/src/expression.rs` and repeated local disk quota errors such as `error writing dependencies to /tmp/sccache*/deps.d: Disk quota exceeded (os error 122)`. Production/test/source edits are forbidden for this bead, so the fmt blocker was not repaired here.

Fresh rerun attempt 3, cheap canonical task slice:

```text
TMPDIR=/home/lewis/src/vb-scxh/.beads/vb-scxh/tmp RUSTC_WRAPPER= moon run :fmt --summary normal
```

Result: FAIL. `rustup run nightly-2026-04-28 cargo fmt --all --check` returned exit code 1 and showed rustfmt diffs in `crates/vb_compile/src/expression.rs` at the float parse / `FiniteF64::new` blocks. This is not a disk-quota symptom.

Fresh rerun attempt 4, canonical forced CI with repo-local temp and disabled wrapper:

```text
TMPDIR=/home/lewis/src/vb-scxh/.beads/vb-scxh/tmp RUSTC_WRAPPER= moon ci --force --summary normal
```

Result: FAIL. Summary markers:

```text
pass RunTask(velvet-ballastics:agent-cli-contract)
pass RunTask(velvet-ballastics:source-length)
fail RunTask(velvet-ballastics:fmt) (1s 635ms, be362cbc)
pass RunTask(velvet-ballastics:nightly-feature-gate)
fail RunTask(velvet-ballastics:lint-src) (9s 730ms, 3fe90829)
fail RunTask(velvet-ballastics:check) (8s 479ms, 6955ae05)
fail RunTask(velvet-ballastics:fuzz-smoke) (15s 847ms, 8f34d24a)
pass RunTask(velvet-ballastics:miri) (20s 519ms, 808b9f06)
Actions: 6 completed, 4 failed, 11 skipped
Time: 20s 519ms
```

Key failure packets from attempt 4:

- `fmt`: rustfmt diff remains in `/home/lewis/src/vb-scxh/crates/vb_compile/src/expression.rs`.
- `lint-src` and `check`: `error: couldn't read crates/vb_runtime/src/runtime/chunk_001.rs: No such file or directory (os error 2)` from `crates/vb_runtime/src/runtime.rs:4:1 include!("runtime/chunk_001.rs");`.
- `fuzz-smoke`: same missing `crates/vb_runtime/src/runtime/chunk_001.rs` during fuzz build.
- Disk quota: not observed in this repo-local-temp / `RUSTC_WRAPPER=` rerun; the remaining forced-CI blockers are source/workspace state blockers, not merely `/tmp` quota.

Fresh rerun attempt 5, final narrow State 11 forced freshness probe:

```text
TMPDIR=/home/lewis/src/vb-scxh/.beads/vb-scxh/tmp RUSTC_WRAPPER= moon ci --force --summary normal
```

Result: FAIL, exit `1`. Summary markers:

```text
pass RunTask(velvet-ballastics:agent-cli-contract)
pass RunTask(velvet-ballastics:source-length)
fail RunTask(velvet-ballastics:fmt)
fail RunTask(velvet-ballastics:lint-src)
pass RunTask(velvet-ballastics:nightly-feature-gate)
fail RunTask(velvet-ballastics:check)
fail RunTask(velvet-ballastics:fuzz-smoke)
pass RunTask(velvet-ballastics:miri)
Actions: 6 completed, 4 failed, 11 skipped
```

Final failure packets:

- `fmt`: local source formatting repair needed in `crates/vb_compile/src/expression.rs` around float parse and `FiniteF64::new` blocks. Classification: `FAIL_LOCAL`, owner source-format repair agent; not deferred-global.
- `lint-src`: missing generated/runtime chunk `crates/vb_runtime/src/runtime/chunk_001.rs` included from `crates/vb_runtime/src/runtime.rs:4:1`. Classification: `FAIL_LOCAL`, owner runtime generation/source repair agent; not deferred-global.
- `check`: same missing runtime chunk blocks lib and lib-test compilation. Classification: `FAIL_LOCAL`, owner runtime generation/source repair agent; not deferred-global.
- `fuzz-smoke`: same missing runtime chunk blocks fuzz build. Classification: `FAIL_LOCAL` because required fresh CI evidence cannot be produced; fuzz adequacy beyond this compile blocker remains unproven.
- Disk quota: not observed in attempt 5 with repo-local `TMPDIR` and disabled `RUSTC_WRAPPER`.

## Moon Task Availability / Canonical Command

- `.moon/workspace.yml` maps project `velvet-ballastics` to `.` and default project `velvet-ballastics`.
- `.moon/tasks/all.yml` defines CI-enabled tasks including `fmt`, `lint-src`, `check`, `test`, `doc-test`, `feature-powerset`, `fuzz-smoke`, `miri`, coverage/build lanes, plus gauntlet rollups.
- Canonical green evidence command for this obligation remains the approved obligation command: `moon ci` from `/home/lewis/src/vb-scxh`. Because normal `moon ci` previously resolved zero changed targets, `moon ci --force --summary normal` is the valid forced freshness probe; the post-repair rerun passed with repo-local `TMPDIR` and `RUSTC_WRAPPER=`.

## Classification

- `CI-SCXH-001`: `PASS` for State 11 fresh Moon CI evidence. The current forced rerun passed all 21 Moon actions and recorded the current command, artifact path, action count, and test count markers.
- This does not close `vb-scxh` or unblock `vb-engine-yaml` because the safety anchor remains `FAIL_LOCAL` / `BLOCK_LOCAL` and State 12 was not executed.
