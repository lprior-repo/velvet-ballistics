# Refactoring and cleanup tasks

Read this when the task is a dedicated refactor, cleanup, rewrite or migration, or an "improve this code" pass.

- Refactoring is behavior-preserving. Do not change public behavior, public APIs, persisted formats, routes, event names, config keys, or database schemas unless explicitly requested.
- Match the existing style of the repository before applying generic best practices: a codebase written in two styles costs every later reader.
- Small, reviewable passes: rename, remove dead code, simplify control flow (nested conditionals become named predicate functions), extract helper, deduplicate — then verify with the narrowest relevant test, typecheck, lint, or build after each pass.
- Before editing, inspect nearby code and tests to infer naming, error handling, logging, and abstraction patterns.
- No broad rewrites, framework swaps, dependency upgrades, or architectural migrations as part of a cleanup pass.
- Rule of 500: a refactor touching more than ~500 lines, or the same mechanical edit in 5+ files, is automation work — codemods, AST transforms, scripted rewrites — not hand-editing, whose copies drift apart. Run the rewrite as a dry run first and read its diff before writing: an unread scripted rewrite changes the cases nobody looked at.
- Mechanical rewrites use a structural tool, never regex: an AST rewriter (`ast-grep`, `comby`), a codemod, or the language's own refactoring API. A regex edits strings, not syntax, and will eventually rewrite a comment, a string literal, or half an identifier.
- No test coverage on the code being refactored → write the characterization test first: capture current behavior exactly as it is, including the parts that look wrong, then refactor against it. Refactoring untested code is editing in the dark (`rules/testing.md`).
- Chesterton's Fence — before simplifying or deleting, answer: what is this code's responsibility? who calls it and what does it call? which edge cases does it handle? which tests define its behavior? why was it written this way (`git blame`/`git log`)? Can't answer most → not ready to change it.
- "Unused" is shown, never grepped: a symbol with no static reference may still be reached by name (reflection, dependency injection or routing by string, templates, config, serialization) or by a published package's consumers. Search the name as a string and in non-code files before calling it dead; an export of a published package is never provably dead from inside the repository.
- Before a rewrite or migration, inventory the behavior the repository does not hold: database triggers and stored procedures, scheduled jobs defined in infrastructure, feature-flag states, config kept on a deploy platform. Absent from the repository is not absent from the system, and a rewrite drops it without a failing test.
- A comment-only pass is a cleanup with its own proof obligation: "comment-only" is shown by comparing the comment-stripped source at HEAD against the working tree, never asserted (`rules/comment-pass.md`).
- Simplification red flags: needing to modify tests (behavior changed, not simplified); removing error handling to make code "cleaner". Simplicity is comprehension speed, not line count.

Not a violation — leave these alone:

- Two copies of similar logic. The helper is earned on the third occurrence or by a shared invariant (core Coding rule); extracting at two couples call sites that only look alike, and a wrong shared function costs more than the duplicate did.
- A long function that is a flat sequence of steps with no branching. Length alone is not complexity, and splitting it into single-caller helpers makes the reader jump between definitions to follow one story.
- Validation and error handling at a trust boundary. It looks defensive because it is: the core rule bans runtime type-guards on internal calls, never checks on input the process does not control.
- A branch that looks wrong and is pinned by a passing test. Treat it as documented behavior until someone proves otherwise — Chesterton's Fence covers logic, not only the code that is obviously load-bearing.
- Naming, layout, or an abstraction that departs from your preference and matches the surrounding file. Repository style outranks generic best practice, and a consistency-only diff spends review attention it did not earn.
- Dead code you did not create. Report it; deleting it is its own task with its own blast radius (core Coding rule).

Done for a refactor means:

- The diff is smaller or clearer than before; behavior is preserved.
- Checks pass per the core Verification gate, or any missing verification is explicitly reported.
- Public API renames are avoided or documented as requiring a separate migration.
- The final response lists files changed, behavior-preservation evidence, and risky areas not touched.
