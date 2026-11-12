# Workflow Model: Verifier Lane Readiness

## State Machine

1. **Discovered**: lane appears in delivery scope or proof context.
2. **CommandSpecified**: command shape has lane, tool, package/target, feature/cfg, and expected evidence class.
3. **AvailabilityChecked**: executable/script/jar/tool is resolved.
4. **WiringChecked**: package features, cfg dependencies, target triples, fuzz target registration, and runner policy are validated.
5. **Executed**: command ran and raw output was captured.
6. **Classified**: output mapped to evidence classification and applicable count.
7. **AcceptedEvidence**: non-vacuous behavior evidence accepted for downstream proof planning.
8. **Blocked**: typed blocker captured; no pass evidence emitted.

## Legal Transitions

- `Discovered -> CommandSpecified`: only with a closed `LaneId` and expected classification.
- `CommandSpecified -> AvailabilityChecked`: tool/script/jar must be resolvable or transition to `Blocked`.
- `AvailabilityChecked -> WiringChecked`: only if availability is present.
- `WiringChecked -> Executed`: only if feature/cfg/target/target-name constraints pass.
- `Executed -> Classified`: raw output path and exit status captured.
- `Classified -> AcceptedEvidence`: only when classification is obligation-closing and applicable count is nonzero.
- `Classified -> Blocked`: zero applicable, incompatible target, missing tool, unsupported selector, unresolved dependency, or nonzero command failure.

## Lane Guards

- **Kani guard**: package exists; requested features declared; inventory evidence not confused with harness execution.
- **Flux guard**: wrapper invoked with package only; unsupported selectors rejected before execution.
- **TLA guard**: TLC runner policy resolves; no missing hardcoded jar; full invariant/deadlock status captured.
- **Proptest guard**: cargo output parser rejects zero tests selected/executed.
- **Fuzz guard**: target registered; sanitizer-compatible target triple explicit.
- **Loom guard**: cfg `loom` compiles with dependency available to the library/integration-test build graph.

## Terminal Outcomes

- `AcceptedEvidence`: downstream proof-planner may consume as a proof seed input, not proof completion.
- `Blocked`: downstream implementer must repair tooling/config/command generation before formal-verifier can close the lane.

## Temporal Hazards

- A lane can regress from runnable to blocked after feature cleanup; contracts require rechecking before every evidence run.
- PATH-resolved TLC can differ across agents; runner provenance must be captured with evidence.
- Cargo test filters can become stale as test names change; zero-test detection must run after every filtered command.
- Fuzz target registration can drift from proof commands; target list must be checked before fuzz run.
