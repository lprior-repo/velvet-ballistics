bead_id: vb-ib8i
phase: 14
updated_at: 2026-05-17T22:18:10Z
attempt: 1-of-7

# Landing report

Code/evidence commit: `56030e15108e` (`fix: repair canonical CI blockers`).

Remote push evidence:
- Command: `jj bookmark create go-skill-vb-ib8i-sub9 -r @ && jj git push --bookmark go-skill-vb-ib8i-sub9`
- Result: pushed bookmark `go-skill-vb-ib8i-sub9` to `origin`.
- Remote reported PR URL creation hint: `https://github.com/lprior-repo/velvet-ballistics/pull/new/go-skill-vb-ib8i-sub9`.

Bead tracking evidence:
- Command: `bd close vb-ib8i --reason "Completed in pushed branch go-skill-vb-ib8i-sub9; moon ci --force --summary normal PASS (22 actions, 8964 tests)."`
- Result: bead status `closed`.
- Command: `bd dolt push`
- Result: `Push complete.`

Main integration completion:
- Command: `jj new main go-skill-vb-ib8i-sub9 && jj describe -m "merge: land vb-ib8i CI repair"`
- Result: merge commit `d6a05f141f93` created with parents `65c693f9` (`main`) and `f8ba6049` (`go-skill-vb-ib8i-sub9`).
- Command: `moon ci --force --summary normal`
- Result: PASS on merge commit: 22 actions completed; 8968 tests passed, 15 skipped; no failures.
- Command: `jj bookmark set main -r @ && jj git push --bookmark main`
- Result: main bookmark moved forward from `65c693f9224a` to `d6a05f141f93` and pushed to origin.
