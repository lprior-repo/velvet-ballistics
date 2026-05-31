# Agent Harness Write-Up

This change turns the repo from a duplicated instruction pile into a small routing system for agents.

## Problem

The old setup made every agent ingest too much policy before doing useful work. `AGENTS.md` repeated beads rules, verifier rules, shell safety, naming, workspace layout, CI rules, and session completion details in one auto-loaded file. That costs tokens every run and makes drift likely because the same rules also live in the master contract, Rust governance docs, beads docs, and tool-specific files.

There was also a bad workspace assumption: agents could start in `/home/lewis/src/velvet-ballistics` and edit there. That path is not a work area. It is the golden checkout that should reflect GitHub main. Real work needs to happen in JJ workspaces outside the golden checkout.

Finally, the repo had a second root agent instruction surface. It duplicated beads boilerplate, included placeholders, and invited future divergence from `AGENTS.md`.

## Target Setup

The new setup has one auto-loaded harness and one optional runbook.

| Layer | Purpose |
|------|---------|
| `AGENTS.md` | Tiny universal routing harness loaded by agents on every task. |
| `docs/agent-skill-routing.md` | Exact skill triggers, non-triggers, and handoffs for this repo. |
| `docs/agent-operating-guide.md` | On-demand details for beads, Dolt, verification, shell safety, and closeout. |
| Repo-root `velvet-ballistics-MASTER.md` | Authoritative product, architecture, milestone, naming, and acceptance contract. |
| Focused `docs/*.md` | Task-specific context loaded only when relevant. |

The core rule is now first-class: `/home/lewis/src/velvet-ballistics` is the golden GitHub-main mirror and must not be edited directly. Agents create or enter `/home/lewis/src/vb-workspaces/<bead-id>` before changing files.

## What Changed

1. `AGENTS.md` became a compact harness under its 100-line target.
2. The golden checkout rule moved to the top of `AGENTS.md`.
3. The duplicate root agent instruction file was removed so there is one root harness.
4. `docs/agent-operating-guide.md` now holds operational details that do not need to be auto-loaded every time.
5. `README.md` now points humans and agents at the right docs and no longer advertises generated Rust/maxperf as active scope.
6. `docs/deferred-codegen-maxperf.md` captures generated Rust and maxperf as deferred history, matching the master contract.
7. Skill invocation is explicit: `AGENTS.md` gives the compact rule, and `docs/agent-skill-routing.md` gives repo-specific use/do-not-use/handoff rules.

## Why This Is Better

Agents now pay token cost only for universal decisions:

- do not edit golden main,
- use JJ workspaces,
- obey the master contract,
- use beads for task state,
- preserve Rust safety invariants,
- choose the right skill or deliberately skip near-misses,
- route to the right focused doc.

Everything else is progressive disclosure. If the task is about storage, the agent reads storage docs. If it is about Kani, it reads verifier details. If it is a docs-only change, it does not need to load proof runbooks, benchmark policy, or the entire master contract.

This reduces token load, lowers contradiction risk, and makes it harder for agents to do the most damaging thing: mutate the golden checkout directly.

## Document Structure Pattern

The stronger structure is progressive disclosure:

| Layer | Rule |
|------|------|
| Root harness | `AGENTS.md` contains only universal constraints and route pointers. |
| Skill routing | `docs/agent-skill-routing.md` decides which specialist to invoke and which near-misses to skip. |
| Operating guide | `docs/agent-operating-guide.md` explains workflow mechanics after the harness routes there. |
| Master contract | `velvet-ballistics-MASTER.md` owns product law and acceptance. |
| Focused docs | Domain docs own local detail and must not restate the master wholesale. |
| Evidence | Beads and evidence artifacts record work state and command proof, not root docs. |

This gives agents a small first read, a deterministic second read, and a clear stop point before they load large context.

## Expected Agent Flow

1. For change work, start in or create a JJ workspace under `/home/lewis/src/vb-workspaces/<bead-id>`.
2. Verify `jj root` is that workspace, not `/home/lewis` and not `/home/lewis/src/velvet-ballistics`.
3. Claim or create a bead.
4. Read `AGENTS.md`.
5. Follow the route table to load only task-relevant docs.
6. Make the smallest correct change.
7. Run scoped verification.
8. Push from the JJ workspace with `jj git push`, not Git from the golden checkout.
9. Sync golden from GitHub main only after landing.

## Non-Goals

This does not relax any engineering rule. It does not weaken Rust safety, formal proof, benchmark, beads, or CI expectations. It only changes where those rules live and when an agent reads them.

This also does not create compatibility shims for other root instruction filenames. The repo standard is `AGENTS.md`; adding another root agent instruction file recreates the drift problem.

## Success Criteria

- `AGENTS.md` stays under 100 lines.
- Golden checkout remains clean and mirrors GitHub main.
- New work happens in JJ workspaces under `/home/lewis/src/vb-workspaces/`.
- Agents can find the right detail doc from the route table without reading the whole master contract.
- Stale active-scope claims about generated Rust, UI, or maxperf do not reappear in onboarding docs.
