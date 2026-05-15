# Contract Specification: vb-qi37.12

## Context
- Feature: eliminate silent discard paths in first-party runtime, storage, and compiler surfaces.
- Bead source: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.12 --json`.
- State 2 inputs: `.beads/vb-qi37.12/codebase-map.md` and `.beads/vb-qi37.12/delivery-scope.jsonl`.
- Acceptance: no first-party runtime/storage/compiler path silently drops fallible outcomes; ignored results are forbidden or justified by explicit typed discard APIs; injected journal/storage/action/recovery failures surface typed errors to callers and evidence.

## Domain Terms
- Fallible outcome: any `Result`, I/O operation, storage operation, decode operation, engine result, or validation result whose failure changes durability, recovery correctness, user-visible diagnostics, or admission safety.
- Silent discard: dropping or erasing a fallible failure by `let _ =`, `.ok()`, wildcard error match, logging-only continuation, defaulting to success/empty state, or mapping to a less informative diagnostic without a typed cause.
- Typed discard API: a deliberately named API for best-effort behavior whose contract says which failures are allowed to be unobservable to callers and which evidence records the decision.
- Evidence chain: persisted or returned diagnostics that retain operation, run id when applicable, record kind when applicable, storage boundary, and causal source.

## Assumptions
- This State 3 artifact does not modify production code, tests, or proof files.
- Existing bead-local proof targets discovered by State 5 are now named exactly in repaired State 3 artifacts. Missing source/test/fuzz wiring lanes are represented as explicit temporary non-approval blockers with owner, limitation, expiry, and compensating evidence; no PASS is claimed for those lanes.
- Existing surfaces named by State 2 are authoritative until implementation agents discover additional production discard sites.

## Preconditions
- PRE-001: Every production fallible operation in the scoped runtime/storage/compiler paths must be classified as `must_propagate`, `must_accumulate`, `typed_optional`, or `typed_best_effort_discard` before implementation changes are accepted.
- PRE-002: A `typed_best_effort_discard` classification is valid only when the operation is non-critical to durability, recovery, caller acknowledgement, or compiler validation, and the API name or error variant documents the discard boundary.
- PRE-003: Runtime, storage, and compiler callers must use fallible signatures (`Result<T, E>` or typed accumulation) whenever a failure can affect acceptance, durability, recovery, or diagnostics.
- PRE-004: Recovery and replay decode accessors must distinguish absent optional payloads from corrupt or undecodable payloads whenever corruption can influence recovery correctness.

## Postconditions
- POST-001: A failed strict journal/storage write returns a typed error to the caller before any success acknowledgement or externally visible success state is emitted.
- POST-002: A failed process lock open/flock/critical metadata write returns `JournalError` or an explicitly documented best-effort discard result; it must not masquerade as full lock metadata success.
- POST-003: Runtime action, ask, wait, retry, cancel, resume, terminal, and engine-drive failures retain causal diagnostics across `RuntimeError`, `ResumeError`, journal evidence, and caller-visible results.
- POST-004: Compiler validation failures are accumulated or propagated as `CompileError`/`CompileErrors`; no validation failure may be converted to success by discard.
- POST-005: Optional accessors that intentionally return `Option` cannot be used by recovery-critical code without a typed decode/corruption path.
- POST-006: Any deliberate best-effort discard is explicit, mechanically inventoryable, and excluded from release-critical durability/recovery/validation paths.

## Invariants
- INV-001: Persistence-before-ack: no acknowledged runtime/storage mutation exists only in memory after a failed required durable write.
- INV-002: Diagnostic preservation: operation, run id where applicable, record kind where applicable, persistence boundary, and causal source are not erased at runtime/storage/compiler boundaries.
- INV-003: Recovery fail-closed: corrupt, truncated, or undecodable persisted data cannot hydrate as an empty successful recovery frame or successful replay summary.
- INV-004: Discard explicitness: all production `let _ =`, `.ok()`, wildcard error matches, and logs-and-continue patterns are either absent or justified by a typed discard API and inventory record.
- INV-005: Compiler validation soundness: validation errors cannot be dropped in a way that admits invalid YAML/IR, unsupported profile events, unresolved references, or invalid schemas.

## Error Taxonomy
- `JournalError::StoragePersistFailed` - required persist/flush/commit boundary failed.
- `JournalError::ProcessLockIo` - process lock file open/read/write/truncate/rewind/flock failed outside documented best-effort metadata reads.
- `JournalError::ProcessLockHeld` - non-blocking exclusive lock is held by another process; holder PID may be absent only through a typed optional metadata path.
- `JournalError::CorruptEventPayload` - persisted event payload or slot payload cannot be decoded on a recovery-critical path.
- `JournalError::ReplayCorruption` - replay or summary reconstruction finds corrupt, missing, or inconsistent journal data.
- `RuntimeError::StorageJournalAppend` - runtime attempted a required journal mutation and storage returned a typed failure.
- `RuntimeError::EngineDriveFailed` - deterministic engine drive failed and the cause must survive terminal failure handling.
- `ResumeError::JournalAppendFailed` - resume/rollback journal append failed with source preserved.
- `CompileErrors` - one or more validation errors accumulated without discarding individual causes.
- `DiscardError::BestEffortSuppressed` - explicit discard classification for non-critical best-effort paths, with operation and rationale.

## Contract Signatures
- `fn classify_fallible_site(site: FallibleSite) -> Result<DiscardClassification, ContractError>`
- `fn close_or_persist_strict(journal: &mut FjallJournal) -> Result<(), JournalError>`
- `fn acquire_process_lock(db_path: &Path) -> Result<ProcessLock, JournalError>`
- `fn decode_recovery_slot_value(event: &JournalEvent) -> Result<Option<SlotValue>, JournalError>`
- `fn apply_drive_result(run: RunId, state: RunState, result: RuntimeEngineResult<RuntimeSignal>) -> RuntimeResult<()>`
- `fn validate_workflow_ast(ast: WorkflowAst) -> Result<ValidatedWorkflow, CompileErrors>`

## Verus-Owned Clauses
- INV-004: classification lattice has no implicit discard state and rejects unclassified production fallible sites.
- INV-002: diagnostic envelope transformations preserve required fields.
- INV-003: recovery decode classification cannot turn corrupt bytes into successful absence.

## TLA+-Owned Clauses
- INV-001 and POST-001: persistence-before-ack state machine for runtime/storage mutation, persist failure, acknowledgement, and recovery visibility.
- INV-003: recovery/replay lifecycle fail-closes on corrupt persisted data instead of producing successful empty state.

## Theorem-Owned Clauses
- None required at State 3. Verus is sufficient for the local classification and diagnostic-preservation kernels once exact proof targets are created.

## State 5 Repaired Proof Evidence
- `TLA-DEADLOCK-011`: bead-local TLA+ config enables deadlock checking; explicit `Stutter` was removed from `Next`; State 5 TLC evidence exits 0 with no deadlock error.
- `SCAN-DISCARD-006`: `silent-discard-scan-report.md` classifies every scoped candidate from the complete raw scan and records zero unclassified release-critical silent discards.
- `FUZZ-DECODE-009`: fuzz target `vb_qi37_12_persisted_payload_decode` is wired and State 5 evidence records a 1000-run cargo-fuzz execution after local environment repair.
- Remaining release gate: canonical `moon ci` is still owned by State 11 formal-verifier/release gate.

## Non-goals
- No production implementation, tests, proof code, benchmark claims, UI, generated Rust/codegen, or source-checkout writes in this state.
