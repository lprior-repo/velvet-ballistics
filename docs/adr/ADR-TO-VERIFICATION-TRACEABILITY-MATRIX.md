# ADR to Verification Traceability Matrix

This matrix maps each ADR to master sections, supporting docs, evidence hooks, and known gaps.

| ADR | Title | Master sections | Supporting docs | Evidence hooks | Known gaps |
|-----|-------|-----------------|-----------------|----------------|------------|
| 001 | Backend IR North Star | 0, 22, 44, 68 | `docs/runtime-architecture.md` | `moon ci`, release evidence bundle | Existing docs still have stale scope language |
| 002 | Naming Workspace | 1, 23, 34 | `AGENTS.md` | spelling scan, workspace member check | Existing docs contain old product spelling |
| 003 | Rust Governance | 2, 3, 4, 7, 52, 53 | `docs/rust-governance.md` | fmt, clippy, no-panic scans, feature gates | PGO current-gate language needs cleanup |
| 004 | YAML Boundary | 8, 9, 10, 25, 26 | `docs/language-spec.md` | parser, validator, YAML fuzz | language doc says draft and has stale UI language |
| 005 | Errors Diagnostics | 16, 17, 50, 60, 75 | `docs/error-variant-completeness.md` | diagnostic parity tests | Needs generated diagnostics matrix refresh |
| 006 | Accepted Artifact IR | 14, 15, 51, 63 | `docs/compiled-ir.md` | IR validation, digest, artifact tests | compiled-ir doc has stale future list |
| 007 | Slot Values Arena | 11, 14, 48 | `docs/slot-value-model.md` | value store tests, handle tests | No GC remains an operational risk |
| 008 | Expression Engine | 27, 38, 46 | `docs/expression-engine.md` | parser, bytecode, helper parity tests | F64/typechecker/helper parity gaps remain |
| 009 | Taint Secrets | 47, 66 | `docs/slot-value-model.md` | taint propagation, secret rejection tests | No control-flow taint in v1 by decision |
| 010 | Whole Workflow Bounds | 13, 56, 64, 67 | `docs/performance-contract.md` | budget rejection tests, proptest | Conservative analysis must be kept visible |
| 011 | Node Semantics Runtime | 20, 45, 55, 62 | `docs/in-memory-runtime.md` | per-node tests, scheduler tests | All 34 node variants need evidence mapping |
| 012 | Actions Idempotency | 19, 47, 65, 66 | `docs/runtime-architecture.md` | action contract, retry, idempotency tests | External behavior is attestation only |
| 013 | Fjall Journal | 18, 49, 61 | `docs/fjall-storage.md`, `docs/storage-journal.md` | storage tests, duplicate key tests | Cross-keyspace batch parity evidence required |
| 014 | Recovery | 18, 49, 61, 67, 68 | `docs/storage-journal.md` | crash/recovery tests, replay tests | Pending-action recovery evidence remains risky |
| 015 | Binary IPC | 21, 50 | `docs/binary-ipc.md`, `docs/ipc-memory-boundary.md` | frame, queue full, payload limit tests | Unix socket server evidence required |
| 016 | Runtime Admission | 63, 66 | `docs/runtime-architecture.md` | artifact digest, capability, secret-presence tests | Raw submit tests can launder evidence |
| 017 | CLI Control Plane | 33, 69, 75 | `docs/language-spec.md` | CLI smoke, structured output checks | `velvet` legacy examples need migration labels |
| 018 | Evidence Gates | 36, 37, 38, 39, 40, 43, 60, 77 | `formal-verification-report.md`, `test-plan.md` | Moon, Kani, Verus, Flux, Miri, fuzz evidence | Evidence freshness and scope must be audited |
| 019 | Performance Evidence | 6, 39, 71, 77.13, 77.14 | `docs/performance-contract.md`, `docs/benchmark-suite.md` | Criterion, iai-callgrind, hyperfine, allocation traces | Placeholder benchmarks are not evidence |
| 020 | Drift Register | 67 | `arch-drift-reports/` | drift reports, follow-up beads | Needs automated doc drift scan |
| 021 | Deferred Scope | 22, 32, 41, 76-83 | `docs/deferred-codegen-maxperf.md`, `docs/deferred-ui.md` | scope scans, dependency checks | Existing docs still mention deferred surfaces |
| 022 | ADR Governance | 43, 60, 77 | this directory | review gate commands | New ADRs can drift without required updates |
| 023 | Single Server Ownership | 18, 54, 58, 61, 68 | `docs/fjall-storage.md` | DB lock tests, durability profile tests | No distributed HA in v1 |
| 024 | Hot Cold Boundaries | 11, 12, 53, 62 | `docs/runtime-architecture.md` | banned API scan, no-async scan | Existing docs use stale crate names |

## Evidence Rule

Architecture docs can define what must be proven. They do not prove it. A claim is closed only when raw command evidence maps the requirement to production source and executable tests or proof artifacts.
