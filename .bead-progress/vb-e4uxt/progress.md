# vb-e4uxt: loom: fix loom-run journal_writer_queue model 'unreachable pub' compile failure

## Scope (per bead prompt)
- ONLY touch the loom test model file found via `rg -n 'journal_writer_queue' .` (search
  upstream of `crates/vb_storage/tests/loom/`), the corresponding journal modules, or
  the loom task definition in `.moon/tasks/loom.yml`.
- Disjoint from vb-11ti1/vb-06t25/vb-s8qon.

## Reference files read
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`

## Site located
The `journal_writer_queue` model lives at:

- **File:** `crates/vb_runtime/src/models/loom/journal_writer_queue.rs`
- **Mod entry:** `crates/vb_runtime/src/models/loom/mod.rs:22` → `pub mod journal_writer_queue;`
- **Discovery path:** `rg -n 'journal_writer_queue'` returns matches under
  `crates/vb_runtime/src/models/loom/`, not `crates/vb_storage/tests/loom/`.
  `xtask/src/loom.rs` (the harness used by `.moon/tasks/loom.yml:loom-run`)
  resolves the model path as `crates/vb_runtime/src/models/loom/<name>.rs`.
- **vb_runtime lint policy:** `crates/vb_runtime/src/lib.rs:3` → `#![deny(unreachable_pub)]`
  (so any `pub` inside the model that is unreachable from outside would fail the lane).

## State of the offending file (initial scan)

`crates/vb_runtime/src/models/loom/journal_writer_queue.rs` contains:
- 1 private `struct JournalWriterQueue { pending, capacity }`
- 1 private `impl JournalWriterQueue { new, try_append, drain, pending, check_invariants }`
- 3 `#[test]` functions

`rg -n '\bpub\b' crates/vb_runtime/src/models/loom/journal_writer_queue.rs` → **0 matches**.

The struct field, methods, and helper functions are all module-private. There is no
`pub` field of any kind, and therefore no `pub` field whose containing type would be
considered unreachable by `rustc`'s `unreachable_pub` deny.

## Visibility before / after
- **Before:** N/A — the file already has no `pub` modifiers on items.
- **After:** N/A — no change required. Visibility is already at the lowest
  appropriate level (module-private).

## Why the bead's "pub field of public type" framing no longer applies
The bead description suggests the model was recently split and a `pub` item was
exposed without a reachable path. Inspection of the file shows the model was
authored (commit `822dbc905`, 2026-06-10) with all items already private. There
is no `pub` field, `pub` struct, or `pub` function inside `journal_writer_queue.rs`
for `rustc` to flag as `unreachable_pub`.

The wrapping module path is `pub mod journal_writer_queue;` so
`cargo test --lib models::loom::journal_writer_queue` can still discover the
tests; the module itself is reachable from `pub mod loom;` in
`crates/vb_runtime/src/models/mod.rs:4`.

## Verification commands run

| Command | Exit |
|---------|------|
| `moon run :loom-run 2>&1 \| tee /tmp/vb-e4uxt/loom-final.txt` | **0** (5/5 models PASS, including `journal_writer_queue`) |
| `RUSTFLAGS="--cfg loom" cargo check -p vb_runtime --features loom` | **0** (0 errors, 13 warnings, none `unreachable_pub`) |
| `RUSTFLAGS="--cfg loom" cargo check -p vb_runtime --features loom --all-targets` | **0** (0 errors, 17 warnings, none `unreachable_pub`) |
| `cargo clean -p vb_runtime` then re-check under loom cfg | **0** (clean build, no `unreachable_pub`) |
| `cargo check -p vb_storage --all-features --all-targets` | **0** |
| `cargo test -p vb_storage --all-features --no-run` | **0** |

`moon :loom-run` model PASS lines (from `/tmp/vb-e4uxt/loom-final.txt`):

```
PASS: Loom model 'journal_writer_queue' completed successfully
PASS: Loom model 'action_completion_cancel' completed successfully
PASS: Loom model 'timer_fired_cancel' completed successfully
PASS: Loom model 'shutdown_drain' completed successfully
PASS: Loom model 'bounded_queue' completed successfully
```

`rg -i "unreachable" /tmp/vb-e4uxt/loom-final.txt` → **0 matches**.

## Note on bead description vs actual `loom-run` task
The bead prompt says "`loom-run` Moon task compiles a loom model under `-D warnings`".
The actual `.moon/tasks/loom.yml:loom-run` script invokes
`cargo xtask loom --model <name>`, which runs plain `cargo test
-p vb_runtime --features loom <model>` with `RUSTFLAGS="--cfg loom"` — there is
no `-D warnings` flag in the xtask command (`xtask/src/loom.rs:39-46`). The
build still emits `dead_code` / `unused_imports` warnings, but those are not
`unreachable_pub` and are pre-existing. They do not fail the lane today, but
they would become failures if a future `lint-src` task adds `-D warnings` to
the loom model compile.

## Residual warnings (NOT touched — out of scope per bead prompt)
The following warnings are present in the loom-model compile under
`cfg(loom)`. They are pre-existing across the model files, not specific to
`journal_writer_queue`, and the bead scope says "Do not refactor the journal
module". Listing only as follow-up candidates for a separate bead:

| File | Site | Warning |
|------|------|---------|
| `crates/vb_runtime/src/models/loom/journal_writer_queue.rs:11` | `use std::sync::Arc` | `unused_imports` |
| `crates/vb_runtime/src/models/loom/journal_writer_queue.rs:14` | `struct JournalWriterQueue` | `dead_code` (never constructed outside the loom closure — false positive) |
| `crates/vb_runtime/src/models/loom/journal_writer_queue.rs:19-55` | associated items `new`, `try_append`, `drain`, `pending`, `check_invariants` | `dead_code` (same) |
| `crates/vb_runtime/src/models/loom/bounded_queue.rs:10-11` | unused `Arc` / `Mutex` imports + `dead_code` for `BoundedQueue` and 6 associated items | `unused_imports` / `dead_code` |
| `crates/vb_runtime/src/models/loom/shutdown_drain.rs:10-11` | unused `Arc` import + unused `AtomicUsize, Ordering` import | `unused_imports` |
| `crates/vb_runtime/src/models/loom/action_completion_cancel.rs:10` | unused `Arc` import + `dead_code` for `ActionState`, `ActionTicket`, 6 associated items | `unused_imports` / `dead_code` |

These are real loom-model artifacts (the `dead_code` warnings are misleading
because loom models are constructed inside the `loom::model` closure, where
rustc cannot see the construction site). A dedicated cleanup bead would add
`#[allow(dead_code)]` at the module level of each model file and drop the
unused imports.

## Final status
PASS — `moon :loom-run` exits 0. All five loom models pass under
`RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --features loom`, including
`journal_writer_queue`. No code change was necessary: the model file
`crates/vb_runtime/src/models/loom/journal_writer_queue.rs` already has zero
`pub` items, so the `unreachable_pub` deny at the crate root
(`crates/vb_runtime/src/lib.rs:3`) is not triggered. The bead's described
failure mode is no longer present in the codebase.

## Residual risk
None for the `journal_writer_queue` model itself. The residual
`dead_code`/`unused_imports` warnings are listed above as follow-up work but
do not block the loom lane.