# Domain Model Review: vb-qi37.4

STATUS: CONTRACT-READY-WITH-BLOCKERS

## Model Summary
- The domain has three distinct records that must not be conflated: accepted artifact envelope, run header, and run admission event.
- The admission decision is a fail-closed boundary before live run state may become externally successful.
- The storage layer owns durable artifact/header/event records; the runtime layer owns semantic admission decisions and typed diagnostic mapping.

## Strengths
- `AcceptedArtifact`, `VerificationProof`, `RunAdmission`, `JournalEvent::RunAdmission`, and `RunHeaderRecord` are named domain concepts.
- Runtime admission has semantic error variants for missing artifacts, invalid envelopes/proofs, capacity, and capability denial.
- Strict storage path exposes `append_strict` and `persist_strict` using `fjall::PersistMode::SyncAll`.

## Domain Risks
- DR-001: Gate-count schema disagreement can make storage-generated artifacts fail runtime strict admission or encourage dummy proof data.
- DR-002: If strict runtime construction accepts `AlwaysPresentArtifactStore`, the accepted-artifact domain invariant collapses to existence-by-test-dummy.
- DR-003: Header persistence and admission event persistence must be one external success boundary; split writes can create crash windows unless made atomic or fail-closed.
- DR-004: Mapping envelope decode errors to a zero digest diagnostic risks hiding the requested digest unless diagnostics preserve contextual digest elsewhere.

## Required Follow-Up For Implementers
- Reconcile v1 gate-count source of truth before final proof/test execution.
- Ensure strict/journaled production paths require `StorageArtifactStore` or equivalent storage-backed accepted artifact loader.
- Prove failure before any header/admission durable boundary prevents ack and leaves no runnable state.
- Preserve exact diagnostic code and digest/capability context through API/CLI/IPC envelopes.
