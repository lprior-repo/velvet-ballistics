---
section: 50
title: "Implementation Phases"
parent: velvet-ballistics-MASTER.md
---

## 50. Implementation Phases

| Phase | Name | Required delivery |
|---|---|---|
| 0 | Contract reset | This master replaces YAML-first scope; archive old material. |
| 1 | Workspace rebaseline | Add SDK-first crates, remove active YAML/codegen/UI crates. |
| 2 | Core artifact types | `ArtifactDigest`, `AcceptedArtifact`, binary envelope, schema versions. |
| 3 | SDK macro parser | `velvet_workflow!` minimal grammar, spans, diagnostics. |
| 4 | Schema derives | `VelvetInput`, `VelvetOutput`, `VelvetData`, bounded types. |
| 5 | Action manifest SDK | `velvet_action!`, `ActionManifest`, schema digests, effect fields. |
| 6 | Action executor SDK | `ActionExecutor`, ticket/completion ABI, bounded workers. |
| 7 | Compiler AST and resolver | Names, actions, slots, accessors, expressions, source maps. |
| 8 | Idempotency verifier | Key AST, retry reachability, action scope checks, certificates. |
| 9 | Policy compiler | Policy profiles, policy digests, warning promotion. |
| 10 | Capability/secret gate | Requirement/grant split, secret handles, admission checks. |
| 11 | Boundedness analyzer | Whole-workflow budget and resource contract. |
| 12 | Numeric IR lowering | Tiny bytecode + side tables, IR structural validation. |
| 13 | Accepted artifact emission | Artifact encode/decode, digest checks, certificate record. |
| 14 | Runtime admission | Install/submit accepted artifacts only by default. |
| 15 | Durable history convergence | Semantic event registry, frame deltas, recoverable arena. |
| 16 | Outbox/inbox actions | Durable action scheduling and completion reconciliation. |
| 17 | Testkit and crash lab | Production-engine simulation, finite crash points. |
| 18 | CLI compiler path | `cargo velvet verify/simulate/artifact/explain/graph`. |
| 19 | Runtime CLI path | install/submit/inspect/events/replay/incident/cancel/answer. |
| 20 | LSP and agent context | Compiler-backed LSP, bounded agent JSON context. |
| 21 | Fuzz/property/compile-fail matrix | Required targets and fixtures pass. |
| 22 | Performance harness | Current-scope interpreter/storage/IPC benchmarks. |
| 23 | YAML deletion | Remove active YAML command/docs/tests/crates. |
| 24 | Release hardening | full gates, evidence bundles, documentation refresh. |

---

