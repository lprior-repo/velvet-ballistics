# Black Hat Review: vb-kkvb

STATUS: APPROVED

## Phase 1: Contract & Bead Parity

Result: PASS.

- Contract requires 20 stable top-level command families. Executable probe confirmed `./target/debug/xtask --help` lists every required family exactly once: `ai-context`, `ai-plan`, `ai-check`, `ai-evidence`, `invariants`, `scans`, `cert-check`, `perf`, `replay`, `crash`, `diff`, `mutants`, `loom`, `kani`, `fuzz`, `prop`, `repro`, `test-plan`, `review`, `why-failed`.
- Typed routing exists and is closed: `xtask/src/lib.rs:11-33` defines `CommandFamily`; `xtask/src/lib.rs:89-95` defines `XtaskCommand`; `xtask/src/lib.rs:62-86` maps public spellings to enum variants.
- Required-command parsing is now split and boring: `parse_xtask_command` delegates to `collect_args`, `top_level_command`, `classify_top_level_command`, and `parse_required_command` at `xtask/src/lib.rs:292-343`.
- Structured JSONL parity passed for all 20 required families with exact required fields `command`, `status`, `message`, `next_steps`. Each command returns `status=deferred`, so no hidden claim that deeper engines ran.
- Error contract passed for hostile samples: unknown command, wrong case, missing `--format`, invalid `--format yaml`, and missing `--bead` all exited non-zero with typed diagnostics (`UnknownCommand`, `MissingRequiredInput`, `InvalidInput`) and remediation.
- Legacy command path is preserved through `XtaskCommand::Legacy` and `run_legacy_cli`; this bead did not regress documented UI/gate routes.

## Phase 2: Farley Engineering Rigor

Result: PASS.

- Function-shape repair is real. Static probe over `xtask/src/*.rs` found `violations=0` for functions over 25 lines.
- Previously rejected functions are repaired:
  - `xtask/src/lib.rs:292-298` `parse_xtask_command` is 7 lines.
  - `xtask/src/lib.rs:345-357` `route_command` is 13 lines.
  - `xtask/src/lib.rs:540-549` `render_json_line` is 10 lines.
  - `xtask/src/main.rs:86-96` `main` is 11 lines.
  - `xtask/src/main.rs:674-680` `run_ai_profile` is 7 lines.
  - `xtask/src/gates.rs:65-70` `Gate::command` is 6 lines.
- Functional core / imperative shell separation is good enough for this bead: pure command classification/render planning lives in small helpers; filesystem/stdout effects are isolated in shell helpers (`prepare_ai_profile_output`, `run_ai_profile_plan`, `write_ai_profile_output`, `write_stdout`).
- Tests prove behavior, not only implementation trivia: Red phase and density suites exercise typed routing, schema stability, diagnostic failures, duplicate/schema drift, dependency boundary, and mutation/order resilience.

## Phase 3: NASA-Level Holzman Rust

Result: PASS.

- `xtask/src` has `#![forbid(unsafe_code)]` in `main.rs` and `lib.rs`. Static grep found no `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`, or `dbg!` in bead-owned xtask source.
- Error taxonomy matches the contract at `xtask/src/lib.rs:97-126`: `UnknownCommand`, `MissingRequiredInput`, `InvalidInput`, `OutputRenderFailed`, `DependencyBoundaryViolation`, `Unavailable`, `InternalInvariantViolation`.
- Illegal command families are unrepresentable once parsed: public names map into `CommandFamily`, not free-form handler strings.
- Runtime dependency boundary passed by `cargo metadata`: `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc` have no forbidden direct dependency on `xtask`, `clap`, `serde_json`, `serde_yaml`, `reqwest`, `hyper`, `toml`, or `serde-saphyr`.
- Non-interactive scan passed for bead-owned xtask source: no `stdin`, `read_line`, confirm/prompt crates, editor launch, or SSH/SCP prompt source was found.

## Phase 4: Ruthless Simplicity & DDD

Result: PASS.

- Domain shape is explicit: command families, output format, deferred reason, structured status, validated registry, and workspace manifest all have named types.
- No hidden global mutable state was found in the command routing path. Route construction uses argv input and immutable environment references.
- Traversal-safe evidence handling is enforced at the shell boundary: `xtask/src/main.rs:754-764` rejects bead ids unless every character is ASCII alphanumeric, `-`, or `_`. Runtime probe rejected `../vb-bh-escape` and created no escape directory.
- Placeholder semantics are honest. Required command families return `deferred`; legacy AI profile evidence uses explicit synthetic/fixture-backed labels and is outside the deeper-engine implementation non-goal. That is not hidden test theater for this bead.

## Phase 5: The Bitter Truth

Result: PASS.

- The prior rejection was for oversized command-shell functions. That specific failure is dead.
- Red Queen approval is credible here: 0 bead-owned survivors, routing/JSONL/diagnostics/traversal/runtime-boundary/function-shape/order/mutation probes all passed.
- Remaining repo-wide debt and legacy fixture-backed gate implementation are not bead-owned blockers under the supplied working rule. The bead under review is the typed xtask command shell expansion, and that surface now behaves and reads like boring code.

## Evidence Executed

- `rtk cargo build -p xtask --quiet` → PASS.
- `cargo +nightly test -p velvet-ballistics-workspace-tests --test vb_kkvb_xtask_red_phase --quiet` → 368 passed.
- `cargo +nightly test -p velvet-ballistics-workspace-tests --test vb_kkvb_xtask_density_explicit --quiet` → 286 passed.
- `cargo +nightly test -p xtask cmd_ai_deep --quiet` → focused mutation resilience passed, including 2 `cmd_ai_deep` tests.
- Static function-shape probe over `xtask/src/*.rs` → `violations=0` for functions over 25 lines.
- CLI probe over all 20 required families with `--format jsonl` → every command returned valid one-line JSON with required fields and `status=deferred`.
- CLI diagnostic probe → `no-such-command`, `AI-plan`, `ai-plan --format`, `ai-plan --format yaml`, and `ai-plan --bead` all failed closed with typed diagnostics.
- Traversal/evidence probe → `ai-fast --bead ../vb-bh-escape` rejected with `Invalid bead id`; `ai-deep --bead vb-bh-final` wrote safe evidence and cleanup removed it.
- Runtime dependency probe via `cargo metadata --format-version 1 --no-deps --quiet` → no forbidden runtime crate edges.
- Static non-interactive grep → no prompt/editor/stdin pattern in bead-owned xtask source.
- `cargo +nightly test -p xtask --quiet` → PASS: 75 unit tests, 171 main tests, 2 command-shell stdout tests, 19 integration gates, 9 UI release errors, 1 UI release tooling red phase, 2 UI release gates.

## Verdict

APPROVED. The repaired bead-owned xtask command shell meets contract parity, Farley function-shape constraints, Holzman Rust safety constraints, runtime dependency isolation, traversal-safe evidence handling, and Red Queen mutation/order pressure. No lethal blockers remain.
