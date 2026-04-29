# vb-7ph Documentation Update Report

## Files Changed

- `velvet-ballistics-MASTER.md`
- `vb-7ph/doc-update-report.md`

## Sections Hardened

- Added top-level authority note and canonical spelling exception for unavoidable existing repository path/file references.
- Sharpened prime directive for Rust-nightly, no-unsafe, no-panic, single-server, YAML-authoring-only, numeric state machine execution, Fjall/Postcard, direct API, binary IPC, and mandatory generated Rust maxperf mode.
- Expanded canonical naming and rejection rules for `velvet-ballastics`, `velvet_ballastics`, bead rig/database, and language version.
- Added HTTP/JSON exclusion rule for v1 runtime core and future cold-path adapter boundary.
- Expanded mandatory Rust tooling, install block, nightly governance, MSRV distinction, unstable feature allowlist, and `RUSTC_BOOTSTRAP` rejection.
- Corrected library table rationale for `ArrayQueue`, `rtrb`, `mio`, Postcard, Fjall, Saphyr, and hot blob handle policy.
- Added maximum performance rules for runtime architecture, hot/cold layout, bounded queues, persistence, compilation, and no post-admission allocation in turbo mode.
- Hardened hot/cold data layout and HashMap nuance: maps are allowed in parser/validator/compiler/diagnostics/tests, but not in hot runtime or generated state.
- Expanded forbidden hot-path API list with formatting, runtime maps, serde_json, YAML parser calls, filesystem/env reads, string action lookup, per-step spawn, blocking Fjall persistence, unchecked ops, and allocation bans.
- Added compile-time and runtime resource contracts.
- Corrected core snippets for handle-based `SlotValue`, `FiniteF64`, `ConstValue::to_slot_value`, `CoreError`, `StepBudget`, step-state mutation methods, and `SetConst` error handling.
- Replaced final IR contract with required final primitive set and choose-lowering rules.
- Added binary record envelope, precise Fjall durability semantics, recovery/replay digest mismatch behavior, and required persistence records.
- Expanded Action ABI with compile-time `ActionId`, `ActionInput`, `ActionOutput`, `ActionTicket`, Ready/Suspended/Failed outcomes, and generated match dispatch.
- Replaced IPC command set with required v1 commands: `SubmitRun`, `SubmitRunInline`, `CancelRun`, `InspectRun`, `ListEvents`, `AnswerAsk`, `CompleteAction`, `FailAction`, `DrainTrace`, `Health`, `Shutdown`.
- Removed unverified `rust-version = "1.91"` from the workspace Cargo contract snippet.
- Replaced implementation phases with Phase -1 through Phase 36 revised build order.
- Expanded mandatory function lists across `vb_core`, `vb_yaml`, `vb_validate`, `vb_expr`, `vb_compile`, `vb_storage`, `vb_runtime`, `vb_ipc`, and `vb_codegen`.
- Added required justfile targets, CI gates, sanitizer nightly job requirement, and expanded hard tooling commands.
- Expanded mandatory tests, benchmark list, benchmark metadata, and acceptance metrics.
- Hardened bead work breakdown with phase parent beads, function child beads, benchmark/fuzz/P0 beads, required first beads, and example `bd` commands.
- Replaced final Definition of Done with a stricter 27-point mechanical acceptance contract.

## Remaining Risks

- The master document intentionally still contains `velvet-ballistics` in the filename/path and migration rejection examples because the current repository and authoritative file are already named that way.
- Some code snippets are mechanical contracts rather than compile-checked Rust source; implementation beads must convert them into compiling crate code and tests.
- The current repository still has existing local untracked directories unrelated to this bead; they were not modified.

## Verification

- Required section/term scan passed for Holzmann matrix, mandatory tooling, strict nightly governance, hot/cold layout, forbidden hot-path APIs, resource contracts, final IR, Fjall persistence, Action ABI, binary IPC, implementation phases, CI gate, bead work breakdown, final DoD, `SubmitRunInline`, `ConstValue::to_slot_value`, and `StepBudget::try_take`.
- Hard contradiction scan passed with no hits for `velvet/v1`, optional generated Rust wording, manual-only trigger wording, tree-heavy value wording, silent fallback wording, `panic = "abort"`, or old HTTP/webhook optional wording.
- Documented exception scan still finds `rust-version = "1.91"` only inside the rule saying not to hardcode it, `HashMap<String, Value>` only in runtime-state bans, `serde_json::Value` only in forbidden hot-path APIs, and legacy IR names only in an explicit migration-only note.
