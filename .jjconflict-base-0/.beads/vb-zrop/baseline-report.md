bead_id: vb-zrop
bead_title: quality: fix verify-standard ignored fallible result gate
phase: 1
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

# Baseline Report

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/go-skill-vb-zrop-git
path_guard: PASS
bd_show: .beads/vb-zrop/bd-show.json
workspace_evidence: .beads/vb-zrop/git-worktrees.txt and .beads/vb-zrop/jj-workspaces.txt
baseline_command: moon run :verify-standard
baseline_log: .beads/vb-zrop/baseline-verify-standard.log
baseline_result: FAIL
primary_gate: GATE-IGNORED-FALLIBLE-RESULTS
classification: BLOCK_RELEASE / REQUIRED_OBLIGATION_FAIL
raw_findings: reproduced in baseline log lines 26-50.
