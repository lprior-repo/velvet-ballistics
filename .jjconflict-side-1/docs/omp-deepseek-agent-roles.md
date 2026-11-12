# OMP DeepSeek Agent Roles

Three reusable OMP task-agent roles pin the `deepseek/deepseek-v4-pro` model with
`thinkingLevel: high` for the velvet-ballistics Rust delivery flow.

These are **OMP backend agents**, not Codex native-picker models. They are discovered
by OMP's task subsystem from `.omp/agents/*.md` (project) and `~/.omp/agent/agents/*.md`
(user), and dispatched via OMP's `task` tool by `agent` name. They do not register any
model in the Codex native picker or any global model config.

## Roles

| name | purpose | tools | model |
| --- | --- | --- | --- |
| `vb-deepseek-rust` | implementation worker | read, edit, write, bash, grep, glob, find | `deepseek/deepseek-v4-pro` |
| `vb-deepseek-code-review` | independent code review (read-only) | read, grep, glob, find, bash, lsp | `deepseek/deepseek-v4-pro` |
| `vb-deepseek-test-review` | independent test review (read-only) | read, grep, glob, find, bash | `deepseek/deepseek-v4-pro` |

All three set `thinkingLevel: high` and declare `model: [deepseek/deepseek-v4-pro]`
directly (no `@slow`/`@smol`/`@plan` role aliases), so the child session resolves the
DeepSeek v4 Pro model from the agent frontmatter regardless of the parent's active model.

## Where they work

Each agent works only in its assigned isolated Git+JJ workspace under `~/src/isoloated/`.
The main coordination checkout at `/home/lewis/src/velvet-ballistics` is never used for
implementation, review, or broad verification.

## Invocation

A coordinator runs OMP (e.g. `mise exec -- omp`) pinned to DeepSeek v4 Pro and uses the
`task` tool to dispatch a named role. The batch shape is:

```json
{
  "context": "Velvet-ballistics bead vb-<id>; isolated workspace ~/src/isoloated/velvet-ballistics-<bead-id>-<role>.",
  "tasks": [
    {
      "agent": "vb-deepseek-rust",
      "task": "Implement the bead contract end-to-end and smoke-verify the changed path.",
      "solutionSpace": "Implementation plan is given; edge cases remain open."
    }
  ]
}
```

- `agent` selects the role; the role's frontmatter `model` list pins the child to
  `deepseek/deepseek-v4-pro` (model precedence: `task.agentModelOverrides` -> agent
  frontmatter `model` -> parent model).
- Pin the parent smoke/coordination runs to DeepSeek v4 Pro as well
  (`--model deepseek/deepseek-v4-pro --thinking high --smol deepseek/deepseek-v4-pro
  --slow deepseek/deepseek-v4-pro --plan deepseek/deepseek-v4-pro`) to avoid any
  auto Sol/Luna routing.

## Delivery order

Respect the contract order: Rust contract -> implementation (`vb-deepseek-rust`) ->
tests -> independent code review (`vb-deepseek-code-review`) -> independent test review
(`vb-deepseek-test-review`). No author self-approval: the review roles must be run by a
different session/agent than the one that produced the change under review.

## Files

- `.omp/agents/vb-deepseek-rust.md`
- `.omp/agents/vb-deepseek-code-review.md`
- `.omp/agents/vb-deepseek-test-review.md`
- Installed copies: `~/.omp/agent/agents/vb-deepseek-*.md` (byte-identical to the project
  templates above)
