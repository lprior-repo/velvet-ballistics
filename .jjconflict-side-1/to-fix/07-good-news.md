# Good News from the Audit

The report is bad because the master contract is strict, not because the repo is empty. There is a lot of real foundation here.

## Core and IR

- Numeric ID types and compiled IR structures exist in `vb_core`.
- `CompiledWorkflow::try_from_parts` performs structural validation for node IDs, table references, reachability, forward edges, symbol bounds, and resource budget in the core layer.
- `Choose` and `ChooseSlot` final IR forms exist and use numeric `ExprIdx`/`SlotIdx`, not raw YAML strings, in core execution paths.
- `SlotValue` is handle-based with `SymbolId`, `ListId`, `ObjectId`, and `BlobId` handles.
- `FiniteF64` rejects non-finite values in construction and deserialize paths.

## YAML and Validation

- `vb_yaml` has strict profile machinery for source size, parser depth, forbidden tags/features, multiple documents, anchors/aliases/merges, duplicate keys, and ambiguous YAML 1.1 scalars.
- Parser/validator surfaces exist for schema, references, control flow, type-taint, resources, and diagnostics.
- Runtime finish taint is no longer simply compile-rejected; preservation behavior appears represented in validation tests.

## Runtime

- Core deterministic drive loop stops on budget, suspension, and finish.
- `StepBudget` is bounded and tested for zero/one budget behavior.
- `RunFrame` uses boxed arrays and checked slot/taint/state accessors.
- Runtime shard queues use bounded `ArrayQueue` with typed `QueueFull` behavior.
- Runtime shape is shard-owned, not a global `Arc<Mutex<RunState>>` map.
- Runtime full engine dispatch has arms for all major final IR families, even though source lowering/evidence still lags.
- Trace ring is bounded and counts drops.

## Storage

- All 9 required Fjall keyspaces are declared and opened.
- Key construction uses prefix bytes plus big-endian numeric fields.
- Record envelope code validates magic/schema/kind/header length/payload length/CRC/digest before Postcard decode in the normal path.
- Typed storage/decode errors exist for bad magic, unsupported schema, unknown kind, family mismatch, header mismatch, payload too large, checksum mismatch, digest mismatch, EOF, Postcard decode, and migration errors.
- Strict direct storage journal path appears to persist admission records before inserting live runs.

## IPC and CLI

- `vb_ipc` defines all 11 required master commands, even though it also has extra commands that need reconciliation.
- Standalone IPC frame decode validates magic, version, reserved, command, and payload bound before typed payload decode.
- IPC payloads use typed Postcard enums, not text command routing.
- Server dispatch covers required handler arms.
- Deferred Rust codegen is not exposed as `compile --emit rust` in the active CLI.
- Deferred UI command is not in the primary CLI parser.

## CI and Tooling

- `rust-toolchain.toml` correctly pins `nightly-2026-04-28` with `rustfmt`, `clippy`, `rust-src`, `miri`, and `llvm-tools-preview`.
- Workspace lints deny or forbid the right broad classes: unsafe, unwrap, expect, panic, todo, unimplemented, dbg, indexing/slicing, arithmetic side effects, `as` conversions, ignored must-use, and locks across await.
- Moon has task skeletons for fmt, lint, check, test, feature powerset, miri, coverage, mutants-smoke, fuzz-smoke, source-length, and bench-build.
- Required fuzz implementation functions exist for YAML, expressions, IPC frames, journal events, and compiled IR. The naming/wiring is the problem, not total absence.
