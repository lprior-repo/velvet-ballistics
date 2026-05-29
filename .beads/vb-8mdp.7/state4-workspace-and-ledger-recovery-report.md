# State 4 Workspace and Ledger Recovery Report — vb-8mdp.7

STATUS: BLOCKED

## Scope

- bead_id: `vb-8mdp.7`
- state: `4`
- sublane: `workspace-verification-and-ledger-provenance-recovery`
- delegate: `proof-plan-reviewer`
- attempt: `7`
- isolated_workdir: `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.7`

## Workspace identity evidence

Command:

```text
pwd -P && rtk git branch --show-current && rtk git status --short --branch && git rev-parse --show-toplevel && git rev-parse --short main && if command -v jj >/dev/null 2>&1; then jj root; jj workspace list; jj log -r @ --no-graph --limit 1; else true; fi
```

Output:

```text
/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.7
review/vb-8mdp.7
## review/vb-8mdp.7
 M crates/vb_core/Cargo.toml
 M crates/vb_core/src/budget.rs
 M crates/vb_core/src/diagnostic.rs
 M crates/vb_core/src/frame.rs
 M crates/vb_core/src/lib.rs
 M crates/vb_core/tests/proptest_supported_codes.rs
 M crates/vb_runtime/Cargo.toml
 M crates/vb_runtime/src/admission.rs
 M crates/vb_runtime/src/error/diagnostics.rs
 M crates/vb_runtime/src/error/display.rs
 M crates/vb_runtime/src/error/equality.rs
 M crates/vb_runtime/src/error/mod.rs
 M crates/vb_runtime/src/lib.rs
 M crates/vb_runtime/src/primitives/mod.rs
 M crates/vb_runtime/src/runtime.rs
 M crates/vb_runtime/src/shard/lifecycle/chunk_001.rs
 M crates/vb_runtime/src/shard/mod.rs
 M crates/vb_storage/Cargo.toml
 M crates/vb_storage/src/lib.rs
?? .beads/vb-8mdp.7/
?? crates/vb_runtime/src/admission_transaction.rs
?? crates/vb_runtime/src/kani_vb_8mdp_7_public_rejection.rs
?? crates/vb_runtime/src/kani_vb_8mdp_7_staged_rollback.rs
?? crates/vb_runtime/src/kani_vb_8mdp_7_typed_rejection.rs/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.7
6a431705c
/home/lewis
Internal error: The repository appears broken or inaccessible
Caused by:
1: Failed to read commit backend type
2: Cannot access /home/lewis/.jj/repo/store/type
3: No such file or directory (os error 2)
Internal error: The repository appears broken or inaccessible
Caused by:
1: Failed to read commit backend type
2: Cannot access /home/lewis/.jj/repo/store/type
3: No such file or directory (os error 2)
```

Conclusion: the shell was in the required isolated path, not `/home/lewis/src/velvet-ballistics`; branch is `review/vb-8mdp.7`. Git top-level matches the isolated workdir. JJ is installed but unavailable/broken in this checkout, so no JJ provenance recovery is available.

Note: the repository-root `STATE.md` is stale/wrong-bead (`vb-rpch`). The bead-scoped required inputs exist under `.beads/vb-8mdp.7/`, and this report only writes under that bead directory.

## Recovery search evidence

Command:

```text
python3 - <<'PY'
...scan agent-invocation-ledger.jsonl hashes across .beads/vb-8mdp.7 including archive/, then git blobs from the isolated worktree and source checkout...
PY
```

Result summary:

```text
ledger_rows 6 unique_hashes 42
found_in_worktree_or_archive 27
missing_after_archive_scan 15
repo /home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.7 objects 98056 candidates 372
repo_hist_found 0
repo /home/lewis/src/velvet-ballistics objects 98056 candidates 372
repo_hist_found 0
missing_after_git 15
```

Unrecovered historical bytes:

- `f94f2bbe84310e6c4700a050bcf8348fd704b28bde68b838f0256e3e874d7fde` — lines 1, 2, 3 `proof-obligations.planned.jsonl`
- `19ba144e6c7a1e17067ec89b440d101360b24424f90378ba1878363e79a9afda` — lines 1, 3 `proof-plan-review.md`
- `c459198aade2697b4fdde284c6a37ef2dce7d98bd0e3f7032e738276fcf9a53d` — line 1 `verifier-lane-review.jsonl`
- `0052d6f594d79b03768e91f056072faf60f3d12ed993a08f332f2ff87aae9262` — lines 1, 3 `proof-plan-findings.jsonl`
- `86d444e01c9a3f6b8eea3df10fff97410ed53f1195057f3bbca6b5e878f58e1d` — lines 1, 3 `proof-plan-repair-guide.md`
- `25535832a33704bc403d0392a1033433c3f27bdeb3bffcb30b0d67c3e6a3fa9e` — line 1 transcript
- `4a83e20814ba9753777a49456e97de8270cd5d166f421e5d89851fa51f39afa2` — line 3 transcript
- `9269e83d7c86a81690b9c43476db6b96b82cb33dd8df6315e38a776dc6862385` — line 4 `proof-plan-review.md`
- `f5391eb6cc684b7d30ad5b694bda53084ca1c5b0e9d094d040367ec20b756eec` — line 4 `proof-plan-findings.jsonl`
- `b7abc8da69e894c43dcdef3b1a3045bf110faf259a3d5a989bb35ed3c9a36251` — line 4 `proof-plan-repair-guide.md`
- `630fccb6ba8ab2e9d21d01d0572acf198bf1191e11bdff30cd6a0484c0e52e93` — line 5 `proof-plan-review.md`
- `8f4320aed1ad4b8fc83a8c85d6c5d0302113f7b6c3d8b5ddfc284d715fb6ea00` — line 5 `verifier-lane-review.jsonl`
- `236dd540ee24c4ac60d672e6dfb810271c08eb974504213605ef950176bfcd95` — line 5 `proof-plan-findings.jsonl`
- `3aac80d0b7309530b262c171351ce15d6df97083b7e8d018dd1506c72b63afde` — line 5 `proof-plan-repair-guide.md`
- `2447499c4bbb017346f0e35c11ceb4e96ed39451e5163df31181883da03732f8` — line 5 transcript

## Safe repair decision

No ledger update was made. Exact historical bytes required to validate ledger rows 1 through 5 could not be recovered from `.beads/vb-8mdp.7/archive/`, the current bead directory, the isolated Git object database, or the source checkout Git object database. Re-hashing historical rows to current mutable top-level artifacts would fabricate provenance, so it is refused.

The active approved plan content (`proof-plan-review.md`, `verifier-lane-review.jsonl`, `proof-plan-findings.jsonl`, `proof-plan-repair-guide.md`, `proof-obligations.planned.jsonl`) was not modified.

## Canonical State 4 validator

Command:

```text
/home/lewis/.opencode/skill/go-skill/tools/go-skill-v9-validate --workspace /home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.7 --bead vb-8mdp.7 --state 4 --source-checkout /home/lewis/src/velvet-ballistics --skill-root /home/lewis/.agents/skills/go-skill --mirror-root /home/lewis/.opencode/skill/go-skill --format json
```

Result: `FAIL` with 24 `E_INVOCATION_LEDGER_FORGED` blockers on historical ledger lines 1 through 5. Raw validator evidence is recorded in `state4-validator-evidence-workspace-ledger-recovery-attempt7.json`.

## Files changed by this sublane

- Added `.beads/vb-8mdp.7/state4-workspace-and-ledger-recovery-report.md`
- Added `.beads/vb-8mdp.7/transcript-state4-proof-plan-reviewer-workspace-ledger-recovery.md`
- Added `.beads/vb-8mdp.7/state4-validator-evidence-workspace-ledger-recovery-attempt7.json`

## Final determination

This is controller-policy blocked. A PASS requires a controller-owned decision to either provide exact historical artifact bytes, authorize non-append-only ledger normalization, or change the validator policy for superseded mutable-path historical rows. A proof-plan-reviewer child cannot truthfully perform those actions.
