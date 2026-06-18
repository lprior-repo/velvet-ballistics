bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
updated_at: 2026-06-17T20:00:00.000000+00:00
attempt: 1-of-7

STATUS: BASELINE_CAPTURED
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/femdation-tier-a-0-002

Path isolation:
- pwd -P: /home/lewis/src/femdation-tier-a-0-002
- equals source: False
- nested under source: False

bd show exit: 0
`bd show tier-a-0-002 --json` produced a valid JSON document. Bead is open, P0 priority, owner=lewis, no blockers on tier-a-0 wave-0.

jj status exit: 0
`jj status` on the isolated workspace reports a clean working copy at the same parent commit (a413ab69 main) as the source checkout. No uncommitted changes.

Baseline note: full moon ci intentionally not run in State 1 to avoid pre-edit fleet cost; State 11 must run scoped bead gates and compare against this clean isolated jj baseline. Bead scope is CI gate installation (shell scripts + moon tasks + tests + documentation), so production code lines and Verus/Kani artifacts are not in scope for tier-a-0 wave.
