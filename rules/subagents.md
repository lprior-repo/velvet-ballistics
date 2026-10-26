# Subagents

Read this when spawning a subagent or writing a reusable agent definition: which model tier the job takes, how to pin it, and when to escalate.

- Every provider ships roughly three tiers under its own names — light (fast, cheap), standard, frontier. Pick by capability class, never by a model name recalled from memory: names, ids and aliases change per release and none of them transfer between providers.
- Resolve the tier's actual name at use time from the running harness itself — its model list or `--model` help, its settings file, its own docs. No way to resolve it → leave the field unset and report that the tier was not pinned; a guessed id either errors or silently falls back to the expensive default.
- Light tier is the default for work whose hardest step is retrieval or mechanics: locating a file, symbol or caller, grep sweeps and inventories, counting, quoting a named file back, renames and formatting, converting between formats, extracting fields from structured output, running a command and reporting what it printed.
- Standard tier: multi-file implementation against a settled design, tests written to a stated contract, executing a plan someone else decided, a first-pass review that a human still reads.
- Frontier tier is earned, not defaulted to: trade-off decisions, a root cause that survived two fixes, security or privacy judgment, the last review of a change nobody will re-check, a refactor whose boundaries are still open.
- The parent's tier is not a child's default — most harnesses hand it down silently, which is how a repo-wide grep bills frontier tokens. State the tier on every spawn, including when it matches the parent's.
- A reusable agent gets its tier in its definition file (a `model:` field in the agent's frontmatter, where the harness has one), so it is decided once; an ad-hoc spawn passes the tier in the spawn call. Reasoning effort is pinned the same way, beside the tier, where the format has a field for it.
- Reasoning effort is the second cost axis, and on current frontier models often the cheaper one to move: for a job that still needs judgment, lower the effort on a capable tier before dropping a tier. Effort level names do not mean the same amount of reasoning across models or versions, so resolve them at use time like tier names.
- Escalate on a symptom, not on a feeling: the agent missed something you can prove is there, contradicted itself, or refused the scope. Escalation re-runs the same task with the same output contract at the next tier up — never run two tiers on one task in parallel and pick the answer you like.
- A light agent gets the same output contract, scope cap and verification as any other. Tier is a cost decision; loosening the contract to match a cheaper model is how the savings turn into a wrong answer nobody caught.
- Where the result is cheaply checkable, two light runs beat one frontier run: a wrong answer you can verify costs less than a right one you cannot afford to repeat.
- Confirm once that a pinned tier took effect — the harness's run log, usage output or session UI names the model actually used. A field the harness ignores looks identical to one it honors.

- Delegate only when the work cannot fit one context, or when exploration would flood the main context with file dumps better isolated in a subagent.
- A subagent returns conclusions, not raw dumps: give it an explicit output contract (a `path:line` list, a diff receipt) and a scope cap it refuses beyond ("too big: split into N tasks") instead of half-doing the job.
- A subagent that verifies gets the diff and the criteria, never the implementer's reasoning, because the justification is what talks a reviewer into a pass.
- Parallel subagents get disjoint files, a timeout, and a cap on how many run at once; work that needs one agreed design is a single agent's job.
- Tell every spawned agent that it shares the workspace, so it leaves alone what another agent or the user has in flight (no reverting, reformatting or staging it), and tell it outright whether it may spawn agents of its own; an unstated recursion limit is how one fleet becomes several.

Not a violation — leave these alone:

- A tier the user named. Their choice stands whatever this ladder says.
- A light agent running many steps: step count is not difficulty, and a long mechanical sweep is exactly what the cheap tier is for.
- An unpinned tier where the harness exposes no model selection — say so once and move on.
