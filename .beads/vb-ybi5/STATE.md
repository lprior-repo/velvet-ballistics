bead_id: vb-ybi5
bead_title: quality: fix verify-standard Kani ignored fallible matches
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/isolated/go-skill-vb-ybi5
current_state: 13
attempt: 1-of-7

State evidence:
- Startup doctrine read: /home/lewis/.claude/skills/go-skill/SKILL.md; /home/lewis/.agents/skills/go-skill/SKILL.md; state-machine.md; checklist.md; artifacts.md.
- State 1: `bd show vb-ybi5 --json`; `bd update vb-ybi5 --claim`; `jj workspace add /home/lewis/isolated/go-skill-vb-ybi5 --name go-skill-vb-ybi5`; `pwd -P` proved workspace is outside source checkout.
- State 2-10: scoped repair to `crates/vb_storage/src/kani_recovery_hydrate.rs`; no Red Queen invoked.
- State 11 attempt 1: baseline scanner reproduced DISCARD-004 lines 78/111. Repair replaced ignored `Err(_) => {}` fallible matches with explicit unexpected-error assertions, deterministic mismatching IDs, nonzero digest generation, and compile-correct digest constructors/imports.
- State 11 evidence: `scripts/check-ignored-fallible-results.sh` PASS `NoViolationFound`; `rustfmt --check crates/vb_storage/src/kani_recovery_hydrate.rs` PASS; `moon run :verify-standard` PASS.
- Focused extra Kani attempts: first failed due `/tmp` disk quota; retry with workspace TMPDIR exposed pre-existing harness compile debt and then timed out after compile on direct vb_storage harness. Not required by verify-standard; recorded in regression-diff as non-landing focused-tooling limitation.
- State 12: black-hat local review approved: the scanner violation is removed without an allow/waiver and proof assertions are stronger than before.
- State 13: evidence approved for bead-local acceptance. Moon CI was attempted and failed on unrelated pre-existing global fmt/check debt outside touched file; acceptance gate remains verify-standard per bead criteria.

Next: State 14 landing if user/repo policy accepts DEFERRED_GLOBAL moon-ci debt for unrelated files.
