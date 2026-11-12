# Miri CI Smoke Contract

## Problem
The Miri CI smoke is false-green. The filter targets the obsolete test ID `ids::tests::run_id_zero_constant`, which no longer exists. The actual current ID is `ids::tests_and_verification::run_id_zero_constant`. As a result, zero tests run and the job passes vacuously.

## Required Behavior
The Miri smoke script must use the exact current test ID with `-- --exact`. It must fail unless the actual test summary is exactly `1 passed; 0 failed; 0 ignored`. The script must reject zero tests run, missing or malformed output, and any nonzero Miri exit code.

## Environment Preservation
The script must preserve the pinned toolchain `nightly-2026-04-28`, the target `vb_core --lib --all-features`, `RUSTFLAGS=-Dwarnings`, and the existing `MIRIFLAGS` including `-Zmiri-disable-isolation`. The three-minute inner timeout cap must remain.

## Output Capture
The script must capture actual stdout via `tee` under `pipefail` and retain the raw log for inspection.

## Gate Integration
The existing `moon :miri` gate must invoke the owned script with the same environment and lockfile. The gate must not weaken any checks.

## Filter Override
The CI filter is fixed. Deliberate bounded negative tests invoke the old filter separately and feed its transcript to the same checker.

## Self-Test
The gate must invoke a separate shell transcript checker that can be exercised by bounded tooling tests. Fixtures must cover: pass with one test, reject with zero tests, fail with one failure, and missing output. An actual invocation with the old zero-match filter must prove failure.

## Scope and Limitations
This smoke runs a single identified test and does not prove the absence of Miri UB across the codebase. The full filtered test count is 2386 with the default seed.

## Development Process
All development must be isolated to the current Git/JJ path. Code and tests must be reviewed in independent local models.
Coordinator clarification: CI accepts no filter override. Separate checker supports deterministic adversarial transcript tests.
