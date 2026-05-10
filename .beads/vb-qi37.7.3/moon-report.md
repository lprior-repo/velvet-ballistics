STATUS: FAIL

# Machine Gate Report: vb-qi37.7.3

## Workspace

- Control-plane artifact root: `/home/lewis/src/Velvet-ballistics`
- Implementation workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`
- Canonical gate full output: `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci.out`
- Forced gate full output: `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci-force.out`

## Inputs Read

- `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/STATE.md`
- `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/implementation.md`
- previous `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-report.md`

## Command 1: Canonical Gate With Velvet Git Environment

Run from `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`:

```bash
GIT_DIR=/home/lewis/src/Velvet-ballistics/.git GIT_WORK_TREE=/home/lewis/src/Velvet-ballistics-femdation-p0p1-25 moon ci; rc=$?; printf '\nEXIT_STATUS:%s\n' "$rc"
```

Output saved to `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci.out`.

Observed excerpt:

```text
217: Action count: 1
218: Requested targets: 30
249: Resolved targets: 0
251: EXIT_STATUS:0
```

Result: canonical `moon ci` still no-ops in this non-colocated Velvet Git environment. The forced gate is the meaningful machine gate.

## Command 2: Forced Canonical Gate With Velvet Git Environment

Run from `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`:

```bash
GIT_DIR=/home/lewis/src/Velvet-ballistics/.git GIT_WORK_TREE=/home/lewis/src/Velvet-ballistics-femdation-p0p1-25 moon ci --force; rc=$?; printf '\nEXIT_STATUS:%s\n' "$rc"
```

Output saved to `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci-force.out`.

Observed excerpt:

```text
212: Action count: 26
213: Requested targets: 30
244: Resolved targets: 20
23277: Tasks: 16 completed, 2 failed, 2 skipped
23278:  Time: 5m 42s 410ms
23281: EXIT_STATUS:1
```

## First Failure Category

`TEST_FAILURE`

Rationale: `fmt`, `lint-src`, `supply-chain`, `nightly-feature-cargo-probe`, and `check` completed before the first actionable red gate. The first actionable failure in the forced output is `velvet-ballastics:test`, where one `vb_validate` test fails.

Earlier gate pass/completion evidence from `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci-force.out`:

```text
497: ▮▮▮▮ velvet-ballastics:fmt (1s 23ms, f904bc67)
1081: ▮▮▮▮ velvet-ballastics:lint-src (1s 960ms, a984be9b)
2242:                       velvet-ballastics:check |     Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s
2243: ▮▮▮▮ velvet-ballastics:nightly-feature-cargo-probe (272ms, 0d478735)
2626: ▮▮▮▮ velvet-ballastics:check (301ms, 172f3a5f)
9876: ▮▮▮▮ velvet-ballastics:supply-chain (57s 806ms, 5cb34bcc)
```

First actionable failure excerpt:

```text
20034:                        velvet-ballastics:test |         FAIL [   0.004s] ( 9279/10841) vb_validate gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence
20049:                        velvet-ballastics:test |     thread 'gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence' (1746966) panicked at crates/vb_validate/src/gate_08_accessor.rs:399:5:
20050:                        velvet-ballastics:test |     Test failed: assertion failed: `(left == right)` 
20051:                        velvet-ballastics:test |       left: `Err(AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`,
20052:                        velvet-ballastics:test |      right: `Ok(())` at crates/vb_validate/src/gate_08_accessor.rs:485.
20053:                        velvet-ballastics:test |     minimal failing input: slot_count = 2, root = 0
20353:                        velvet-ballastics:test |      Summary [  22.703s] 9309/10841 tests run: 9308 passed, 1 failed, 0 skipped
20356:                        velvet-ballastics:test | error: test run failed
20357: ▮▮▮▮ velvet-ballastics:test (40s 720ms, efc13b58)
```

Later non-first failure observed in the same forced output:

```text
22451:                        velvet-ballastics:miri | error: unsupported operation: `getcwd` not available when isolation is enabled
22497:                        velvet-ballastics:miri | error: test failed, to rerun pass `-p vb_validate --lib`
22502: ▮▮▮▮ velvet-ballastics:miri (4m 46s 712ms, efe40622)
```

## Post-forced Stability Verification

### Exact formatter check

Run from `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25` after the forced gate:

```bash
GIT_DIR=/home/lewis/src/Velvet-ballistics/.git GIT_WORK_TREE=/home/lewis/src/Velvet-ballistics-femdation-p0p1-25 rustup run nightly-2026-04-28 cargo fmt --all --check; rc=$?; printf '\nEXIT_STATUS:%s\n' "$rc"
```

Observed stdout/stderr:

```text

EXIT_STATUS:0
```

Result: PASS. Exact Moon formatter check remained stable after the forced gate.

### Public API presence

Run from `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25` after the forced gate:

```bash
rtk grep -n 'pub fn validate_symbol_references|pub fn validate_resource_references|pub fn validate_action_references' 'crates/vb_core/src/workflow/mod.rs' 'crates/vb_validate/src/shared.rs'; rc=$?; printf '\nEXIT_STATUS:%s\n' "$rc"
```

Observed stdout/stderr:

```text
3 matches in 2F:

[file] crates/vb_core/src/workflow/mod.rs (2):
   724: pub fn validate_resource_references(parts: &WorkflowParts) -> Result<(), Work...
   729: pub fn validate_symbol_references(parts: &WorkflowParts) -> Result<(), Workfl...

[file] crates/vb_validate/src/shared.rs (1):
   156: pub fn validate_action_references(


EXIT_STATUS:0
```

Result: PASS. Required public APIs still exist after the forced gate.

## Artifact Updates

- `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci.out` overwritten with the new canonical gate output.
- `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci-force.out` overwritten with the new forced gate output.
- `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-report.md` updated with this report.
- `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/ci-failure-category.txt` updated to exactly `TEST_FAILURE`.

## Verdict

FAIL. Canonical `moon ci` no-oped with `EXIT_STATUS:0` and `Resolved targets: 0`. Forced `moon ci --force` reached real tasks and failed with `EXIT_STATUS:1`. First actionable forced-gate failure category: `TEST_FAILURE`.
