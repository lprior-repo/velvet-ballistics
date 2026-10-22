# Bead vb-x6o83: Test Task TMPDIR Fix

## Problem

The `test` task in `.moon/tasks/all.yml` forces `TMPDIR="$PWD/target/tmp"`, which places test temporary files on disk. On systems where `/tmp` is tmpfs (like the target workstation), this causes storage worker tests to stall and timeout (30s timeout observed for PS001 tests) versus completing in ~1.8s with the default TMPDIR.

## Scope

- Fix ONLY the `test` task in `.moon/tasks/all.yml`
- Add a self-test for the temporary directory handling
- Do NOT modify other tasks (build, lint, check, etc.) unless explicitly required
- Do NOT modify test code, production code, or storage assertions
- Do NOT claim proved root cause (Fjall deadlock/fsync) - actual cause unknown

## Solution

The test task should:
1. Create a fresh unique test-only tmpdir using `mktemp -d` under the user's actual TMPDIR (defaults to `/tmp`)
2. Pass this unique directory to the test runner via `TMPDIR`
3. Clean up ONLY the fresh directory created by the task on exit (success or failure)
4. Preserve any user-supplied TMPDIR override
5. Use robust shell quoting for paths with spaces
6. Propagate the exact exit code from the test command

## Acceptance Criteria

1. Test task uses fresh unique tmpdir under system TMPDIR, not `target/tmp`
2. Cleanup occurs on both success and failure paths
3. User TMPDIR override is honored
4. Self-test validates: unique directory creation, TMPDIR propagation, cleanup on success/failure, exit code propagation
5. Actual test run via Moon completes with 14349 tests (or equivalent full suite count)
6. No test assertion failures
7. Source formatting passes (`moon run :fmt`)
8. No unsafe/unwrap/expect/panic/todo/dbg in any changes

## Verification Plan

1. Write shell self-test extracting the actual Moon test script with stubs
2. Validate unique directory creation, TMPDIR propagation, cleanup behavior
3. Invoke actual test task via Moon runner with bounded 300s timeout
4. Verify test count and pass rate
5. Run format check

## Limitations

- Cannot prove Fjall deadlock/fsync root cause without deeper investigation
- tmpfs durability characteristics differ from disk; tests still verify storage behavior
- Timeout increased from 10m to accommodate disk-based environments
