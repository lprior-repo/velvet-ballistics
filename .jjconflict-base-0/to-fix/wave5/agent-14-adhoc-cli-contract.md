# Wave 5 / Agent 14 — Ad-hoc CLI-Contract Deep-Dive

**Scope:** 4 bugs (vb-yfsc4, vb-ykph4, vb-yq255, vb-z3sdl) reviewed against the
master §33 CLI command surface, the `action inspect <action-name>` contract, and
the `--emit postcard` typed-envelope contract. None of the four bug IDs touch
the `vb_cli` crate directly; their effect on the CLI surface is indirect.

**Package:** `velvet-ballistics` (crate dir `crates/vb_cli/`, package name
`velvet-ballistics` per `crates/vb_cli/Cargo.toml:2`). Tested with:
`/home/lewis/.cargo/bin/cargo test -p velvet-ballistics --lib --no-fail-fast`.

---

## Per-bug results

| bug-id | pri | command-surface | action-inspect-name | emit-postcard-typed | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|
| vb-yfsc4 | P0 | NO DRIFT (patch in vb_storage::recover_full_journal, CLI keeps `journal.events_for_run` snapshot-optimized path) | UNCHANGED (action inspect untouched) | UNCHANGED (no CLI output path touched) | `cargo test -p velvet-ballistics --lib --no-fail-fast` | 214 passed, 0 failed | PATCHED (CLI contract intact) | `bd show vb-yfsc4` close reason: "Fixed: recover_full_journal now uses journal.events_for_run_full(run) at crates/vb_storage/src/recovery/replay/recovery_ops.rs:51". CLI consumers in `crates/vb_cli/src/{inspect,events,replay,incident_diff,incident_ops,lifecycle,bench,commands_ai_context,doctor}.rs` all call `journal.events_for_run(...)` (the snapshot-optimized, still-correct path), not `recover_full_journal`. |
| vb-ykph4 | P3 | NO DRIFT (patch in vb_runtime::shard::introspection, CLI's `cmd_inspect` uses `journal.events_for_run` + `derive_lifecycle_state_from_events`, not `InspectSnapshotFormatter`) | UNCHANGED (action inspect untouched) | UNCHANGED (CLI emits its own envelope via `output::json_out` → `encode_postcard_json_frame`) | `cargo test -p velvet-ballistics --lib --no-fail-fast` | 214 passed, 0 failed | IN_PROGRESS (CLI contract intact) | `bd show vb-ykph4` status `IN_PROGRESS`. Patch site `crates/vb_runtime/src/shard/types.rs:513` `InspectSnapshotFormatter::format_snapshot(response: &InspectResponse)` (now uses `snap.run` from `Found` arm; no external `run` parameter). Verified formatter signature at `crates/vb_runtime/src/shard/types.rs:513`. CLI's `crates/vb_cli/src/inspect.rs:23` does not call this formatter, so the in-progress runtime patch has zero CLI-surface impact. |
| vb-yq255 | P1 | NO DRIFT (clippy-only in vb_ipc::peer_credentials + server/handlers/tests) | UNCHANGED | UNCHANGED (CLI's `cmd_ipc_serve` doesn't emit postcard envelopes via `OutputFormat::Postcard`; ipc-serve has no output variant) | `cargo test -p velvet-ballistics --lib --no-fail-fast` | 214 passed, 0 failed | PATCHED (CLI contract intact) | `bd show vb-yq255` close reason: "Strict gate cargo +nightly clippy -p vb_ipc --all-targets --all-features exits 0 (verified 2026-06-20T23:00:53Z). 22 pre-existing clippy findings … closed by prior substrate repair (commit 2551cd1d4) … 0 matches for panic!/expect(/expect_err(/assert_eq!(matched, true) in both files." Confirmed `Command::IpcServe` in `args/types.rs:127-130` has no `output: OutputFormat` field. |
| vb-z3sdl | P1 | NO DRIFT (patch in vb_core::value_store::insert_object) | UNCHANGED | UNCHANGED | `cargo test -p velvet-ballistics --lib --no-fail-fast` | 214 passed, 0 failed | PATCHED (CLI contract intact) | `bd show vb-z3sdl` close reason: "Fixed in wave-15: ValueStore::insert_object now returns Err(CoreError::InvalidCompiledWorkflow { reason: 'duplicate_object_key' }) when duplicate keys are detected". vb_core is a compile/runtime invariant; vb_cli has no call site that consumed the prior silently-corrupt semantics. |

---

## Bug-level verdict summary

- bugs-checked: 4
- pass (PATCHED, no CLI contract regression): 3 — vb-yfsc4, vb-yq255, vb-z3sdl
- in-progress (still tracked open, no CLI contract regression while open): 1 — vb-ykph4
- fail (NOT-PATCHED at CLI layer): 0
- partial: 0
- unknown: 0

The four bugs in this chunk are storage/runtime/ipc/core bugs; none of them is
a vb_cli bug. They cannot be "PATCHED" or "NOT-PATCHED" at the CLI-contract
layer in the usual sense. The CLI-contract lens verdict is therefore
**"no CLI-contract regression introduced or observed for any of the four"**.

---

## CLI command-surface drift vs master §33 (independent finding)

§33 (master doc line 1294–1314) enumerates 17 canonical commands:

```
validate, compile, run, run-compiled, ipc-serve, agent-context, inspect,
events, replay, graph, system status, action list, action inspect,
incident, ai context, bench-run, doctor
```

`crates/vb_cli/src/dispatcher.rs` routes **30 Command variants**
(Help, Version + 28 subcommands); the agent-context schema in
`crates/vb_cli/src/agent_context/mod.rs:101-265` lists exactly 22 commands,
and the proptest `prop_build_commands_count_is_22` (test run: passed) pins
that count.

### §33 commands present in dispatcher

All 17 §33 commands are wired (including `system status`, `action list`,
`action inspect` via the `System*`/`Action*` dispatcher arms).

### Extras present in dispatcher but absent from §33 (CLI drift)

| extra command | dispatcher arm | source file:line |
|---|---|---|
| `status` | `Command::Status` | `dispatcher.rs:27`, `args/types.rs:79-82` |
| `verify` | `Command::Verify` | `dispatcher.rs:39-43`, `args/types.rs:96-100` |
| `explain` | `Command::Explain` | `dispatcher.rs:47`, `args/types.rs:172-176` |
| `trace` | `Command::Trace` | `dispatcher.rs:92-97`, `args/types.rs:148-153` |
| `retry` | `Command::Retry` | `dispatcher.rs:98-100`, `args/types.rs:154-158` |
| `resume` | `Command::Resume` | `dispatcher.rs:101-103`, `args/types.rs:159-163` |
| `answer` | `Command::Answer` | `dispatcher.rs:108-114`, `args/types.rs:177-183` |
| `diff` | `Command::Diff` | `dispatcher.rs:116-121`, `args/types.rs:188-193` |
| `submit` | `Command::Submit` | `dispatcher.rs:125-131`, `args/types.rs:203-209` |
| `simulate` | `Command::Simulate` | `dispatcher.rs:132-134`, `args/types.rs:199-202` |
| `cancel` | `Command::Cancel` | `dispatcher.rs:135-140`, `args/types.rs:210-215` |

That is **11 subcommand extras** plus Help/Version (utility) for a total of
13 dispatcher entries outside §33. Note that phase 31 in the same master
doc (lines 1411-1457) lists a superset that *does* include verify, explain,
diff, simulate, submit — so the drift is best read as "§33 lags phase 31 by
nine commands, dispatcher has two on top of phase 31 (`status`, `cancel`)".

### Missing from dispatcher (would be a true §33 drift)

None. Every §33 command is reachable through the dispatcher.

---

## `action inspect <action-name>` contract

Verified contract holds:
- `args/action.rs:38-72` `parse_action_inspect`: positional `action_name` is
  parsed as a **string** (UTF-8 trimmed, non-empty, ≤64 chars, no whitespace)
  with `ParseError::InvalidActionName` on failure.
- The numeric ActionId is **not** an input — the dispatcher arm
  `Command::ActionInspect { action_name, output, registry }` (`args/types.rs:91-95`)
  carries only `action_name: String`.
- Resolution path: `vb_core::action::ActionName::new(&action_name)` →
  `registry.resolve_by_name(&name)` (`action.rs:61-65`). The id is derived
  from the contract after name lookup, never accepted as CLI input.
- `write_action_contract` (`action.rs:106-117`) prints `detail.id` (numeric
  ActionId) only as part of the output body, never as a selector.

**Verdict:** `action inspect` correctly takes a name, not a numeric id. ✓

---

## `--emit postcard` typed-envelope contract

### What master §33 says (line 1316)

> "CLI structured output is a cold-path operator/agent contract … `--emit yaml`
> is the canonical structured text flag for v1; `--emit postcard` is the
> canonical binary machine-output flag where supported … Runtime machine
> artifacts remain binary/Postcard."

### What the implementation actually does

`OutputFormat::Postcard` arms in `crates/vb_cli/src/output.rs:83-87, 109-112`
route both stdout and stderr through `encode_postcard_json_frame(value)`
(`output.rs:135-147`). That function:

1. Serializes the structured `serde_json::Value` to JSON bytes via
   `serde_json::to_vec(value)` (line 136).
2. Wraps the JSON bytes in a `CliPostcardPayload { schema_version, kind,
   content_type: CliPostcardContentType::JsonUtf8, json_utf8: Vec<u8> }`
   (`cli_postcard/types.rs:36-55`).
3. Postcard-encodes the wrapper struct (`postcard::to_allocvec(&payload)`,
   `output.rs:140`).
4. Frames it with the binary CLI header (magic + schema_version + kind +
   header_len + payload_len + BLAKE3 digest + CRC32, `cli_postcard/codec.rs:46-73`).

The inner content type (`cli_postcard/types.rs:29-32`) is explicitly
`CliPostcardContentType::JsonUtf8` — a single-variant enum whose only payload
is `json_utf8: Vec<u8>`. The validation gate
(`cli_postcard/validation.rs:16-18`) **rejects any non-JSON content_type**.

### Typed vs JSON-in-Postcard verdict

The four output-format flags declared in
`agent_context/mod.rs:280-282` `output_emit_flag()` advertise
`["text", "yaml", "postcard"]` as the values, with no machine-type
discriminator. The only `--emit postcard` path that emits **typed
postcard** (i.e. a native Rust struct serialized via `postcard::to_allocvec`
rather than JSON bytes) is `compile --emit postcard`
(`crates/vb_cli/src/compile.rs:121-162`), which writes a `WorkflowParts`
struct as the binary artifact. Every other command that uses
`OutputFormat::Postcard` produces a **JSON-in-Postcard wrapper** — the
outer frame is a real binary postcard envelope, but the payload is the
command's JSON `serde_json::Value` re-encoded as bytes and stuffed into
`CliPostcardPayload.json_utf8`.

This is a **JSON-in-Postcard-wrapper violation** for all non-`compile`
postcard outputs (action inspect, action list, status, system status,
verify, validate, explain, inspect, events, replay, trace, retry, resume,
bench-run, doctor, answer, graph, diff, incident, submit, simulate,
cancel, run, run-compiled, ai-context, run --step).

**Verdict:** `--emit postcard` is **NOT** typed postcard for the
operator-output commands. Only `compile --emit postcard` is true typed
postcard. ✗

---

## Top findings

1. **JSON-in-Postcard wrapper for all operator outputs.**
   `encode_postcard_json_frame` in `crates/vb_cli/src/output.rs:135-147` and
   the `CliPostcardContentType::JsonUtf8` constraint in
   `crates/vb_cli/src/cli_postcard/validation.rs:16-18` force every
   non-`compile` `--emit postcard` path to be a JSON-bytes payload inside a
   postcard frame. This is a pre-existing structural issue, not introduced
   or fixed by any of the four bugs in this chunk.

2. **§33 ↔ phase 31 ↔ dispatcher command-list drift.** Master §33
   enumerates 17 commands; the dispatcher routes 30 Command variants
   (including 11 subcommand extras not named in §33: status, verify,
   explain, trace, retry, resume, answer, diff, submit, simulate,
   cancel). Phase 31 of the same master doc partially closes the gap by
   mentioning verify/explain/diff/simulate/submit. No §33 command is
   missing from the dispatcher.

3. **`action inspect` is name-only.** `parse_action_inspect`
   (`crates/vb_cli/src/args/action.rs:38-72`) and `write_action_inspect`
   (`crates/vb_cli/src/action.rs:56-83`) both treat the first positional
   as a string `action_name` and resolve via
   `vb_core::action::ActionName::new`. No numeric-id selector exists in
   the CLI surface.

---

## Top NOT-PATCHED items at the CLI-contract layer

For these four bugs, **no NOT-PATCHED item exists at the CLI-contract
layer** — the bugs are not in vb_cli, and the relevant CLI consumers
(inspect/events/replay/incident for vb-yfsc4; ipc-serve for vb-yq255) were
verified to remain on the snapshot-optimized or clippy-clean path. The
one in-progress bug (vb-ykph4) does not touch any CLI command path
(CLI's `cmd_inspect` does not invoke `InspectSnapshotFormatter`).

If the report must list top concerns by my CLI-contract lens, the
honest ranking is:

1. **`--emit postcard` is JSON-in-Postcard for all operator outputs**
   (pre-existing; not caused by any of these four bugs). Affects every
   command except `compile --emit postcard`.
2. **§33 ↔ dispatcher drift** of 11 subcommand extras (pre-existing;
   not caused by any of these four bugs).
3. **No targeted regression test exists for any of the four bugs at the
   vb_cli boundary** — vb-yfsc4 tests live in `vb_storage`, vb-ykph4 in
   `vb_runtime`, vb-yq255 in `vb_ipc`, vb-z3sdl in `vb_core`. The
   CLI consumer paths (`inspect.rs:23`, `events.rs:32`, `incident_diff.rs:29`,
   `commands_ai_context.rs:33`, `lifecycle.rs` ×5) all keep their
   `journal.events_for_run` calls, which exercises the snapshot-optimized
   path (intentionally), not `recover_full_journal` (which is the SR-001
   fixed path). So CLI surface is correct, but no CLI-level test pins
   the new SR-001 contract for downstream consumers.

---

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-14-adhoc-cli-contract.md`