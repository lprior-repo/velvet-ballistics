# vb-kyyf State 14 Quota Cleanup Rerun Report

STATUS: REJECTED

## Scope

- Isolated workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Source checkout forbidden path was not touched: `/home/lewis/src/velvet-ballistics`
- Cleanup was limited to:
  - `/tmp/opencode/vb-kyyf-moon-ci-tmp`
  - `/tmp/opencode/cargo-mutants-*`
  - `/home/lewis/src/bd-vb-kyyf-bdd/.tmp`

## Required Commands

- `pwd -P`
  - Output: `/home/lewis/src/bd-vb-kyyf-bdd`
- `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check`
  - Result: PASS, no output
- `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci`
  - Result: FAIL

## Pre-Cleanup Evidence

Command: `rtk df -h /tmp/opencode && rtk df -i /tmp/opencode && rtk du -sh /tmp/opencode/vb-kyyf-moon-ci-tmp /tmp/opencode/cargo-mutants-* /home/lewis/src/bd-vb-kyyf-bdd/.tmp`

```text
Filesystem      Size  Used Avail Use% Mounted on
tmpfs            62G   49G   13G  80% /tmp
Filesystem      Inodes  IUsed  IFree IUse% Mounted on
tmpfs          1048576 490913 557663   47% /tmp
zsh:1: no matches found: /tmp/opencode/cargo-mutants-*
```

## Cleanup Evidence

Command: `rtk ls -ld /tmp/opencode /home/lewis/src/bd-vb-kyyf-bdd && setopt NULL_GLOB && rm -rf /tmp/opencode/vb-kyyf-moon-ci-tmp /tmp/opencode/cargo-mutants-* /home/lewis/src/bd-vb-kyyf-bdd/.tmp && mkdir -p /tmp/opencode/vb-kyyf-moon-ci-tmp && rtk df -h /tmp/opencode && rtk df -i /tmp/opencode && rtk du -sh /tmp/opencode/vb-kyyf-moon-ci-tmp /home/lewis/src/bd-vb-kyyf-bdd/.tmp`

```text
/home/lewis/src/bd-vb-kyyf-bdd/
/tmp/opencode/
Filesystem      Size  Used Avail Use% Mounted on
tmpfs            62G   49G   13G  80% /tmp
Filesystem      Inodes  IUsed  IFree IUse% Mounted on
tmpfs          1048576 489287 559289   47% /tmp
du: cannot access '/home/lewis/src/bd-vb-kyyf-bdd/.tmp': No such file or directory
0	/tmp/opencode/vb-kyyf-moon-ci-tmp
```

## Post-Failure Evidence

Command: `rtk df -h /tmp/opencode && rtk df -i /tmp/opencode && setopt NULL_GLOB && rtk du -sh /tmp/opencode/vb-kyyf-moon-ci-tmp /tmp/opencode/vb-kyyf-moon-ci-tmp/cargo-mutants-* /tmp/opencode/cargo-mutants-* /home/lewis/src/bd-vb-kyyf-bdd/.tmp`

```text
Filesystem      Size  Used Avail Use% Mounted on
tmpfs            62G   49G   13G  80% /tmp
Filesystem      Inodes  IUsed  IFree IUse% Mounted on
tmpfs          1048576 490915 557661   47% /tmp
du: cannot access '/home/lewis/src/bd-vb-kyyf-bdd/.tmp': No such file or directory
11M	/tmp/opencode/vb-kyyf-moon-ci-tmp
```

Command: `rtk quota -s`

```text
[rtk: No such file or directory (os error 2)]
```

## Blocker

`moon ci` failed in `velvet-ballastics:mutants-smoke` while copying into the external cargo-mutants temp directory:

```text
Error: Failed to copy /home/lewis/src/bd-vb-kyyf-bdd/target-test/debug/deps/accepted_artifact_red_phase-f64f6994ca2a8121 to /tmp/opencode/vb-kyyf-moon-ci-tmp/cargo-mutants-bd-vb-kyyf-bdd-8p7npn.tmp/target-test/debug/deps/accepted_artifact_red_phase-f64f6994ca2a8121

Caused by:
    Disk quota exceeded (os error 122)
```

## Decision

vb-kyyf State 14 landing cannot rerun as approved from this sublane. The environment still rejects cargo-mutants temp copies with `Disk quota exceeded (os error 122)` despite allowed temp cleanup and 13G reported free on `/tmp`.
