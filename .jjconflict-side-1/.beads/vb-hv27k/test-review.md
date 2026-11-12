# vb-hv27k Test Review — vb-ja1zd validation approach

**Reviewer**: DeepSeek (independent, vb-hv27k)
**Verdict**: APPROVED
**Mode**: config/script validation inquisition (no Rust test suite in this bead's scope; the change is a shell-script path contract in `.moon/tasks/all.yml`).

## What was validated (and how)

### 1. Author helper robustness — positive + negative

The author's `.beads/vb-ja1zd/evidence/path-fix-validation.py` was run unmodified, both directions:

- **Positive** (`author-helper-positive`, exit 0): extracts all three scripts, `bash -n` passes, `--target-dir` matches lookup, ASAN shared / TSAN separate, build-log redirection present, `cargo fuzz list` → 93 targets.
- **Negative equal-dirs fixture** (`author-helper-negative`, exit 1): a temporary copy of `.moon/tasks/all.yml` with `fuzz-tsan` `--target-dir` and bin lookup changed from `fuzz-tsan` to `fuzz-asan` (equal ASAN/TSAN dirs). The helper correctly fails with `TSAN and ASAN use the same target dir (race condition)`. Nonzero exit proves the isolation check is fail-closed, not vacuous.

The negative fixture lives in `/tmp/vb-hv27k-neg/` only; it does **not** modify the production `.moon/tasks/all.yml` (diff vs `HEAD` shows exactly the two intended TSAN→ASAN lines in the fixture copy, zero changes to the repo).

### 2. Independent validation (not trusting the helper)

A separate extraction (`/tmp/vb-hv27k-indep-check.py`) re-parses the three scripts with its own logic and asserts, all PASS (exit 0):

- `bash -n` on all three scripts → exit 0 each.
- `--target-dir` == lookup prefix for all three.
- TSAN dir != ASAN dir.
- `>build.log 2>build.err` redirection present in all three.
- fail-closed `"${#fuzz_targets[@]}" -eq 0` guard present in fuzz-smoke + fuzz-tsan (fuzz-regress is artifact-driven, no target listing — unchanged design).
- fail-closed `[ -x ]` / `[ ! -x ]` executable presence checks present in all three.
- sanitizer flags unchanged (TSAN `--sanitizer thread --build-std`; ASAN tasks no explicit flag).
- target triple, `timeout 30s`/`5s`, corpus copy, artifact `find` all unchanged.

### 3. Config validity (read-only)

`moon query projects` exit 0; `ruby -ryaml` parses `.moon/tasks/all.yml` (45 tasks, all three fuzz tasks present).

## Distinction: config/read check vs fresh TSAN validation

This review is a **config/read/syntax validation only**. It does **not** claim a fresh sanitizer build+run. The 93-target count comes from `cargo fuzz list` (manifest enumeration), not from executing instrumented binaries. The final canonical ASAN+TSAN rebuild + run from the frozen commit is owned by root — no cold rebuild was performed here.

Transparency: an accidental `moon check velvet-ballistics` (a Moon command that, unlike read-only `moon query`, executes affected `runInCI` tasks) began a cold fuzz build in this isolated workspace. It was terminated immediately and no fresh sanitizer validation was produced or relied upon. The only config-read evidence used is `moon query projects` (exit 0) and `ruby -ryaml` parse; all remaining validation in this review is script-extraction/`bash -n`/path-contract checks.

## Robustness findings on the helper (non-blocking)

- The helper's `cargo fuzz list` subprocess in the negative fixture dir reports an unrelated "could not find a cargo project" error (expected — the fixture has no workspace). It does not mask the isolation finding; both errors appear and exit is nonzero.
- `extract_task_script` and `get_binary_path` are regex-based and would miss a `bin` written via a differently-shaped assignment; but the independent check above re-verifies the actual paths, so the contract holds regardless of helper regex shape.

No LETHAL / MAJOR / MINOR findings.
