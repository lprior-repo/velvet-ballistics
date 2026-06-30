# Codebase map: vb-core-ipc-sync-evidence

bead_id: `vb-core-ipc-sync-evidence`
title: `ipc/orchestrator: Prove local binary ingress synchronization`
workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`
mapped_at: `2026-05-15T19:41:41Z`

## Commands and evidence used

- Read `.beads/vb-core-ipc-sync-evidence/STATE.md` and `.beads/vb-core-ipc-sync-evidence/baseline-report.md`.
- Ran `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-ipc-sync-evidence --json` from the isolated workspace; exit 0; bead is in progress and scoped to local binary IPC ingress plus runtime orchestration semantics.
- Globbed `crates/vb_ipc/src/**/*.rs`, `crates/vb_runtime/src/**/*.rs`, `crates/vb_core/src/**/*.rs`, and `crates/*/Cargo.toml` in the isolated workspace.
- Grepped for IPC/sync/evidence/backpressure/timer/shutdown/cancel/queue terms in Rust and Markdown files.

## Relevant crates and dependency boundaries

- `crates/vb_ipc`: binary IPC protocol, Unix socket server, bounded memory ingress, payload/header validation, typed IPC payloads and responses.
- `crates/vb_runtime`: shard-owned synchronous command orchestration, bounded command queue, run admission, cancel/action/timer/shutdown commands, trace and metrics surfaces.
- `crates/vb_core`: shared IDs, compiled workflow digest, runtime policy, capabilities, action payloads, frame/state types used by IPC and runtime.
- `crates/vb_storage`: accepted artifact types are referenced by runtime admission through `AcceptedArtifactStore`; this bead should not broaden storage behavior unless strict admission evidence requires it.
- Dependency files currently relevant: `Cargo.toml`, `crates/vb_ipc/Cargo.toml`, `crates/vb_runtime/Cargo.toml`. Existing deps include `mio`, `crossbeam-channel`, `crossbeam-queue`, `postcard`, `serde`, `loom` dev-dependency for `vb_runtime`. No dependency changes appear necessary for exploration scope.

## Relevant files and APIs

### IPC ingress and protocol

- `crates/vb_ipc/src/lib.rs`
  - Public crate facade still contains full IPC definitions (`IpcCommand`, `IpcFrameHeader`, `IpcFrame`, `BoundedPayload`, `MemoryIngress`, `IpcPayload`, `IpcError`) in addition to split modules.
  - Risk: duplicate definitions remain alongside `bounded.rs`, `ingress.rs`, `frame_types.rs`, etc.; dependency bead `vb-0253.2` explicitly says this modularization/dedupe is incomplete.
- `crates/vb_ipc/src/ingress.rs`
  - `IngressFrame::new(run_id, workflow, payload, max_payload)` enforces `BoundedPayload`.
  - `MemoryIngress::bounded(QueueCapacity)` uses `crossbeam_channel::bounded`.
  - `MemoryIngress::try_submit` maps full/disconnected to `IpcError::Full` / `IpcError::Disconnected` and never blocks.
  - `MemoryIngress::try_recv` returns `Ok(None)` for empty and typed error for disconnected.
- `crates/vb_ipc/src/bounded.rs`
  - `QueueCapacity(NonZeroUsize)`, `MaxPayloadBytes(NonZeroUsize)`, `MaxPayloadBytes::DEFAULT = 1_048_576`, and `BoundedPayload::new` enforce caller-visible payload bounds.
- `crates/vb_ipc/src/frame_types.rs`
  - `IpcFrameHeader::decode` validates magic/version/command/reserved/payload length before payload allocation.
  - `IpcFrame::new` enforces header/payload length agreement and max payload.
- `crates/vb_ipc/src/server/mod.rs`
  - Public `IpcServer`, `IpcResponse`, `WorkflowResolver`, and `WorkflowResolutionError`.
  - `IpcResponse` includes `AcceptedRun`, `PayloadError`, `WorkflowResolutionRequired`, `WorkflowResolutionUnsupported`, `WorkflowDigestMismatch`, `RuntimeError`, metrics, trace, graph, and verification responses.
- `crates/vb_ipc/src/server/impl_.rs`
  - `IpcServer::bind` uses `mio` Unix listener.
  - `poll_once_with_resolver` polls readable/writable events and serially handles clients.
  - `handle_readable` reads into per-client `Vec`, decodes fixed header with `MaxPayloadBytes::DEFAULT`, waits for a full frame, dispatches command, and sends response.
  - Risks: per-client `read_buffer` and `write_buffer` are unbounded `Vec`s except command/payload guards; slow-client/backpressure evidence must prove this cannot grow without bound or must scope a fix.
- `crates/vb_ipc/src/server/dispatch.rs`
  - `serve_ipc` and `serve_ipc_with_resolver` expose one polling turn.
  - `dispatch_command_with_resolver` routes all 16 `IpcCommand` variants to handlers.
- `crates/vb_ipc/src/server/handlers.rs`
  - `handle_submit_run` decodes `IpcPayload` and only accepts `SubmitRun`/`SubmitRunInline` matching frame command.
  - `submit_resolved_workflow` caps submit input at 65,536 bytes, requires a `WorkflowResolver`, rejects digest mismatch, then calls `runtime.submit_compiled` for `SubmitRun` and `runtime.submit_direct` for `SubmitRunInline`.
  - Current behavior proves resolver-backed compiled artifact digest path reaches runtime enqueue, not necessarily strict accepted-artifact admission; runtime policy/admission interaction must be tested.

### Runtime orchestration

- `crates/vb_runtime/src/runtime.rs`
  - `Runtime::submit_compiled` delegates to `submit_direct`; `submit_direct_with_grants` enqueues `ShardCommand::Submit`.
  - `Runtime::cancel_run`, `resume_run`, `answer_ask`, `complete_action_with_output`, `fail_action`, `timer_fired`, and `shutdown_graceful` enqueue typed `ShardCommand`s on the owning shard.
  - `Runtime::tick_all` processes one command per shard tick.
- `crates/vb_runtime/src/shard/types.rs`
  - `ShardCommand` variants cover submit, pre-persisted submit, inputs, contracts, resume, action complete/fail, ask answer, timer fired, cancel, inspect, and shutdown.
  - `ShardCommandQueue` wraps `crossbeam_queue::ArrayQueue<ShardCommand>` with non-blocking `enqueue`, `pop`, `remaining_capacity`, `is_full`; full maps to `RuntimeError::QueueFull`.
  - `ShardConfig::default` uses command queue capacity 1024 and strict runtime policy.
- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
  - `Shard::enqueue` probes journal health for submit variants before queueing.
  - `Shard::tick` pops one command and dispatches deterministically to submit/resume/action/timer/cancel/inspect/shutdown handlers.
- `crates/vb_runtime/src/admission.rs`
  - Defines `REQUIRED_GATE_COUNT = 15`, `AcceptedArtifactStore`, `ArtifactEnvelopeError`, and `AdmissionError`.
  - `AlwaysPresentArtifactStore` is the default in `Shard::new_with_journal`, so strict admission evidence must ensure tests use a real/rejecting accepted artifact store where needed.
- `crates/vb_runtime/src/shard/transitions.rs`, `crates/vb_runtime/src/shard/lifecycle/*.rs`, `crates/vb_runtime/src/shard/timer_wheel.rs`
  - Relevant for cancel-vs-completion, timer ordering, terminal cleanup, and shutdown drain behavior.

### Existing verification/test surfaces

- `crates/vb_runtime/src/models/loom/action_completion_cancel.rs`: loom model for completion/cancel mutual exclusion; currently abstract and not production-connected.
- `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`: loom model around `TimerWheel`; contains weak invariant (`assert!(true)`) and `unwrap` in test-only lock handling.
- `crates/vb_runtime/src/models/loom/shutdown_drain.rs`: abstract pending counter model; not production shard drain evidence.
- `crates/vb_runtime/src/models/loom/bounded_queue.rs`: abstract bounded counter model; not `ShardCommandQueue` or IPC server queue evidence.
- `crates/vb_runtime/src/shard/tests/chunk_026.rs` and `chunk_027.rs`: shutdown drain, queue full, cancel, and timer cleanup tests exist.
- `crates/vb_ipc/src/server/impl_tests.rs`: server tests include workflow resolution, response roundtrips, submit payload cap, and IPC response cases.
- `verification/tla/*` and `verification/verus/*`: global verification assets exist; no bead-specific IPC sync proof artifact was inspected as already complete.

## Current behavior summary

- Binary IPC frames are little-endian, fixed-header, postcard-payload frames with pre-allocation header validation.
- In-process memory ingress and runtime shard command queues are bounded and non-blocking, with typed full/backpressure errors.
- IPC submit requires a resolver and digest match, then dispatches to runtime submission APIs.
- Runtime command processing is synchronous and shard-owned: one queued command is processed per tick; no task-per-step async runtime is present in the mapped hot path.
- Strict accepted-artifact admission exists in runtime types, but IPC `SubmitRun` currently resolves a workflow and calls `submit_compiled`/`submit_direct`; evidence must prove that this path actually reaches strict admission with accepted artifact semantics or identify the missing connection.

## Risks to carry forward

- `risk:duplicate-ipc-definitions`: `vb_ipc/src/lib.rs` still duplicates split module implementations; public API can diverge from modularized code.
- `risk:strict-admission-gap`: IPC submit path may only prove resolver-backed `CompiledWorkflow` enqueue, not accepted-artifact strict admission with 15-gate proof flags.
- `risk:slow-client-buffer-growth`: server client read/write buffers are `Vec`; slow-client/backpressure evidence must prove boundedness or scope a bounded-buffer change.
- `risk:abstract-loom-evidence`: existing loom models are mostly abstract and may not satisfy production-connected evidence requirements.
- `risk:race-ordering`: cancel/completion/timer/shutdown race semantics span runtime state, timers, journal, and IPC handlers; tests must assert exact deterministic outcomes and typed errors.
- `risk:dependency-blockers-open`: key dependencies `vb-0253.1`, `vb-0253.2`, `vb-0253.5`, and `vb-core-ipc-loom-property` remain in progress and may block this bead's closure.

## Candidate scoped verifier commands for later states

- `cargo test -p vb_ipc`
- `cargo test -p vb_runtime`
- `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime action_completion_cancel`
- `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel`
- `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime shutdown_drain`
- `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime bounded_queue`
- `moon ci` as the final canonical gate if code changes occur.
