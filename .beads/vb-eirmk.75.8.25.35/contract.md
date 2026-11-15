# Compiled node and constant hard caps

MASTER §13 sets hard compile-time limits of 1,000 compiled steps and 8,192 constants. `MAX_STEPS_PER_WORKFLOW` and `MAX_CONSTANTS` are the canonical bounds used by compiler and checked workflow validation. Exact-limit workflows are accepted; one-over workflows are rejected through existing typed error surfaces.

Checked `CompiledWorkflow` constructors reject declared `ResourceContract.max_steps` or `max_constants` above their hard cap with `WorkflowError::ResourceContractTooLarge` naming `Steps` or `Constants`. When the declared value is within the cap but actual node or constant usage exceeds it, constructors retain `WorkflowError::ResourceContractExceeded` naming the actual dimension. `ResourceContract::DEFAULT` uses the hard step and constant caps while preserving unrelated defaults. The execution `BoundednessPolicy.max_total_steps` remains 1,000,000 and independent from the number of compiled nodes.

The compiler accepts source workflows at the compiled-node and constant-pool boundaries and rejects one-over fixtures with existing typed compiler errors carrying the actual hard limit. Source-slot reservation remains bounded coherently with the node hard cap. No arithmetic wraps or truncates at either boundary.

Previously encoded `WorkflowParts` with over-limit declared contracts remain byte-for-byte decodable. Checked admission rejects them using the existing `ResourceContractTooLarge` variants; no migration, permissive fallback, legacy admission path, schema, public API, dependency, or persisted format is introduced.

Tests exercise the public compiler and checked constructors at both sides of each boundary, including declared-over-cap and actual-over-declared errors. Other §13 bounds, runtime arena caps, storage codecs, historical records, and workflow-execution accounting remain unchanged.
