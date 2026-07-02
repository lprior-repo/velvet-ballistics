# Hazard Analysis: Global Verifier Tooling

| ID | Hazard | Impact | Contract Control |
|---|---|---|---|
| HAZ-001 | Kani wrapper missing or inventory mistaken for execution | False proof closure | Separate `Inventory` from `BehaviorProof`; availability preflight |
| HAZ-002 | Kani undeclared feature names | Commands fail before harness evidence | `FeatureSet` checks declared package features |
| HAZ-003 | Flux unsupported selectors in wrapper command | Invalid command appears like tool failure | `UnsupportedSelector` pre-execution error |
| HAZ-004 | Hardcoded missing TLA jar or user-local path | Non-portable model checking lane | Portable runner contract using PATH/TLA2TOOLS_JAR/approved jar |
| HAZ-005 | TLC output truncation | Invariant/deadlock evidence hidden | Raw evidence preservation invariant |
| HAZ-006 | Proptest filter runs zero tests and exits 0 | Vacuous green evidence | `ApplicableCount::NonZeroApplicable` required |
| HAZ-007 | cargo-fuzz uses musl with address sanitizer | Lane blocked by environment mismatch | explicit GNU target triple requirement |
| HAZ-008 | Fuzz command names absent target | False blocker or no execution | target registration preflight |
| HAZ-009 | Loom cfg imports unavailable dependency in library build | Integration model does not compile | cfg/dependency parity guard |
| HAZ-010 | Setup/version/list logs close obligations | Proof ledger lies | evidence class separation |
| HAZ-011 | Ambient workspace target changes across agents | Non-reproducible sanitizer behavior | target triple explicit for sensitive lanes |
| HAZ-012 | Prior capped evidence reused as fresh pass evidence | Stale assurance | prior evidence classified as context only |

## Residual Illegal-State Risks

- Until a downstream implementation adds a shared parser/wrapper, `running 0 tests` remains representable in raw cargo output and must be blocked manually by formal-verifier.
- Until Kani feature policy is decided, missing `vb_runtime/kani-artifact-version-barrier` remains a command-spec blocker.
- Until Loom wiring policy is decided, cfg-only integration tests remain likely to fail when library modules import `loom`.
