bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 1
updated_at: 2026-05-14T22:14:30Z
attempt: 1-of-7

# Baseline Report

## Workspace

- source_checkout: `/home/lewis/src/Velvet-ballistics`
- isolated_workspace: `/home/lewis/src/vb-qi37-13-r2`
- forbidden_partial_workspace: `/home/lewis/src/vb-qi37-13`
- bead db: `/home/lewis/src/.beads/dolt`

## Commands captured

### Bead show

Command:

```bash
bd --db "/home/lewis/src/.beads/dolt" show vb-qi37.13 --json
```

Evidence summary: bead `vb-qi37.13` exists, title `cli: Reconcile structured output contract`, status `open` before state update, assignee `Lewis`, acceptance criteria require stable structured output and exit codes matching master-doc table.

### Bead in-progress update

Command:

```bash
bd --db "/home/lewis/src/.beads/dolt" update vb-qi37.13 --status in_progress --json
```

Evidence summary: command returned bead with status `in_progress` and updated_at `2026-05-14T22:14:14Z`.

### Workspace path and VCS

Command:

```bash
pwd -P && jj workspace list && jj status
```

Output summary:

```text
/home/lewis/src/vb-qi37-13-r2
Internal error: The repository appears broken or inaccessible
Caused by:
1: Failed to read commit backend type
2: Cannot access /home/lewis/.jj/repo/store/type
3: No such file or directory (os error 2)
```

Fallback command:

```bash
git status --short && git rev-parse HEAD && git branch --show-current
```

Output summary:

```text
c6272854a341ff3e5017db2aae703aa6d1483d7f
```

No `git status --short` lines were emitted.

### Focused exit-code baseline

Command:

```bash
TMPDIR="/home/lewis/src/vb-qi37-13-r2/target/tmp" RUSTC_WRAPPER= cargo test -p velvet_ballastics exit_code::tests::discriminant_values_match_spec -- --exact
```

Result summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 11.07s
test exit_code::tests::discriminant_values_match_spec ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 171 filtered out; finished in 0.00s
```

## Baseline source finding

File: `crates/velvet_ballastics/src/exit_code.rs`

Current relevant source:

```text
ReplayDivergence = 8,
DomainError = 9,
```

Current relevant tests assert `DomainError as u8 == 9` and include ten variants.

## Classification

- Existing local defect: `CliExitCode::DomainError = 9` violates target public exit code range `0..=8`.
- Failure class for later formal gates if unchanged: `BLOCK_LOCAL` / `REQUIRED_OBLIGATION_FAIL`.
- owner_state for repair: State 10 implementation, after State 5/6 proof artifact adequacy and State 7/8/9 tests approve.
- rerun_from after repair: State 10, then State 11 formal/test execution and downstream reviews.
