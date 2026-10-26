# Debug escalation ladder

Read this when a fix has failed twice, the same error keeps returning, a debugging session is going in circles, or a regression needs locating. `rules/workflow.md` carries the short form; this is the full ladder.

- Read the exact error message and the logs before guessing — the answer is usually printed. Quote the decisive line, not the whole dump.
- Form 3 different hypotheses before testing any. Test the most likely first — and design the check that would disprove it, not confirm it. Reverse the assumption too: "problem is in A" → test "problem is NOT in A".
- Trace backward to where the bad value originates, not where it surfaces; diff against the nearest working case (last green commit, the sibling endpoint that works, the passing test).
- Regression with a known-good revision → `git bisect run <check>` before reading code: log2(N) mechanical runs name the commit that hypotheses would guess at.
- Before the fix, search history for the symptom (`git log -S<symbol>`, `git log --grep=<error>`): an earlier fix of the same symptom, a reverted one above all, says what was already tried and why it failed.
- Escalation ladder: 2nd failure → change methodology, not parameters; 3+ failed fixes → question the architecture, not the code.
- "Tried everything" requires listing the attempts; fewer than 3 distinct approaches = not exhausted.
- Use available tools instead of asking the user to debug manually; no "environment issue" claims without evidence.
- A fix that can't explain the original symptom is a coincidence, not a fix: state the mechanism ("X returned null because Y") before claiming resolution.
- A failing test is a report about the test as much as about the code: before editing the implementation, check the fixture, the environment it needs and the assumption the assertion encodes (`rules/testing.md`). The probe that settles it prints the real state — the final URL, the computed value, the response headers — rather than asserting what you expect to see.
- An edit to a config file, environment variable, secret or certificate is not yet the value the running process uses: most runtimes read these at startup or on an explicit reload. Before trusting a re-test after such an edit, confirm the process picked it up with a restart, a reload log line, or a probe of the live value.
- Close the loop: re-run the original failing scenario and show it passing (core Verification gate).
