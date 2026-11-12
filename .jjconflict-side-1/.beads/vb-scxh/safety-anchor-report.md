# Safety Anchor Report: vb-scxh

STATUS: REJECTED

## Decision

The intended rescue bundle and rescue ref were not found or verified. Keep `SAFETY-SCXH-001` and `ERR-SCXH-006` as `FAIL_LOCAL` / `BLOCK_LOCAL`; do not mark the safety anchor PASS.

State 11 rerun on 2026-05-14 reconfirmed the same result after Moon CI source repair. This is a waiver candidate only for an owner decision; it is not approved, not waived, and not adequate close/unblock evidence.

## Exact Target

- Bundle: `/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle`
- Ref: `rescue-vb-scxh-ci-green-20260513T030158Z`
- Isolated worktree: `/home/lewis/src/vb-scxh`
- Source checkout: `/home/lewis/src/Velvet-ballistics`

## Raw Verification Evidence

### Exact bundle verify

- Workdir: `/home/lewis/src/vb-scxh`
- Command: `git bundle verify "/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle"`
- Exit: `1`
- Output:

```text
error: could not open '/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle'
```

### Post-Moon-CI-repair exact chained check

- Workdir: `/home/lewis/src/vb-scxh`
- Command: `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle && git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`
- Exit: `1`
- Output:

```text
error: could not open '/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle'
```

### Exact ref lookup in isolated worktree

- Workdir: `/home/lewis/src/vb-scxh`
- Git dir command: `git rev-parse --git-dir`
- Git dir output:

```text
/home/lewis/src/.git/worktrees/vb-scxh
```

- Ref command: `git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`
- Exit: `1`
- Ref output:

```text
(no stdout)
```

- For-each-ref search command: `git for-each-ref --format='%(refname) %(objectname)' | rg 'rescue-vb-scxh-ci-green-20260513T030158Z'`
- For-each-ref search output:

```text
(no stdout)
```

### Exact ref lookup in source checkout

- Workdir: `/home/lewis/src/Velvet-ballistics`
- Git dir command: `git rev-parse --git-dir`
- Git dir output:

```text
/home/lewis/src/.git
```

- Ref command: `git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`
- Exit: `1`
- Ref output:

```text
(no stdout)
```

- For-each-ref search command: `git for-each-ref --format='%(refname) %(objectname)' | rg 'rescue-vb-scxh-ci-green-20260513T030158Z'`
- For-each-ref search output:

```text
(no stdout)
```

## Bundle Search Evidence

### Targeted glob searches

Final narrow pass additions:

- Tool query: `glob Velvet-ballistics-rescue-20260513T022011Z.bundle` under `/home/lewis/src`.
- Result: no files found.
- Tool query: `glob *.bundle` under `/home/lewis/src`.
- Result: no files found.
- Tool query: `glob **/Velvet-ballistics-rescue-20260513T022011Z.bundle` under `/home/lewis/src/vb-scxh`.
- Result: no files found.
- Tool query: `glob **/*rescue-vb-scxh-ci-green*` under `/home/lewis/src/vb-scxh`.
- Result: no files found.

- Tool query: `glob **/*Velvet-ballistics-rescue-20260513T022011Z.bundle` under `/home/lewis/src`
- Result: no files found.
- Tool query: `glob **/*20260513T02*.bundle` under `/home/lewis/src`
- Result: no files found.
- Tool query: `glob **/*20260513T03*.bundle` under `/home/lewis/src`
- Result: no files found.
- Tool query: `glob **/*Velvet-ballistics-rescue-20260513T022011Z.bundle` under `/tmp/opencode`
- Result: no files found.
- Tool query: `glob **/*20260513T02*.bundle` under `/tmp/opencode`
- Result: no files found.
- Tool query: `glob **/*20260513T03*.bundle` under `/tmp/opencode`
- Result: no files found.
- Tool query: same three patterns under `/home/lewis`
- Result: no bundle match returned; search encountered permission-denied subtrees including `/home/lewis/tandoor/postgresql` and container overlay work directories.

### Constrained command searches

- Workdir: `/home/lewis/src/vb-scxh`
- Command: `rg --files "/home/lewis/src" -g "*.bundle"`
- Output:

```text
(no stdout)
```

- Command: `rg --files "/tmp/opencode" -g "*.bundle"`
- Output:

```text
(no stdout)
```

- Command: `rg --files "/home/lewis" -g "*.bundle" -g "!**/.local/share/containers/**" -g "!**/tandoor/postgresql/**"`
- Output:

```text
rg: /home/lewis/tandoor/postgresql: Permission denied (os error 13)
```

The constrained `/home/lewis` scan emitted no bundle path on stdout and hit the permission-denied subtree shown above.

## Final Low-Output State 11 Probe: 2026-05-14

- Path guard: `pwd` from `/home/lewis/src/vb-scxh` returned `/home/lewis/src/vb-scxh`; guard exit `0`.
- Exact bundle recheck from isolated worktree: `test -e /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle` returned nonzero; `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle` failed with `error: could not open '/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle'`.
- Exact ref recheck: `git show-ref rescue-vb-scxh-ci-green-20260513T030158Z` exited `1` with no stdout in both `/home/lewis/src/vb-scxh` and read-only `/home/lewis/src/Velvet-ballistics`.
- Bundle search: `glob **/*.bundle` found no files under `/home/lewis/src` or `/tmp/opencode`.
- Plausible alternate refs seen in both worktree and read-only source checkout, but none match the required exact ref: `main c6272854a`, `origin/main c6272854a`, `origin/release-clean-main 3b3d4218d`, `origin/rescue-main-pre-recovery-20260513T022011Z e1d254daf`, `origin/rescue/vb-zdxm-base-20260511T161708Z dd9ceba60`.
- Existing report search found only repeated references to the missing exact bundle/ref and no owner-approved waiver. The candidate waiver remains `CANDIDATE_ONLY_NOT_APPROVED`.
- Owner action required before any State 12 close/unblock decision: restore the exact bundle and exact ref, or explicitly approve a waiver that names the missing-anchor risk and accepted alternate immutable evidence.

## Classification

- `SAFETY-SCXH-001`: `FAIL_LOCAL` / `BLOCK_LOCAL`.
- `ERR-SCXH-006`: `FAIL_LOCAL` / `BLOCK_LOCAL` mapped to `Error::SafetyAnchorMissing`.
- State 12 close/unblock remains blocked unless the bundle/ref are repaired and rerun or an approved waiver exists.
- Waiver posture: candidate row may be reviewed by owner because reasonable scoped searches found no bundle/ref, but evidence policy does not permit this agent to approve it. The default remains repair/restore, not waiver.
