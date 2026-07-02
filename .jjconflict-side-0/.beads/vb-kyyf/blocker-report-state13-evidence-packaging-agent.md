# vb-kyyf blocker report — State 13 evidence-packaging agent unavailable

STATUS: BLOCKED

## Current state

`vb-kyyf` reached State 13 after:

- State 12 attempt 3: `STATUS: APPROVED`.
- State 11 cap-unblock rerun: `STATUS: APPROVED` for `formal-verification-report.md` and `machine-gate-report.md`.
- PO-001..PO-009: `PASS`.
- PO-010: `DEFERRED_GLOBAL` for out-of-scope `vb_cli` exit-code failures and environment disk-quota copying `.tlc-metadir`, after scoped vb-kyyf obligations passed.

## Blocker

State 13 requires `evidence-packaging` and `truth-serum` artifacts:

- `.beads/vb-kyyf/assurance-bundle.md`
- `.beads/vb-kyyf/truth-serum-report.md`
- `.beads/vb-kyyf/final-evidence-decision.md`

The `evidence-packaging` specialist is not currently available as an OpenCode agent:

- glob checks under `/home/lewis/.opencode`, `/home/lewis/.agents`, and `/home/lewis/.claude` found no `evidence-packaging` agent file.
- `/home/lewis/.agents/skills/evidence-packaging/SKILL.md` exists as a skill, but the femdation child-prompt rule forbids child agents from invoking skills.
- Prior `vb-vt2f` State 13 attempt used `opencode run --agent evidence-packaging`; OpenCode reported `agent "evidence-packaging" not found. Falling back to default agent`, and truth-serum rejected the resulting bundle provenance.

## Required owner decision

Choose one:

1. Install/register a real OpenCode `evidence-packaging` agent, then rerun State 13.
2. Authorize an explicit State 13 provenance waiver allowing a named substitute agent for evidence packaging.
3. Authorize controller-written evidence packaging despite go-skill control-plane boundaries.

No State 14 landing is allowed until State 13 has approved `truth-serum-report.md` and `final-evidence-decision.md`.
