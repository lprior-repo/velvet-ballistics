# Error Taxonomy: Verifier Tooling Contracts

## Command Specification Errors

- `UnknownLane { lane }`: lane label does not parse to closed `LaneId`.
- `UnsupportedSelector { lane, selector }`: command asks wrapper to accept forbidden target selector.
- `MissingPackage { package }`: selected Cargo package is absent.
- `UndeclaredFeature { package, feature }`: requested feature is not declared.
- `InvalidCfgWiring { cfg, reason }`: cfg-exposed code imports dependencies unavailable to that build.

## Availability Errors

- `MissingScript { path }`: required repo script absent.
- `MissingExecutable { name }`: tool unavailable on PATH.
- `MissingTlaRunner`: neither approved PATH `tlc`, `TLA2TOOLS_JAR`, nor approved jar is available.
- `MissingJar { path }`: command hardcodes a jar path that does not exist.

## Execution Environment Errors

- `IncompatibleTargetSanitizer { target, sanitizer }`: selected target cannot support requested sanitizer lane.
- `MissingFuzzTarget { target }`: target not registered by cargo-fuzz.
- `WorkspaceTargetAmbiguous`: proof command relies on ambient target when lane requires explicit target.

## Evidence Classification Errors

- `ZeroApplicableTests { command }`: command exited successfully but selected/executed zero tests/harnesses/models.
- `InventoryOnlyEvidence { command }`: output lists harnesses/versions but does not execute behavior.
- `TruncatedEvidence { path }`: output omits required status/count/error lines.
- `UnclassifiedEvidence { command }`: output cannot be mapped to closed classification.

## External Tool Failures

- `VerifierNonzeroExit { lane, status }`: command failed after passing preflight.
- `ToolOutputParseFailed { lane, reason }`: parser cannot extract count/status safely.

## Railway Policy

Every lane returns `Result<AcceptedEvidence, VerifierBlocker>`. There is no boolean pass/fail API. Any unknown or unparsable state returns a blocker.
