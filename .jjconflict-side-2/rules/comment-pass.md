# Comment pass

Read this when running a comment-only cleanup pass over existing files: what the pass must leave alone and how to prove it changed nothing but comments. The rules for the comment text itself are in `rules/code-comments.md`.

## Never changed by a comment pass

- Behavior. A pass that changes behavior is a failed pass: public API contracts, signatures, exported names, storage keys, message names, request and response shapes, env var names, timeouts, config values, and test expectations stay stable unless the task explicitly requires a functional change.
- Functional directives, wording included, because a tool matches the text or a reviewer needs it verbatim: `biome-ignore`, `eslint-disable`, `@ts-expect-error`, `@ts-check`, `/// <reference>`, `# noqa`, `# type: ignore`, `# shellcheck disable=`, `//go:` directives, `//nolint`, encoding and modeline headers, SPDX headers, shebangs, `nosemgrep`, test-skip reasons, and any other comment a tool parses.
- TODO and FIXME. Flag one that looks obsolete; do not resolve or delete it as part of a comment pass.
- A comment inside a string that is injected as code (an in-page evaluation body, a generated file's template): its prose may be fixed under these rules, but never a line of its code and never a value the surrounding code interpolates. A comment in a template that emits a generated file needs the generator re-run, so leave it.
- Line width — never reflow to satisfy a character limit (core rule); the repo's formatter config governs. If a rewrite leaves a ragged short line inside an already-wrapped block, re-wrap that block's own lines and re-read it afterwards to confirm no word was dropped.

## Proving a pass changed nothing

- Do not assert that a pass was comment-only, prove it: print each changed file with comments removed at HEAD and in the working tree, and compare the two outputs — identical output means the diff is comment-only. The reliable rendering is the language's own parser or tokenizer printing the file back without comments (Python `tokenize`, the TypeScript compiler already in a JS/TS project). Where none is at hand, strip that language's comment markers while respecting quoted strings and heredocs, collapse whitespace, and compare; a naive regex strip eats a URL inside a string and a comment marker inside a template literal. Files that legitimately differ are the ones where a comment lives inside a template literal or a heredoc; for each of those, show separately that every changed line in the diff is a comment line.
- Then the repo's own gates, read for real output and exit codes rather than a wrapper's summary: lint, typecheck for every config in the repo, and the full unit suite. A single failing run that later runs cannot reproduce is a flake — report it with the evidence, never silently retry until green (core Verification gate).
- Beyond a few thousand comment lines, split the work into partitions along the repo's own directory boundaries, with no file in two partitions — that disjointness is what makes parallel editing safe without a worktree per agent. Each agent gets its exact file scope, these rules, "do exactly this and nothing else", and "do not run lint or tests" so parallel runs cannot collide; verification runs centrally once they finish. An agent that notices a second wording pattern reports it and does not act on it.
