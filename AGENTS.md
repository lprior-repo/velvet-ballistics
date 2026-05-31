# Agent Harness

This is the tiny auto-loaded routing layer. Keep it short; load linked docs only when the task needs them.

## Golden Checkout Rule

- `/home/lewis/src/velvet-ballistics` is the golden GitHub-main mirror. Never edit, commit, rebase, or push from it.
- All implementation and doc work happens in a separate JJ workspace outside the golden checkout, normally `/home/lewis/src/vb-workspaces/<bead-id>`.
- Before editing, verify `jj root` and current path are the intended workspace, not `/home/lewis/src/velvet-ballistics` or the broken parent JJ root `/home/lewis`.
- If a tool starts in the golden checkout, move to a JJ workspace before changing files.

## Authority

- Repo-root `velvet-ballistics-MASTER.md` is the source of truth for architecture, lifecycle, phase status, naming, and acceptance.
- If docs conflict, the master contract wins. Read only the relevant heading instead of loading the full file.
- `AGENTS.md` is the only root agent harness. Do not create secondary root agent instruction files.
- Use `docs/agent-operating-guide.md` for beads, Dolt, verification, shell safety, and closeout details.
- Use `docs/agent-skill-routing.md` for exact skill invocation rules.

## Start Change Tasks

1. For implementation or doc changes, create or enter the bead workspace under `/home/lewis/src/vb-workspaces/`.
2. Check status and preserve unrelated dirty files.
3. Use beads before implementation: `bd ready`, `bd show <id>`, then `bd update <id> --claim` or create a focused bead.
4. Inspect relevant files before changing them.
5. Make the smallest correct change and verify the affected blast radius.

## Karpathy Guardrails

- Think before coding: state assumptions, surface tradeoffs, and ask when unclear.
- Simplicity first: no speculative features, abstractions, configurability, or impossible-case scaffolding; invalid states must be unrepresentable or fail closed with typed errors.
- Surgical changes: touch only what the task requires, match local style, and clean up only your own orphans.
- Goal-driven execution: define verifiable success criteria; for multi-step work, pair each step with its check.

## Skill Routing

- Load only skills that match the task; do not stack broad skills speculatively.
- For beads or issue state, use `beads`; if `bd`/Dolt/server-mode fails, use `dolt`.
- For Rust changes, use `functional-rust`; add domain/verifier skills only when `docs/agent-skill-routing.md` says they apply.
- For Moon task or CI configuration changes, use `moon-v2`.
- For proof-first bead delivery, follow the specialist chain in `docs/agent-skill-routing.md`; no agent reviews its own proof/test/evidence artifact.
- For review/audit, use `black-hat-reviewer` for contract/architecture gates and `truth-serum` for execution-evidence/hallucination audits.
- Do not invoke unrelated platform, desktop, fleet, or deferred-UI skills for this backend repo unless the user explicitly changes scope.

## Non-Negotiables

- Canonical names: product/binary/package/bead rig `velvet-ballistics`; Rust crate/module and bead database `velvet_ballistics`; language `velvet-ballistics/v1`.
- Workspace is virtual: production crates in `crates/`, cross-crate tests and benches in `crates/workspace_tests/`, fuzzing in `fuzz/`, tooling in `xtask/`.
- Never add root production code, root `tests/`, or root `benches/`.
- First-party Rust has no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, ignored `Result`, or unbounded resource growth.
- Runtime core has no YAML interpretation, JSON parsing, HTTP routing, dynamic string lookup, or task-per-step scheduler.
- `moon ci` is canonical for full CI. Rust governance and feature scope live in `docs/rust-governance.md`.
- No performance claim without real baseline/result benchmark evidence.
- Formal artifacts must bind to production behavior: no hardcoded Kani shapes, vacuum Verus proofs, unbounded TLA+ math, proof-contract weakening, or whole-fleet verifier blasts for local changes.

## Route By Task

| Task | Read next |
|------|-----------|
| Beads, Dolt, closeout | `docs/agent-operating-guide.md` |
| Architecture, milestone, acceptance | Relevant heading in repo-root `velvet-ballistics-MASTER.md` |
| Rust rules and nightly features | `docs/rust-governance.md` |
| IR, runtime, scheduler | `docs/compiled-ir.md`, `docs/runtime-architecture.md`, `docs/shard-scheduler.md` |
| Expressions, values, language | `docs/expression-engine.md`, `docs/slot-value-model.md`, `docs/language-spec.md` |
| Storage, journal, IPC | `docs/fjall-storage.md`, `docs/storage-journal.md`, `docs/binary-ipc.md` |
| Performance | `docs/performance-contract.md`, `docs/benchmark-suite.md` |
| Deferred codegen/UI history | `docs/deferred-codegen-maxperf.md`, `docs/deferred-ui.md` |

## Finish

- File follow-up beads for remaining work.
- Run relevant gates and report exact commands plus skipped gates.
- Close or update the active bead, then run `bd dolt push`.
- If landing is requested, use JJ from the workspace: `jj status`, `jj diff`, then `jj git push --remote <remote> -b <bookmark>` or `jj git push --remote <remote> --named <bookmark>=@`. Never `git push` from golden; sync golden from GitHub main only.
