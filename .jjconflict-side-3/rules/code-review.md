# Code review

Read this when reviewing a diff/PR or preparing your own changes for review.

## Requesting review

- Package context precisely: BASE and HEAD commits, what the change claims to do, and the plan/requirements it implements, leaving out the full session history, whose reasoning talks a reviewer into agreeing with it.
- PR description answers: what changed, why, risks, how it was tested, where reviewers should focus; UI changes include screenshots; note the rollout steps this change itself requires.
- Prefer small PRs with one coherent intent — split unrelated changes instead of bundling them.
- The description is written for a reviewer who never saw the conversation that produced the change: lead with what the change does and the resulting behavior, give a trigger and a before/after where one clarifies, and scale the detail to the change — two sentences plus the validation is a complete description of a small PR.
- Open on the change, not the breakage: one or two sentences in the imperative of the diff ("Adds a check for characters no reader can see", "Passes the rewind reason the script requires"), and the problem second. A description that opens on the failure reads as an incident report and buries what is being merged.
- The opening names what changed, never how many, trimming a long list with "and others" instead of replacing it with its length. "correct seven behaviors" and "fixes three modules" send the reader into the diff to learn which seven; naming them costs the same room and answers the question where it was asked, and the shape of the change is what the first sentence exists to give; this binds the title too, which a squash merge turns into the commit subject and the changelog line.
- A contrast frame in front of the opening goes the same way, deleted with nothing put in its place: "Four typecheck configurations run, not one:" and "One thing worth naming rather than hiding:" both stand in front of sentences that already make the point (`rules/code-comments.md`).
- A description persists, so it carries no line numbers, no byte offsets or counts, and no totals the next commit invalidates ("1596 tests, same as before"). Name the file, the symbol and the suite instead: a pin sends the next reader to unrelated code with full confidence, and a test total is stale the day someone adds a test (`rules/markdown.md`).
- Third person, neutral: not "I did not run it" but "the integration suite was not run"; never "you can check it by" or "worth knowing if you touch that line". A description states the change — it is neither advice addressed to the reviewer nor a report of the author's session, and the forensic aside that reconstructs how the defect was found ("the last green run finished 79 minutes before the guard landed") belongs in neither.
- Plain spoken English, the register a developer uses explaining the change at a desk: no literary abstraction and no ceremonial verb where the plain one exists ("The substance here is that the retry reuses the first key" → "The retry reuses the first key"). The vocabulary lists in `rules/code-comments.md` bind a description the way they bind a comment.
- Leave out what the pipeline already does. "Checked with lint, typecheck and the unit suite" restates what every PR runs; state only verification a reviewer could not assume — a probe that had to be constructed, or a lane deliberately skipped and why.
- Leave out branch coordination: merge order, what this is stacked on, which sibling lands first. That is operator traffic for the chat or the merge queue, and it is wrong the moment either branch moves.
- Findings the change does not fix go to the tracker, not to a "still open" section that turns a reviewable diff into a status report.
- Everything about shape below is the default for a repository that states nothing of its own, and it yields to whatever the repository does state. A `PULL_REQUEST_TEMPLATE` is the strongest of those signals: its sections replace these outright and get filled in rather than worked around, and a section it asks for that seems redundant is still answered. `CONTRIBUTING`, the README and any other document in the tree come next, then the shape the repository's own recent merged descriptions already show.
- Read those before writing the first line, the way the commit convention is read before a message rather than after a hook rejects one (`rules/commit-messages.md`).
- Four sections, always the same four, so a reviewer knows where to look before reading: `Summary` for what the change does, `Problem` for what led to it and why nothing caught it earlier, `Verification` for what was run and what it showed, `Confidence` for what remains unproven and what that would cost. Bare one-word names, no articles, no invented synonyms - `Test plan` promises work not yet done, `Background` says nothing about what follows, and a reader who learns the set once should never have to learn it again.
- A change that alters no behavior - a comment pass, a rename of prose, a docs-only edit - answers `Verification` and `Confidence` in a line each: there was nothing to run, and behavior is unchanged. Say what was checked instead ("each rewritten comment against the code it sits on") and stop.
- Reasoning that holds for every such change is padding a reviewer already knows from the diff: that a comment is not executed, that no lint reads a claim, that nothing outside the repository moves.
- `Problem` keeps only what specifically produced this drift - the mechanism that moved and left the prose behind - never the general observation that prose and code drift apart.
- A heading is followed by a sentence or three before any bullet, and that lead earns its place only by saying something the bullets do not: what kind of thing the section holds and why it is shaped that way. "Evidence differs by item, because not all of this can be reached without a signed-in browser" tells a reviewer how to read what follows; "Each part of the template restated something already enforced elsewhere" is the bullet list with the detail removed, and costs a reader the time to discover it was. A lead that summarizes its own bullets is the machine tell this whole section exists to keep out.
- Inside `Problem` and `Verification`, one bullet per item rather than a subheading per item: subheadings turn a reviewer's scan into a table of contents for a few lines apiece. Keep a bullet to a couple of short sentences and split a complex one instead of extending it; the reader is looking for the item that concerns them, and a paragraph hides it.
- `Summary` and `Confidence` stay prose, being a statement and a judgement rather than lists, and a change with a single item keeps all four sections with a sentence in place of each list.
- Machine-written shape to drop: any heading beyond the four above, bold-lead bullets, a table for a handful of rows, a numbered list of the commits the host already renders, em dashes (`rules/markdown.md`).
- One paragraph is one line, however long it runs. The commit convention that wraps a body under 80 columns exists because `git log` reflows nothing; a review host reflows to whatever width the reader has, so a hard wrap there pins the text to a column no reader shares, leaves a ragged edge on every other viewport, and has to be redone by hand after each edit. The same holds for a description written into an issue, a release note or a tracker.
- Generating the body → generate it through a check that refuses to write when it finds `:[0-9]+`, first or second person, a non-ASCII character, a cardinal number in the title or the opening paragraph, or two prose lines in a row (a paragraph that survived as one line cannot produce them). Catching it before upload costs one run; catching it after costs an edit on every open PR.
- Scope drifted during the work → rewrite the title and description around what was actually built, and drop the abandoned approaches. A history of what you tried belongs in the PR only where it explains a tradeoff the reviewer has to judge.
- Review is due before merging any major feature, after complex bug fixes, and before large refactors (to establish a baseline). Never skip it claiming the change is simple.

## Reviewing (two axes)

- Fatal blockers first (doesn't build, doesn't run, broken premise): if one exists, stop scoring details and lead the report with it.
- Spec compliance: does the diff do exactly what was asked — nothing missing, nothing extra?
- Standards: correctness, security, error handling, naming, test quality.
- A diff that adds a trust boundary or data flow (new endpoint, file upload, external integration, queue consumer) or touches auth/payments escalates to security-focused review.
- A diff touching CI workflow files, lockfiles, or build scripts escalates to supply-chain review — those files execute with repository credentials, and they are the least-read code in the repo (`rules/ci-cd-security.md`, `rules/dependencies.md`).
- Tests with more mocking than logic exercise the mock, not the code — flag them. Mock external services only, never your own app.
- A test's existence proves nothing — review what its assertions would catch: an assertion that survives the regression it claims to guard is decoration.
- Architecture findings get names: boundary drift (UI reaching into DB, domain types leaking transport shapes) and one-way doors (schema choices, public API shapes, persisted formats — anything expensive to reverse) are called out explicitly, reversibility stated.
- Diff touches a cache → check the key encodes every variable the value depends on; a cache key missing one input serves user A's data to user B (`rules/caching.md`).
- Review the diff, not the repository: a defect the change introduced is a finding, a pre-existing one is a separate note the author can decline. Neither is a demand for rigor the rest of the codebase doesn't carry — validation and comment density the surrounding files never had is a proposal, not a blocker.
- "This might break something elsewhere" is not a finding until you name the code it breaks. Speculation costs the author the search you skipped.
- Before reporting a finding, try to disprove it against the code and drop it if it folds. What survives goes out with its confidence; the low-confidence tail stays out of the report.
- Leave out what a formatter or linter would catch or fix: style the repository's tooling enforces is not review material. Spend the review on what tooling cannot reach.
- Deterministic checks run first: tests, typecheck, lint and schema checks give a verdict, and review (human, subagent or model judge) adds to them, never stands in for one that could run. A check that could not run is reported as not run, never covered by reading the code.
- Fix causes, not symptoms. A "simplification" that requires changing tests is a behavior change in disguise — flag it.
- A comment in the diff is a claim: check it against the code it sits on. One that no longer describes that code is a defect, not a nit, and the machine-written tells (mirrored "X, not Y", figures from one measurement) are findings of their own (`rules/code-comments.md`).
- History is review context: `git log -L <start>,<end>:<file>` on the lines being changed says what the replaced code was for. A line introduced by a commit naming a bug, CVE, or incident is a guard — its removal is a regression until the author says why the cause is gone; a file that keeps appearing in fix commits is a hotspot worth reading whole.
- A confirmed defect is a class, not an instance: before closing the finding, search for the same shape elsewhere — the same sink with another caller, the same missing check on sibling routes, the same pattern copy-pasted into a second module — and report every location as one finding. Fixing the one place the review named leaves the siblings live (core Coding rule on grepping every caller).
- A change to one side of a paired operation is checked against its counterpart: create and update, serialize and deserialize, encode and decode, acquire and release, setup and teardown, migrate up and down. A validation added to create and missing from update, or a field written and never read back, passes every test that exercises one side.
- An agent-authored diff earns a plausibility pass a human's would not: confirm every cited file, symbol, flag, and API actually exists, that a claimed verification run really happened and its output says what the summary claims, and that no unrequested file was touched. Fluent prose is not evidence (core Verification gate).
- Label numbers as measured / estimated / unknown; never present an estimate as measured — unknown stays "unknown", not N/A.

## Severity triage

- CRITICAL (blocks merge): data loss, security, broken build — fix immediately.
- HIGH: resolve before proceeding.
- MEDIUM: note for this cycle.
- LOW: optional polish.
- Approve only with zero CRITICAL/HIGH findings.
- Disputed feedback: evidence-based pushback only — re-check against the code first, then agree with evidence or object with reasoning.

## Review comment format

- A review skill or the repository's own template that sets a finding format wins; otherwise one line per finding: `file:line: <severity>: problem. fix.` — location, problem, concrete fix. No throat-clearing ("I noticed that...", "You might want to consider...").
- Concrete fix over "consider refactoring"; exact symbols in backticks; add the why only when the fix isn't obvious from the problem.
- Unsure whether it's a real problem → ask it as a question, never hedge with "perhaps"/"maybe"/"I think".
- No praise for what was already right: the review is its findings, and praise beside them reads as padding and dilutes them.
- Full-prose exceptions: security findings (complete explanation plus reference) and architectural disagreements (rationale, not a one-liner). Resume one-liners after.

## Receiving review

- Read all feedback without reacting; verify each claim against the codebase before implementing — reviewers can lack context, and bots miscount (check a bot's claimed finding count against what you actually fetched).
- Any item unclear → stop and ask before implementing anything. Otherwise implement one item at a time, testing each.
- YAGNI check on suggestions: confirm the suggested thing is actually used or needed before building it.
- Process in severity order; never skip CRITICAL/HIGH without explicit approval. Every comment gets a reply: a fix, evidence-based pushback, or a question.
- Functional fixes commit separately from cosmetic ones, so the functional diff reviews, bisects and reverts on its own.
- Acknowledge factually ("Fixed in <location>"), no thanks or praise; retract your own pushback the same way ("You were right — checked X, it does Y").
