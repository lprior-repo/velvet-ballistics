# Per-worktree JJ backing repair

Observed parent evidence-only change6d703ba4 rewritten to70dbe5f4 with8unrelated source changes during parallel operations. The unchanged delivery checkpoint remains856603b8; bad coordination evidence branches are not source acceptance. Main stayed clean.

All active stores were checked against Git/JJ roots.14of15 used commonGit .git as git_target. A fresh Git worktree with `jj git init --git-repo . .` produced correct ../../../.git indirection. Parent backed up each original target under /tmp/vb-em7tf-backing-backups, repointed only store target files, and ran jj --ignore-working-copy status successfully in all15. No source reset/copy requested during repoint. Parent before/after tracked-source hashes unchanged across repoint and subsequent normalstatus.

A nonempty uniquely tracked JJ probe plus newchange changed ONLY its own worktree GitHEAD; all other HEADs, including main, remained unchanged. Future new workspaces must initialize JJ from the Git worktree itself, never common main/.git.

Parent source was subsequently restored by creating a new JJ change from preserved exact good6d703ba4, with bad source diff retained as evidence. Registry and repair evidence restored in that new change. Workers must independently inspect their source diff and roots before publication; no acceptance of previously imported unrelated edits. This repair does not close any feature or v1 gate. Independent review pending.
