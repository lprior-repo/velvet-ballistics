# Contributing

## The bar for a new rule

One question decides admission and retention: **would an agent get this wrong without the line?**

- Yes → it belongs.
- No, a competent agent already does it → it does not, however true it is. A rule nobody needed spends attention that the rules below it needed.

The same test prunes. When a model generation stops making a mistake, the line that prevented it becomes a deletion candidate.

Some lines exist only to compensate for a model weakness: push harder to use a tool, keep going instead of stopping early, verify again before claiming done. They cost the most when the weakness is gone, because a newer model over-applies them: an instruction to be thorough turns into over-triggering, and a carried-over verification instruction into over-verification that adds tokens and latency. Re-test these lines against each new model generation the ruleset supports, and cut or narrow the ones the model no longer needs. The ruleset serves several agents at once, so a line stays while any supported model still needs it.

## Where a rule goes

`AGENTS.md` is the always-loaded core and carries two caps, both enforced by `scripts/lint.py`:

- **200 instruction lines** above the module index. Frontier models follow roughly 150–200 instructions reliably ([IFScale](https://arxiv.org/abs/2507.11538)), the agent's own system prompt already spends some of them, and Claude Code's guidance targets the same figure per file.
- **32 KiB for the whole file**, index included. Codex reads at most 32 KiB of project instructions by default and silently truncates the rest ([Codex AGENTS.md docs](https://developers.openai.com/codex/guides/agents-md)), so anything past that byte never reaches it. The gate counts the file with CRLF line endings, the size a Windows checkout reads. Leave headroom: a project's own `AGENTS.md` shares the same budget.

Adding to the core means removing from the core. A long line counts once toward the line cap and in full toward the byte cap, so moving words into fewer lines saves nothing.

Everything conditional goes to `rules/` — a module is read only when the task matches, so it costs nothing until it is needed. A new module needs:

1. The file, `rules/<topic>.md`.
2. One entry in `rules/INDEX.md`, naming the trigger ("Read this when ..."). CI fails if a module is unlisted or a listed module is missing.
3. One entry in `llms.txt`.

One module per bullet in `rules/INDEX.md`, in theme order. The index is a separate file the core points at, so a new module costs a line there and nothing from the always-loaded core.

## Module format

```markdown
# Topic

Read this when <the exact trigger — a task type, not a technology fan club>.

- <one rule per bullet, imperative, specific enough to act on>
```

- The trigger names what the agent can see at the start of the task: the kind of task, the files it touches, the command it is about to run. A module loads only when the agent decides to open it, so a trigger phrased around a concept the agent never names at that moment ("coupling", "a decision record") never fires. When a module gains a rule, re-read the trigger and widen it if the new rule fires on a task the trigger does not name.
- Every bullet states a decision, a threshold, or a boundary. "Be careful with X" is not a rule; "X is atomic — a read, then a decision, then a write loses to parallel requests" is.
- Every bullet carries its reason in a clause: the failure it prevents. A model generalizes from the reason to cases the rule never listed, and a rule without one is followed only where its words match.
- A prohibition names what to do instead, where an alternative exists: "never parse a completion with a regex; validate against a schema" steers, "never parse a completion with a regex" leaves the agent guessing.
- State the outcome and the check that proves it, not the procedure. Numbered steps belong only where the order itself matters; a step list for a task the model can plan narrows its search and reads as mechanical.
- One rule per bullet. A bullet that needs a third sentence to state its rule is two rules.
- Sources go in one line under Sources in `README.md`, never in the module: an agent reads a comment in a rule file like any other text, and pays for it every time the module loads.
- Plain register, no emphasis. Current models follow system and project instructions closely, and CRITICAL, MUST or a line in capitals makes them over-apply that rule everywhere ([Claude prompting best practices](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices)). Emphasis goes on one line at most, and only after the agent was seen skipping that line; emphasis spread over many lines marks none of them.
- No rule contradicts another, in its own module or elsewhere: a model given two conflicting rules follows one of them at random ([Claude Code memory docs](https://code.claude.com/docs/en/memory)), and a model that follows instructions literally is hurt more ([GPT-5 prompting guide](https://developers.openai.com/cookbook/examples/gpt-5/gpt-5_prompting_guide)). Before adding a rule, search `rules/` and the core for rules it could collide with; where two apply to the same case, the narrower rule names the exception or the precedence itself.
- Cross-reference sibling modules inline as `` (`rules/<sibling>.md`) `` instead of repeating their content. Repeat a rule across modules only when its absence at that moment would cause the mistake, and say why it is repeated.
- Distil sources in your own words; do not paste licensed text. A module written from first principles has nothing to cite; never invent a source to fill the Sources line.

Rule shapes that outperform prose — prefer them when the material allows:

- A correct/wrong pair with the failure mode named ("`==` on a token compiles and leaks timing — compare in constant time") beats an abstract warning.
- A numeric threshold beats an adjective: "nesting ≤ 2, function ≤ 50 lines" is enforceable; "keep it small" is not.
- Version-migration knowledge as old → new pairs (`protect --staged` → `git --pre-commit --staged`), not narrative history.
- Where a rule can be checked mechanically, name the command that checks it (a grep, a lint rule, a CI step) — a rule that ships its own enforcement stops being advisory.
- A module whose rules get over-applied ends with a `Not a violation — leave these alone:` block naming what looks like a breach of its own rules and is not. Over-application is a real failure mode with a real cost: an agent that deletes a deliberate duplicate, flattens a chosen register, or optimizes a path nobody waits on has followed the module and damaged the repository. The same line test admits each carve-out — would an agent get this wrong without it? A carve-out that merely restates the rule's scope does not earn a line.

## Stack-agnostic, and what that permits

The ruleset holds for any stack with nothing extra to install (README). It holds line by line: no rule applies to one framework, platform or tool only. Four tests decide it.

- The core `AGENTS.md` names a framework, library, or non-baseline CLI only inside an explicit presence check — "A secret scanner already installed (`gitleaks version` exits 0, or similar) →", "a commitlint config always wins". The assumable baseline is git and the OS shell; the core declares no OS as its default either, it matches whichever one the user is on.
- No module is gated on a framework, platform or tool. A rule that holds for one of them is rewritten as the invariant behind it and placed in the module that owns the concern, or left out.
- The trigger states exactly the scope the content delivers, never wider. A trigger naming three CI platforms over a body covering one, or ending "any framework" over library-specific bullets, is a defect: narrow the trigger or add the missing content. Name the primary technology and where the transfer stops — "The invariants hold for any relational database; the SQL examples use PostgreSQL syntax, and every major engine has the equivalent."
- A module carries no unmarked stack-specific rule. Where general advice needs a concrete API, either give it across ecosystems (`rules/crypto.md`, `rules/dependencies.md`) or state the invariant and leave the API to the reader's stack.

Never assume a path, file, or configuration belonging to another repository or one machine: "`x.txt` at the repo root" is a rule about someone else's project. Make it conditional or drop it.

## Style

Follow the repo's own `rules/markdown.md` — it applies to this repository first. Short, exact, no filler, no marketing adjectives, commands and paths verbatim.

## Before opening a PR

`python3 scripts/lint.py` runs every check in a few seconds — CI runs that same script, so nothing can pass here and fail there. `python3 scripts/install-hooks.py` installs it as a `pre-commit` hook once per clone; a machine without a Python interpreter skips the hook and relies on CI.

- `AGENTS.md` ≤ 200 lines above the module index, and ≤ 32 KiB as a whole file.
- No ellipsis glyph (U+2026) in any tracked Markdown file; three plain dots stand in for it.
- The core names no framework, library, or non-baseline CLI above the module index — a curated blocklist, so a tool name that belongs inside a presence check is added to the exception list in `scripts/lint.py`, never waved through.
- Modules name blocklisted stacks only on a line marked as an example or on a line that names two or more ecosystems side by side — the same curated-blocklist approach, applied to every line of `rules/`, triggers included, and to each module's entry in the core index and in `llms.txt`.
- Every `rules/*.md` appears in `rules/INDEX.md`, the core's On-demand section points at that file, and every module referenced anywhere exists.
- Every module opens with its `Read this when` trigger line and stays under 3,500 words: a task that opens it pays for all of it, so an oversized module is split along a trigger the agent can see.
- `llms.txt` lists every module, carries no stale entry, and every file it links exists.
- Relative links resolve and code fences are balanced.
- The Claude Code, Codex and Gemini CLI manifests agree with each other, and every path a hook runs still exists — a hook naming a file that moved loads nothing and says nothing. No `hooks/hooks.json` exists: Gemini CLI runs that file from an extension's root, and these hooks are written for Claude Code and Codex, so they live in `hooks/plugin-hooks.json`.
- Every hook ships twice, `.sh` for Claude Code and `.ps1` for Codex on Windows, and `hooks/plugin-hooks.json` wires each event to both (`command` and `commandWindows`). The checks below run every twin in each shell the machine has (Git Bash, `pwsh`, Windows PowerShell), from a plugin copy whose path contains spaces, and a missing shell fails the gate.
- The plugin guard (`hooks/guard.*`) is a rule table in the shape of Anthropic's `bash_command_validator` example: it blocks every command on its block rules and lets the neighbouring commands through, since a guard that stops ordinary work gets disabled, and one that lets a case slip reports a protection nobody has. Its two ask rules hand a push and a commit naming an AI agent as co-author to the user (a human co-author passes), and so a payload that names a command it cannot read.
- The plugin `SubagentStart` hook (`hooks/subagent.*`) gives a subagent the core's Boundaries, Security and Verification sections exactly as the core states them, under the inline limit.
- The plugin's `SessionStart` parts (`hooks/load-core.*`) each stay under Claude Code's inline limit for hook output, rebuild `AGENTS.md` in order, print the same parts in every shell, and each has a hook: a part over the limit reaches the session as a two-kilobyte preview.

CI runs the script on Ubuntu and Windows, runs `evals/benchmark/run.py --selftest` (every benchmark scorer against scripted good and bad outcomes, no agent), and installs the checkout as a plugin into empty Claude Code and Codex homes and as a Gemini CLI extension, failing unless each lists it enabled. A manifest can pass every schema check and still fail to install.

## Changing the core or a hook

The benchmark against the other plugins is the acceptance test for anything that changes what an agent does: a core rule, a hook, the way the plugin delivers the core. Its tasks are split. The train split is what a change is developed against: read its failures, change the ruleset, run it again. The test split is held out: it is run once a change looks done, to accept or reject it, and its failures are never read to shape the change.

1. Try the idea cheaply on train: `evals/benchmark/run.py --split train --arms awesome-agents-md --plugin-path awesome-agents-md=<candidate copy> --repeats 2`. One or two attempts only say which way a change points; several changes that looked like wins at two attempts were losses at five.
2. Read a sample of the scored transcripts before believing a verdict (`--review <run dir>`): a scorer that judges the wrong thing is found there, not in the totals.
3. Confirm on both splits at five attempts or more, on the models of the published tables, and compare with the current ruleset: `--compare <before> <after> --split test`. Cost and answer length are compared task by task, trap passes as rates, each with a 95% interval.
4. Accept the change only if the test split does not get worse on any interval that excludes zero. A change that improves train while test stays flat is fitted to train: revert it. A difference whose interval straddles zero is not evidence either way, and a change that trades trap passes for cost, code size or answer length is a decision for the maintainer, stated in the PR with both numbers.

- A rule or hook message names no task, file or phrase from the benchmark. It states the invariant the task tests; wording lifted from a transcript fits that transcript.
- A rule that text alone does not hold on the benchmark's models belongs in a hook: a hard block or a confirmation prompt (`hooks/guard.*`). Rewording a rule again rarely moves the number.
- `--discrimination <run dirs, weakest model first>` flags a task every arm passes on every model, which measures nothing, and a task the baseline passes less often on a stronger model, which is ambiguous or scored wrongly. A flagged task is rewritten or dropped, and a new task goes to the test split unless it replaces a train task.
- Cells the agent process failed (non-zero exit, timeout, no result) are listed under the summary as infrastructure failures; rerun them with `--resume` before reading the totals.

Trigger-versus-content scope is not mechanically checkable — it is the reviewer's job. Read the trigger, then read the bullets, and ask what a reader on a different stack does with each one.

State in the PR which rule the change adds or removes and which mistake it prevents. A PR that adds a module without naming the mistake gets that question back.
