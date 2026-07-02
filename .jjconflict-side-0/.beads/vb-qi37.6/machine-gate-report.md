# State 11 Machine Gate Report — vb-qi37.6 integration repair

STATUS: PASS

## Workspace

- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workspace: `/home/lewis/src/vb-ws/vb-qi37.6-integration`
- base: `origin/main` at `6cb83882`

## Commands

- `TMPDIR=/home/lewis/src/tmp_build/vb-qi37.6-integration-moon CXXFLAGS=-pipe CFLAGS=-pipe RUSTC_WRAPPER= moon ci --force`
- Result: PASS, `Tasks: 20 completed`, `8414 tests run: 8414 passed, 6 skipped`.
- Prior retry evidence: first forced run failed only from missing TMPDIR/`/tmp` tool temp errors; rerun after creating TMPDIR and using `CXXFLAGS=-pipe CFLAGS=-pipe` passed.

## Classification

- No local, regression, release, or required-obligation blocker remains after the integration repair.
