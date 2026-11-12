bead_id: vb-ib8i
phase: 3
updated_at: 2026-05-17T22:07:20Z
attempt: 1-of-7

- Layer 1: rustfmt (`moon ci` fmt task).
- Layer 2: compile/lint (`lint-src`, `check`).
- Layer 3: repository proof/test lanes included by canonical `moon ci`: fuzz smoke, miri, nextest, mutants smoke, coverage, bench build, feature powerset, docs, maxperf/hardened builds.
