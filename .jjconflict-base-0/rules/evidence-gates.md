# Evidence gates

Read this when turning a verification rule into an enforced gate, when writing an instruction that must hold on every run, or when a completion claim has to survive review by someone who was not watching it being made.

- A rule that must hold on every run with zero exceptions belongs in a hook or a CI gate, not another prose line: an instruction is advisory and gets skipped under load, a gate does not. Propose the gate; adding or editing hooks and gate configs yourself stays a core Boundaries ask.
- Evidence is an artifact, not a sentence: the command, its exit code, the revision it ran against, and when it ran. Extends the core Verification gate to what the claim has to leave behind for a reader who arrives later.
- Bind the receipt to the run that produced it (commit sha, run id, attempt) and have the gate compare each field against its own context, rejecting a mismatch. A receipt carried over from an earlier run is the easiest false pass to produce and the hardest to catch in review.
- A placeholder in a required field fails the gate. One that accepts a pending marker "until the real value arrives" never fires, and the artifact ships carrying the hole it was written to document.
- Recompute every derived verdict from the fields underneath it rather than trusting the flag: where an artifact carries both a ready-state boolean and the data that decides readiness, the gate compares the two and rejects disagreement.
- Failure output is machine-parseable — one line naming the file, the reason code, and the field. Prose failures get re-diagnosed from scratch on every rerun.
- A gate that cannot run reports unavailable, never pass. A missing tool, an absent credential, or a skipped environment is a hole in coverage, and folding it into a green result ships an unchecked layer as verified.
- The gate reads the artifact at a path the run controls, not one that can be edited between the run and the check.
- Keep the human report and the machine artifact separate: the report is read once, the artifact is diffed against the next run's.
- A gate earns its place by failing first: run it against a known-bad input and watch it reject before it is allowed to pass anything. A gate green on its first run has proven nothing yet (core Verification rule, same reason as a regression test that never failed).
- Every suppression carries a reason and an expiry. An undated exception is how a real finding gets inherited as already-triaged.
- Where the gate runs in shared automation, scope its token and pin its components (`rules/ci-cd-security.md`); where it validates a schema or contract the code also depends on, generate both from the one source (`rules/api-contracts.md`).
