# Independent cancellation suite review

STATUS: REJECTED pending narrow corrections.

Exactcandidate1e12ef61fa0ba50e7d4cd223f8f2149181810cec, authorlocal3090, reviewerroot in unique isolated Git/JJ workspace.

1. Blocker: author summary claims all six stale ticket tests compare full journal snapshots, but HP3 and several peers still compare journal_after.len() to journal_before.len() (e.g. testfile350). Such an assertion misses replacement/corruption at unchanged length. Only the main stale_action_after_cancel test has full equality. Require full snapshot equality for all six claimed no-mutation cases, preserving exact typed errors and genuine ticket capture.
2. Blocker: exactfile rustfmt --check --edition2024 --config skip_children=true exits1. Source diff whitespace otherwise exits0. Format the exact testfile and rerun; no broadfmt necessary.

Independent focused test command passes18, zero failures/ignored/filtered, actual subprocessreturncode0 and full logs retained. No new explicitpanic/unwrap/expect/ignoredfallible result remains in this exact file. Fake tickethelper removed, genuineissued tickets captured before terminal mutation and missing afterwards; action7 capability retained with unrelatedpaddingempty. Exact RunNotFound/InvalidActionCompletion distinctions are asserted. These are useful improvements, but scope claims must match assertions.

No production defect or fullv1 acceptance asserted. Independent code reviewer local3090 is reviewing this same pinned candidate separately. Source author must fix remaining assertions+format then both reviewers examine exactnewcandidate.
