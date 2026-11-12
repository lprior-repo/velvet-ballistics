# Commit message prose

Read this when writing any commit message, PR title or branch name, planning or rebuilding a commit series, or reviewing commit messages. Every rule here is style-agnostic: whether the repo dresses subjects as `type(scope): summary` or as a plain capitalized sentence changes nothing below, and the same body text serves both.

## House style, shape and type

- Every proposed commit names the files it covers, one path per line under the message — a proposal without its file list is unactionable, and a comma-run of a dozen paths is unreadable at the moment the user stages them. The list ends the proposal: whether a hook, a linter or a config validated the message is not part of it. A multi-commit proposal splits along file boundaries: each commit takes whole files; hunks of one file never spread across two commits. The series is ordered by dependency: when one commit's files require a file changed in another (an import, a helper, a schema, a config key), the required commit comes first — every commit leaves a tree where nothing references what has not landed yet and that builds and passes its tests, so the series bisects. A file that also carries changes from outside the task (a parallel session, the user's own edits): either widen that commit's subject to cover the file's whole diff, or keep the task's subject and add one body line naming the extra changes — what changed, never which tool or session changed it.
- The proposal has a fixed shape, so the user can stage it top to bottom: each commit is its subject line, then a blank line and the body when the commit earns one, then a blank line and the file list — one `- path` per line, paths relative to that repository's root. A blank line separates commits. A task that touched more than one repository groups the commits under a `[repository]` header line per repository, repositories ordered by dependency the way commits are; a single-repository task carries no header. A run of files that would repeat the same path shape a dozen times collapses to one line naming the pattern and the count — `public/_locales/*/messages.json (all 26)`. Worked example: The shape of a proposal, below.
- House style first, in this order: a config or hook that enforces a format (commitlint, a `commit-msg` hook, a `.gitmessage` template, an explicit CONTRIBUTING rule) — a commitlint config always wins; then the convention the repo's own `git log` already shows (e.g. Conventional Commits `type(scope): summary`, React-style `[Area] Fix ...`, kernel-style `subsystem: ...`); then the plain default below.
- Read that spec before writing the message, not after a hook rejects it — and read the one belonging to the repository these files live in, which in a multi-repository checkout is not the one you read an hour ago. It governs type and scope vocabulary, which types may not carry a user-visible change, required footers, subject length and branch naming, it covers PR titles and branch names too, and it outranks every general convention including this file. Where the repo ships the machine-readable half, check the composed message against it rather than guessing: write the message to a file and run the repo's own validator (the `commit-msg` hook's command, or the linter's CLI with `--edit <file>`) — one run settles scope-enum questions that reading the config does not.
- Plain default, for a repo that configures nothing and has no history to copy: one capitalized imperative sentence, no type prefix, no scope, no trailing period — `Add session refresh on 401`, which the same change under a commitlint config would instead write as `feat(auth): add session refresh on 401`. Conventional Commits is one house style among several, never the fallback: proposing `feat(auth):` in a repo whose log is plain sentences is the same error as ignoring a commitlint config.
- Subject: aim ≤50 chars, hard cap 72. Body (wrap under 80, at most four paragraphs ordered problem, mechanism, decision) answers why, only when the subject can't; footer carries `Closes #N`. Where the repo uses Conventional Commits, the summary after `type(scope):` is lowercase, breaking changes take `BREAKING CHANGE:` (or `!` after type/scope), and `revert:` repeats the reverted subject.
- Scope, in a repo whose style has one: kebab-case; reuse scopes already in `git log`, never rename an established one (`auth`, not `authentication`) — a renamed scope splits one area's history across two names in `git log` and any filter built on it.
- Breaking changes, security fixes, data migrations, and reverts always get a body — future debuggers need the context; never subject-only.
- A security-relevant message says what the code now does, never what was wrong with it. Subjects, bodies, branch names, PR titles and generated changelog lines are permanent and public; a message that names the weakness (`Stop leaking the test account`, `Remove the auth bypass`, `Fix IDOR on /orders`, `Patch XSS in the comment renderer`) hands a reader of `git log` the exact commit range to attack — and every user still on the previous release is inside it. Write the area and the action neutrally (`Trim the e2e harness docs`, `Tighten session handling`, `Scope order lookups to the owner`), keep the vulnerability, its impact and its reproduction in the private tracker or the security advisory, and publish that detail only once the fix has shipped. The body still explains the change for a future debugger — it just describes the new behavior, not the hole. This applies to the removal of a disclosure too: a commit that scrubs a leak must not name what it scrubbed.
- Never in the message: "This commit does X", "I"/"we", "now"/"currently", the borrowed diction of the comment rule — the diff already says what.
- Where the repo's style carries type prefixes, the type follows what the diff changes for someone running the product, not how the work felt: `chore` is user-invisible housekeeping only (deps, configs, release bumps), a behavior-preserving rewrite is `refactor`, a speedup `perf`, formatting `style`, and a repair to a test, a harness helper, a gate script or CI wiring is never `fix` however genuinely broken it was — where the changelog is generated from types, that `fix` puts a "Bug Fixes" line in front of users about code they never run. Decide in that order: does anyone running the product observe this change (no → a type the changelog hides), then which hidden type the same paths already carry (`git log --pretty=%s -- <path>`), then write the message. Which types are published and which are hidden is the repo's own setting — read its changelog config the way you read its message config.
- Commit-message skills or plugin styles never override this module: the message follows the repo convention above regardless of the active mode, since a plugin's default carries no knowledge of this repo's own house style.

## When a body is written at all

- Default: none. In repos at the scale of vue, vite, and nest, roughly seven commits in ten carry no body — the subject and the diff are the whole message.
- A body earns its place only by carrying what a competent reader cannot derive from the diff: an outside constraint (a platform behaves badly, an API answers ambiguously), a decision that looks wrong without the explanation, an incident the change prevents from happening again, or a policy the code cannot state (what is deliberately left alone, what is never undone).
- Scaffolding, configuration, icons, translations, and documents get no body, and neither does anything whose subject already says everything.
- One case earns a body regardless of the default: a staged file carrying changes beyond the task's own (a parallel session, the user's edits) whose subject stays task-scoped gets one line naming the extra changes — what changed, never which tool or session changed it. Widening the subject to cover the file's whole diff removes the need.
- Shape: at most four paragraphs, wrapped under 80 columns, ordered problem, mechanism, decision. The bodies a breaking change, security fix, data migration or revert always gets follow the same shape.

## What never stands in a body

- Enumeration of members: a sentence that names a category stops there — the diff carries which three rules and which six checks ("Three rules are stricter than the preset: no explicit any, hooks at top level, console limited to error" → "A few rules are stricter than the preset"). This holds even when the members are non-obvious edge cases; if a test pins them, the test is where they belong. Delete the paragraph outright when the subject already said it.
- Explaining a competent reader's vocabulary: a mechanism whose name carries its purpose is named, never explained — pinning LF, a lockfile beside its manifest, an atomic temp-file rename, a debounce. Same for a guarantee of the language or type system (an exhaustive match, a union a renderer cannot forget).
- The consequence half: a clause restating what the first half already means is one thing said twice ("Every colour the UI uses has a name in the theme mapping, and no component writes a hex value" → stop after "mapping").
- An explanatory colon: a colon introduces a command block or a real list, never joins a claim to its explanation — that is two sentences.
- A second contrast frame: state what the code does, and contrast with what it does not do ("instead of", "not X but Y") at most once per message — 47 uses across 66 commits is a fingerprint, not a style. "rather than" never appears: the Vocabulary of `rules/code-comments.md` bans it on every surface it binds, and commit prose is one of them. Where the rejected alternative carries the reason, give it its own sentence.
- An invented past: in a commit that introduces its files, "used to", "previously", and "no longer" describe history the repository does not have — a false statement, not a style problem. Rewrite as the hypothetical it is ("Pushing a few hundred commits would otherwise spawn a few hundred processes"). In a commit changing existing code, past tense is correct and stays.
- Stacked reasons: one reason per sentence — a chain of so, which, and because is two sentences, and three reasons behind one decision is one reason plus noise; keep the one that decided it.
- A count that can be recounted: a number the code enforces stays (a depth cap, a history window, a concurrency limit, a timeout, an HTTP status, a version floor) — it cannot drift without the code drifting with it; a number describing the current shape of the code rots on the next edit ("bumps it in all four version files" → "bumps every version file"). A surviving number is written as digits, including 0–9 and at the start of a sentence (200 commits, 8 at a time), except where it is not data ("one place every git client agrees on"). A count in a subject is fine when the commit fixes it forever ("translate the interface into ten more locales").
- A reference outward: a body stands alone — no reference to another commit in a plan, no "as above", no "the reason given at the top"; the reader has `git log` and the diff, and a planning document is not in the repository. A sha or an issue number is different and belongs in a footer.
- Borrowed diction, padding, hedging prefixes and claimed intent: the ceremonial verb where the plain one exists, and the filler around it. The lists are the Vocabulary section of `rules/code-comments.md`, which governs message prose as well, its ban on "rather than" included.
- Flourish and signposting: delete a sentence whose only job is to say the preceding fact matters ("The evidence ladder is the whole point"), an opener that announces what is about to be said, and the cleft that inflates a plain statement ("The noreply address is what lets a profile commit privately" → "The noreply address lets a profile commit privately"). The phrase blocklist of `rules/markdown.md` applies in full — inflation, summary stamps, the forced group of three — and the word lists come from the Vocabulary section named above.

## Characters

- Plain ASCII throughout: no em dash, no arrow glyph, no curly quote, no ellipsis character, no emoji, no non-breaking space — they break `git log`, changelog parsers, and terminals, and they read as machine output. The one exception is the house style itself: a log whose subjects open with an emoji (gitmoji) keeps that leading emoji.
- No backticks anywhere, since `git log` renders no markdown and a backtick shows up as a stray character; identifiers are written bare (profiles.json, ssh-keygen, user.useConfigOnly). Quoting is for the rare string that has to be marked off, and then single quotes.
- No implementation trivia in prose, since a literal copied from the diff goes stale the moment that code changes: not an address template, not an include condition, not a full command line, not a call with its argument object — name the thing in words.

## What must survive the cut

Every rule above removes text; this section wins on contact. A body anchored to something that actually happened is the opposite of machine writing — cut a sentence because a reader could have derived it, never because it is long or because a rule above matches its shape.

- Incident provenance: "a standing timer once made this NaN" — past tense here is not invented history; it justifies a present decision.
- Operational knowledge of a live third party that cost real time to learn: which status a host answers to a duplicate key, which greeting closes with a non-zero exit, which browser tries IPv6 first.
- A deliberate refusal: what the change will not do, and why — an option left alone, an identity not restored because it cannot be known.
- A constant with its reason: "200 commits, far enough back to catch a long-wrong identity and short enough to stay instant" — the code enforces the number and the clause says what it buys.
- A wire-format note: field names or spellings shared with data already on disk — nothing in the diff says they cannot be renamed.
- A security boundary: why a check runs before the response is written, why a listener binds both address families.
- The bundled-change line: one sentence naming edits a staged file carries beyond the task's own — derivable from the diff, kept anyway, because it marks what the subject deliberately does not claim.
- Never write a claim into a message to satisfy a rule here: everything stated is confirmed against the code first (core Verification gate). A rewritten sentence that no longer matches the code is a worse defect than the wordiness it replaced.

## Rhythm

- Vary the paragraph opening: half the paragraphs starting on "The" is a rhythm a reader feels before naming it.
- Vary sentence length: a body of uniform 20-word compounds with a comma before "so" reads as generated even when every fact in it is true.

## The shape of a proposal

The shape rules above, written out.

```text
Add session refresh on 401
Handle ISO dates without timezone
Document per-agent install steps
```

A two-repository proposal, in a repo whose house style is Conventional Commits:

```text
[acme-api]

feat(api): answer an unsupported build with its own error string

A build under the supported floor gets 426 upgrade_required instead of
the generic refusal, so the client can tell the difference between an
update it must ship and a request it must retry.

- src/middleware/client-version.ts
- src/middleware/client-version.test.ts
- src/routes/session.ts
- src/config.ts
- openapi.json
- docs/api/versioning.md

ci(ops): publish the production image from a release tag

- .github/workflows/release.yml

[acme-extension]

feat(session): ask for an update when the API refuses this build

- src/popup/session.tsx
- src/popup/session.test.tsx
- public/_locales/*/messages.json (all 26)
```

## Checklist

Run over a drafted message before proposing it, then ask the three questions.

- No backtick anywhere, and no character outside plain ASCII.
- No "rather than" anywhere; the contrast frame ("instead of", "not X but Y") appears at most once in the whole message.
- "used to", "previously" and "no longer" appear only in a commit that changes existing code.
- Every spelled-out count is a number the code enforces; one that merely describes today's shape comes out.
- No word from the vocabulary in `rules/code-comments.md` and no phrase from the blocklist in `rules/markdown.md`.

- Could a competent developer have derived this sentence from the diff? Yes → delete it.
- Could this paragraph sit unchanged in a hundred other commits in a hundred other repositories? Yes → it says nothing about this change; delete it.
- Is every remaining claim confirmed against the code, or assumed while rewriting? The survival section outranks every deletion rule above it.
