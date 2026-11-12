# vb-kyyf State 14 Build Cache Cleanup Rerun Report

STATUS: APPROVED

## Scope

- Isolated workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Source checkout `/home/lewis/src/velvet-ballistics` was not touched.
- Production, test, and proof code were not modified.
- Cleanup was limited to the allowed paths listed in the dispatch manifest.

## Commands

```bash
pwd -P
rtk du -sh target-test target .tmp .tlc-metadir vb_kyyf_normalization /tmp/opencode/vb-kyyf-moon-ci-tmp /tmp/opencode/cargo-mutants-* 2>/dev/null || true; rtk df -h . /tmp; rtk df -i . /tmp
rtk ls -ld /home/lewis/src/bd-vb-kyyf-bdd /tmp/opencode; setopt NULL_GLOB; rtk rm -rf -- /home/lewis/src/bd-vb-kyyf-bdd/target-test /home/lewis/src/bd-vb-kyyf-bdd/target /home/lewis/src/bd-vb-kyyf-bdd/.tmp /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir /home/lewis/src/bd-vb-kyyf-bdd/vb_kyyf_normalization /tmp/opencode/vb-kyyf-moon-ci-tmp /tmp/opencode/cargo-mutants-*; rtk mkdir -p /tmp/opencode/vb-kyyf-moon-ci-tmp; setopt NULL_GLOB; rtk du -sh /home/lewis/src/bd-vb-kyyf-bdd/target-test /home/lewis/src/bd-vb-kyyf-bdd/target /home/lewis/src/bd-vb-kyyf-bdd/.tmp /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir /home/lewis/src/bd-vb-kyyf-bdd/vb_kyyf_normalization /tmp/opencode/vb-kyyf-moon-ci-tmp /tmp/opencode/cargo-mutants-* 2>/dev/null || true; rtk df -h /home/lewis/src/bd-vb-kyyf-bdd /tmp; rtk df -i /home/lewis/src/bd-vb-kyyf-bdd /tmp
TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check
TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci
setopt NULL_GLOB; rtk du -sh /home/lewis/src/bd-vb-kyyf-bdd/target-test /home/lewis/src/bd-vb-kyyf-bdd/target /home/lewis/src/bd-vb-kyyf-bdd/.tmp /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir /home/lewis/src/bd-vb-kyyf-bdd/vb_kyyf_normalization /tmp/opencode/vb-kyyf-moon-ci-tmp /tmp/opencode/cargo-mutants-* 2>/dev/null || true; rtk df -h /home/lewis/src/bd-vb-kyyf-bdd /tmp; rtk df -i /home/lewis/src/bd-vb-kyyf-bdd /tmp
```

## Evidence

### Workspace

```text
/home/lewis/src/bd-vb-kyyf-bdd
```

### Pre-Cleanup Space

The initial `du` diagnostic used an unmatched zsh glob for `/tmp/opencode/cargo-mutants-*`, producing `zsh:1: no matches found: /tmp/opencode/cargo-mutants-*`. The filesystem evidence still completed:

```text
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  1.9T  886G  893G  50% /home
tmpfs              62G   49G   13G  80% /tmp
Filesystem        Inodes  IUsed  IFree IUse% Mounted on
/dev/mapper/root       0      0      0     - /home
tmpfs            1048576 492540 556036   47% /tmp
```

### Cleanup Evidence

```text
/home/lewis/src/bd-vb-kyyf-bdd/
/tmp/opencode/
0	/tmp/opencode/vb-kyyf-moon-ci-tmp
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  1.9T  867G  911G  49% /home
tmpfs              62G   49G   13G  80% /tmp
Filesystem        Inodes  IUsed  IFree IUse% Mounted on
/dev/mapper/root       0      0      0     - /home
tmpfs            1048576 490912 557664   47% /tmp
```

### Required Gates

`cargo fmt --check` passed with no output.

`moon ci` passed:

```text
Tasks: 21 completed (4 cached)
 Time: 2m 28s 960ms
```

Full `moon ci` output was captured by opencode at:

```text
/home/lewis/.local/share/opencode/tool-output/tool_e3d190dae001y4v23yqYkJU6nQ
```

### Final Space

```text
11G	/home/lewis/src/bd-vb-kyyf-bdd/target
20M	/tmp/opencode/vb-kyyf-moon-ci-tmp
Filesystem        Size  Used Avail Use% Mounted on
/dev/mapper/root  1.9T  874G  905G  50% /home
tmpfs              62G   44G   18G  72% /tmp
Filesystem        Inodes  IUsed  IFree IUse% Mounted on
/dev/mapper/root       0      0      0     - /home
tmpfs            1048576 495790 552786   48% /tmp
```

## Result

vb-kyyf State 14 landing can rerun. The disk-quota build-cache blocker is cleared for this isolated workspace rerun, and `moon ci` passed under the required environment.
