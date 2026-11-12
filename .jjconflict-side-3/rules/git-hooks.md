# Git hooks and pre-commit gates

Read this when setting up or changing pre-commit, commit-msg or pre-push automation, with any hook manager.

- Pre-commit runs on staged files only, not the whole repo — fast enough that nobody is tempted by `--no-verify`. Whole-repo lint/typecheck/test belongs in CI, not the commit hook.
- Hooks install as part of the project's normal setup step, so every clone gets them without a manual step.
- The hook runs the formatter and linter with zero tolerance on the staged set; auto-fixable issues fix-and-restage, non-fixable ones block the commit.
- A `commit-msg` hook validates the message against the repo's convention (`commitlint` for Conventional Commits) — `rules/commit-messages.md` states the format, and the hook is where it stops being advisory.
- Secret scanning runs in the hook as well as in CI (`gitleaks git --pre-commit --staged`, `detect-secrets`): once a key reaches a pushed commit the remedy is rotation, not a revert (core Security rule).
- gitleaks 8.19 renamed the old commands — `protect --staged` → `git --pre-commit --staged`, `detect` → `git`, `detect --no-git` → `directory`; the old spellings still run but are hidden from `--help`.
- The hook is blind to `.gitignore`d paths: it scans the staged set, and a secret written to an ignored path (`secrets/`, `*.local`, `.env`) is never staged, so no hook and no working-tree scan ever sees it. A clean hook run proves the staged set is clean, nothing more.
- Give the hook a time budget and hold it — a pre-commit over a couple of seconds trains everyone to reach for `--no-verify`. Anything slower moves to pre-push or CI.
- Keep hooks deterministic and side-effect-free — no network calls, no writes outside the staged files; a flaky hook trains developers to bypass it.
- CI re-runs the same gates: hooks are a fast local shortcut, never the source of truth, since `--no-verify` and clients that bypass the hook manager skip them.
- Never weaken a shared hook to fix a local-only problem (core Boundaries rule) — fix your environment instead.
