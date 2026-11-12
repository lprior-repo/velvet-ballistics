# Markdown editorial rules

Read this when writing or editing Markdown documents: docs, articles, READMEs.

- Preserve facts, links, citations, code blocks, commands, and YAML frontmatter — changing any of them changes what the document proves or runs, not just how it reads.
- Do not invent sources, numbers, quotes, dates, or personal experience: use only what the source already states, since a fabricated detail reads as verified until checked.
- Documentation you write about a codebase resolves every path, symbol, flag and command it names against the repository (a listing, a grep) and runs every example it shows; one you could not run is marked unverified. A generated document reads as authoritative and is wrong in exactly the names a reader copies.
- Prefer specific, direct prose over generic summaries — a generic summary could describe any document, so it tells the reader nothing about this one.
- Remove chatbot artifacts, placeholders, excessive bold, decorative formatting, and empty conclusions — none of them adds information the reader can use.
- Replace vague headings with concrete headings — a vague heading tells a reader scanning the document nothing about what's under it.
- Delete paragraphs that add no fact, instruction, example, or decision.
- Return concise diff summaries — the user is scanning for what changed, not reading a review. Do not add generic praise: it carries no information about what changed.
- Quoted examples and cited text stay byte-identical, since altering them misrepresents the source — even when they contain patterns banned below.
- Consistency pass: uniform terminology, heading capitalization, and number style across the document — inconsistency reads as an unfinished edit.
- One H1 per document; heading levels descend without skipping (never H2 straight to H4), and heading text is unique within the file so generated anchors don't collide.
- Every code fence declares a language after the opening backticks (`ts`, `bash`, `json`) — highlighting, copy buttons, and downstream tooling all key off it; use `text` when the block has no language.
- Links to files inside the same repository are relative (`rules/testing.md`), not absolute URLs to the hosting provider — a relative link survives forks, mirrors, and an org rename. Exception: files whose format specification requires absolute URLs (`llms.txt`).
- Images and diagrams carry alt text describing what a reader who cannot see them would otherwise miss, never the filename.
- Never hard-wrap prose to a column limit: one paragraph is one line and the renderer wraps it. Reflowing turns a one-word edit into a whole-paragraph diff (core rule against wrapping to satisfy a character count).
- Editing feedback names the location and the concrete fix ("'it' in §2 is ambiguous — name the subject"), never bare adjectives ("unclear").
- At most two em-dashes per paragraph. Mix sentence lengths deliberately; flag sentences over ~30 words.
- Merge bullet lists where 3+ items share the same opening words or rhythm. Compress fillers: "in order to" → "to", "due to the fact that" → "because".
- Not every claim needs a "however" — drop performative balance.
- A heading is followed by content, never by a sentence restating the heading. `## Performance` then `Speed matters.` then the real point: delete the middle line, which is a rhetorical warm-up that reads as padding.
- A document describes what the thing is, not what changed about it. Cut prose narrating its own last revision ("has been updated to", "now uses", "previously") outside the genres that are version-scoped by definition: changelogs, release notes, migration guides.
- Documentation of a system carries the present tense only, and four things get deleted on sight in any document you are already editing. "It used to", "was replaced by", "before the rewrite" — a reader who never saw the old thing gains nothing and a reader who did is being told twice; where the removal carries a lesson (an endpoint that must never come back), state the lesson as why the current shape is the one that holds. Migration and rename recitals — "the six files that replaced the initial schema", a table row naming a file that no longer exists: describe the thing as it is, git holds the rest. Dated drill and incident records inside an architecture or design document — they belong to the investigation that ran them. Names of code that no longer exists — a constant, module, flag, script or endpoint named in prose is a claim that it is there, and when it is gone the sentence is wrong even if its point still stands. The exceptions are narrow: a postmortem, a drill report, a decision record, anything whose whole subject is a past event, and a release checklist naming what is about to change, because that is its present tense. Test for a sentence: if the code changed tomorrow, would this line have to change too? If yes it is current-state documentation and must be correct; if no, ask what it is doing in a document about the code.
- Never point at a line number. `auth/session.ts:60-75`, "line 205", a table of config rows listed by number — each is wrong after the next edit to a file nobody was thinking about, and the reader who follows it lands on unrelated code believing the document. In an actively developed repository these pins rot within days. Name what survives a reformat instead: the file, and inside it the function, constant, route, binding or heading (`validateSource` in `auth/session.ts`, the cache binding in the deploy config) — a grep for the name finds the place, while a stale number sends the reader elsewhere without telling them. Numbers are allowed only inside quoted output that already carries them (a stack trace, a diff, a log line), where they describe that run and not the current tree. The same holds for commit bodies and code comments; a chat reply is the exception, since the editor makes it clickable and nobody reads it a month later.
- No word claiming a choice was intended — "deliberately", "intentionally", "on purpose", "by design" about the document's own subject, and the same four in comment text and commit messages (`rules/code-comments.md`). The reason is what shows the intent: "The marker is deliberate: it proves the import chain works" → "The marker proves the import chain works" (`rules/code-comments.md` carries the same rule for code comments).
- The borrowed-diction entries in `rules/code-comments.md` bind a document's own prose too: the ceremonial verb standing in for the plain one (mint, honored) and the figurative idiom standing in for the fact (the same beat as, deliberate slack until). Name the action, or the margin and its unit. The entries live in that module; this list keeps no second copy.
- No aphorism formulas — "X is the Y of Z", "the language of", "the currency of", "the architecture of", "X becomes a trap". They dress an ordinary claim as a maxim without adding precision. State the claim the formula is gesturing at.
- One short sentence for emphasis is fine; three or more clipped fragments in a row is manufactured drama. Break the run by restoring a full clause, not by shortening the neighbours.
- Hyphenate a compound only where it sits before the noun: `a high-quality report`, `a data-driven decision`, but `the report is high quality`, `the decision is data driven`. Uniform hyphenation in both positions is a machine tell; the attributive-only rule is what people actually write.
- Headings default to sentence case. Title Case On Every Heading is a tell in prose documents; follow the repo's existing convention where it has one, and never switch conventions inside a document.

Avoid these patterns unless the document's house style requires them:

- "It is important to note"
- "In conclusion"
- "Overall"
- "This guide explores"
- "plays a crucial role"
- "seamless", "robust", "cutting-edge", "pivotal"
- "delve", "tapestry", "testament to", "embark on a journey"
- figurative "navigate" / "landscape" / "realm"; "leverage" as a verb
- "holistic", "paramount", "state-of-the-art"; "comprehensive" where "complete" works
- "Generally speaking", "In essence", "At its core", "It's worth mentioning"
- sentence-initial "Furthermore," / "Moreover,"
- forced groups of three
- "not only X but also Y"
- tiny tables that should be prose

Not a violation — leave these alone:

- A quoted source, a cited title, or a fixture that contains a banned phrase. Quoted text is evidence and stays byte-identical; the ban covers what the document says in its own voice.
- Em-dashes and long sentences in prose the repository publishes as writing rather than as reference: an essay, a post, a personal README. The two-per-paragraph bar is for documentation.
- A sentence fragment used deliberately in marketing copy or a pull-quote. Ask the author before normalizing a register they chose.
- A table whose rows genuinely carry parallel fields. "Tiny tables that should be prose" targets a table of one column, not a short table of real data.
- Title Case inside a proper name, a product name, a book or paper title, or a heading the project's existing house style already sets that way.
- "deliberately", "by design" and their siblings describing something the document did not choose: a platform's behavior ("hooks are bypassable by design"), a register another author picked, an adverb of manner in an instruction ("mix sentence lengths deliberately"). The ban covers a document defending its own decisions.
- A repeated word where the alternative is a synonym the reader has to map back. Term consistency outranks variety in reference documents.
