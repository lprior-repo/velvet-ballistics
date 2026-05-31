# Agent Operating Guide

Read this only after `AGENTS.md` routes you here. The goal is token efficiency: keep universal rules in `AGENTS.md`, stable product law in repo-root `velvet-ballistics-MASTER.md`, and task detail in focused docs.

## Workspace Discipline

1. Treat `/home/lewis/src/velvet-ballistics` as read-only for Git-tracked files and all VCS operations.
2. Create workspaces under `/home/lewis/src/vb-workspaces/<bead-id>` with JJ.
3. Verify location before editing: `jj root` should be the intended workspace, and the current path must not be the golden checkout.
4. If `jj root` resolves to `/home/lewis`, stop; that is the broken parent JJ root, not the repo workspace.
5. Push and land from the JJ workspace. Update golden only by syncing from GitHub main after landing.

## First Five Minutes

1. Enter or create the JJ workspace.
2. Inspect status and preserve unrelated dirty files.
3. Claim or create the bead.
4. Read the smallest relevant source/doc slice.
5. Define the verification blast radius before editing.

## Beads And Dolt

- Use `bd` for live task state; do not create markdown TODO lists.
- Use `bd remember` for durable project knowledge.
- Active Dolt remote: `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`, branch `main`.
- `.beads/dolt/`, `.beads/backup/`, `.beads/embeddeddolt/`, locks, sockets, and runtime DB state are never Git content.
- If `bd` fails inside a JJ workspace because the copied `.beads` server database is missing, use the golden checkout only for `bd` database commands; never edit tracked files, commit, rebase, or push from it.
- If `bd context`, `bd where`, or `bd dolt` reports embedded mode, run `bash scripts/check-beads-server-mode.sh` before more bead work.

## Documentation Routing

| Need | Source |
|------|--------|
| Current milestone and acceptance | Repo-root `velvet-ballistics-MASTER.md` headings 0, 1, 2, plus task-specific section |
| Rust lint and nightly policy | `docs/rust-governance.md` |
| Compiled workflow shape | `docs/compiled-ir.md` |
| Runtime and scheduler behavior | `docs/runtime-architecture.md`, `docs/shard-scheduler.md` |
| Expression semantics | `docs/expression-engine.md` |
| Value and slot model | `docs/slot-value-model.md` |
| Persistence | `docs/fjall-storage.md`, `docs/storage-journal.md` |
| IPC boundary | `docs/binary-ipc.md`, `docs/ipc-memory-boundary.md` |
| Performance evidence | `docs/performance-contract.md`, `docs/benchmark-suite.md` |
| Deferred historical scope | `docs/deferred-codegen-maxperf.md`, `docs/deferred-ui.md` |
| Skill invocation | `docs/agent-skill-routing.md` |

## Skills

Use `docs/agent-skill-routing.md` before invoking a non-obvious skill. The default is no skill unless that guide has a matching trigger.

## Verification Choices

| Change | Minimum useful check |
|--------|----------------------|
| Docs only | Read-through plus link/path sanity |
| Rust formatting | `cargo fmt --all -- --check` |
| Rust source | Focused package tests and clippy, then `moon ci` for full acceptance |
| Nightly features | `moon run :nightly-feature-gate` |
| Verus | `bash scripts/verify-verus.sh` or documented one-off `verus --crate-type=lib <file>` |
| Flux | `bash scripts/flux-check-package.sh <package>` or `cargo flux -p <package> --message-format human` |
| Kani inventory | `bash scripts/kani-list.sh <package> <optional-package-args>`; do not use root `cargo kani list --format json` as proof evidence |
| Performance | Before/after benchmark commands, raw output, workload shape, host CPU, and threshold |

Tool version checks, scaffold builds, and placeholder Moon tasks are not behavior evidence. Reports must state what actually ran, passed, failed, or was skipped.

## Formal Proof Guardrails

- Kani harnesses must use `kani::Arbitrary` or safe exhaustive `kani::any()` generation for core structures.
- Verus proofs must bind mathematical claims to production `exec fn` behavior.
- TLA+ specs must model bounded hardware limits and error states, not unbounded `Nat` shortcuts.
- If a verifier finds an implementation bug, fix the implementation; do not weaken the contract or harness.
- Scope expensive Kani, mutation, Miri, fuzz, or model-checking runs to the bead blast radius.

## Shell Safety

- Prefer tool-native read/search/edit tools over shell text plumbing.
- Use non-interactive file commands when shell file operations are unavoidable: `cp -f`, `mv -f`, `rm -f`, `rm -rf`, `cp -rf`.
- Use `ssh -o BatchMode=yes` and `scp -o BatchMode=yes` for remote commands.
- Do not run destructive VCS commands such as `git reset --hard` or `git checkout --` unless the user explicitly asks.

## Closeout

1. File follow-up beads for remaining known work.
2. Run relevant gates, or state exactly why a gate was skipped.
3. Close completed beads or update in-progress beads with next action and evidence.
4. Run `bd dolt push` from a bead-working location that can access the server database.
5. For landing, inspect `jj status`, `jj diff`, and recent log; describe the JJ change; push with `jj git push --remote <remote> -b <bookmark>` or `jj git push --remote <remote> --named <bookmark>=@`; then sync golden from GitHub main only.

## Doc Hygiene

- Keep `AGENTS.md` under 100 lines unless a new auto-loaded rule is truly universal.
- Prefer route tables over restating entire policies.
- Delete stale current-scope claims when the master contract has moved them to deferred history.
- Do not create secondary root agent instruction files; this repo uses `AGENTS.md` as the single root harness.
- For docs-only edits, do not invent build, test, benchmark, or verifier evidence.
