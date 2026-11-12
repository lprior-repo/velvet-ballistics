# vb-hv27k Code Review — vb-ja1zd three-lane sanitizer path fix

**Reviewer**: DeepSeek (independent, vb-hv27k)
**Verdict**: APPROVED
**Source reviewed**: `e3c6f4ee` (`vb-ja1zd: consistently isolate ASAN and TSAN artifacts and retain raw build logs`)
**Baseline**: `746221674` — full delta includes intermediate `ffdcfbd6d` (1-line TSAN diagnosis) then the `e3c6f4ee` lane fix.

## Scope of the exact FULL delta (746221674..e3c6f4ee)

```
 .beads/vb-ja1zd/contract.md          |  53 +++
 .beads/vb-ja1zd/evidence/*           |  (7 files, doc/evidence only)
 .beads/vb-ja1zd/handoff.md           |  52 +++
 .moon/tasks/all.yml                  |  20 +-
 11 files changed, 488 insertions(+), 8 deletions(-)
```

Only `.moon/tasks/all.yml` is a production gate. No `crates/**`, no `scripts/**`, no `Cargo.*`, no other `.moon` config touched. Scope is the exact three-lane contract.

## Contract verification (actual current state, not handoff assertions)

| Requirement | fuzz-smoke | fuzz-tsan | fuzz-regress |
|---|---|---|---|
| explicit `--target-dir` | `"$PWD/target/fuzz-asan/build"` | `"$PWD/target/fuzz-tsan/build"` | `"$PWD/target/fuzz-asan/build"` |
| lookup path | `target/fuzz-asan/build/x86_64-unknown-linux-gnu/release` | `target/fuzz-tsan/build/x86_64-unknown-linux-gnu/release/${tgt}` | `target/fuzz-asan/build/x86_64-unknown-linux-gnu/release/${tgt}` |
| sanitizer | default ASAN (no flag) | `--sanitizer thread --build-std` | default ASAN (no flag) |
| target triple | `x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-gnu` |

- **ASAN isolated from TSAN**: ASAN tasks share `target/fuzz-asan/build`; TSAN uses `target/fuzz-tsan/build`. Independent dirs ⇒ no instrumented-binary race when ASAN and TSAN run concurrently. Verified by helper cross-task check and my independent check (`tsan=target/fuzz-tsan/build asan={'target/fuzz-asan/build'}`).
- **Relative vs absolute resolve to same root**: build uses `"$PWD/target/..."` (absolute); lookup uses `target/...` (relative). Both expand against the same process CWD (`$PWD` ≡ CWD in bash, script does not `cd` between build and lookup), so they resolve identically.
- **Unchanged, confirmed against baseline diff**: `--sanitizer thread` + `--build-std` (TSAN), default ASAN (others), `--target x86_64-unknown-linux-gnu`, `RUSTFLAGS="-Dwarnings"`, `timeout 30s` (smoke) / `timeout 5s` (regress), corpus copy `fuzz/corpus/${fuzz_target}`, artifact `find fuzz/artifacts`, and the fail-closed executable checks (`[ -x ]` / `[ ! -x ]` → exit 1 / rc=1).
- **No gate relaxation**: no `touch`/`ln -s`/fake binaries, no waivers, no skipped targets, no `continue`-past-missing. Every missing-binary path still exits nonzero.

## Corrected earlier Qwen mistakes (both present in current state)

1. **TSAN log redirection restored**: the `>target/fuzz-tsan/build.log 2>target/fuzz-tsan/build.err` line is present in the current `fuzz-tsan` script (the block was rewritten from `flock … || { cat …; exit 1; }` to `if ! flock …; then cat …; exit 1; fi`, preserving redirection and fail-closed semantics).
2. **`asan_dirs.pop()` mutation fixed**: `.beads/vb-ja1zd/evidence/path-fix-validation.py` now uses non-mutating `list(asan_dirs)[0]` in both the shared-dir and TSAN-separation branches, so the TSAN check is no longer skipped by a set mutation.

## Evidence

- `.beads/vb-hv27k/evidence/author-helper-positive.stdout` — helper exit 0; 3× `bash -n` OK; `--target-dir` == lookup for each; `ASAN tasks share target dir`; `TSAN uses separate dir`; `Actual fuzz targets: 93`.
- `.beads/vb-hv27k/evidence/indep-validation.stdout` — independent extraction + `bash -n` (all 3 exit 0), lookup-under-target-dir, TSAN isolation, log redirection, fail-closed guards, sanitizer flags, triple, timeout, corpora all PASS (exit 0).
- `.beads/vb-hv27k/evidence/moon-config-valid.stdout` — `moon query projects` exit 0; `ruby -ryaml` parses `.moon/tasks/all.yml` (45 tasks; fuzz-smoke/tsan/regress all present).

## Remaining (root-owned, out of scope here)

Final canonical ASAN+TSAN rebuild + run from the frozen commit, and `moon ci`.

**Config/read vs fresh validation distinction**: this review is config/read/syntax validation only. No fresh sanitizer build was completed or claimed. Note for full transparency: an accidental `moon check velvet-ballistics` (run while probing Moon config validity) began a cold `cargo fuzz build` (TSAN `--build-std` compiling core/std, plus ASAN) in this isolated workspace; those in-flight builds were terminated immediately (`pkill`), two `verify-tlc`-clobbered tracked files (`.evidence/verus/{step_budget,summary}.txt`) were restored via `jj file show -r @-`, and the working copy is back to no-changes. The canonical build remains root-owned and unperformed here.
