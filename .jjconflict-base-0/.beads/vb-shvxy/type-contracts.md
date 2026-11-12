# Type Contracts: vb-shvxy Verifier Tooling

## Value Objects

### LaneId

Closed enum: `Kani | Flux | TlaTlc | Proptest | Fuzz | Loom`.

Contract: no stringly free-form lane names may enter the core evidence classifier. External labels parse through a fallible constructor that rejects aliases unless explicitly mapped.

### ToolExecutable

Non-empty executable reference with resolved availability state:

- `OnPath { name, resolved_path }`
- `Script { relative_path, exists, executable_policy }`
- `Jar { path, exists }`
- `Unavailable { requested }`

Contract: command execution is illegal while executable is `Unavailable` or repo-local script/jar `exists=false`.

### CargoPackageSelector

Newtype for package names already present in workspace metadata. `vb_runtime` is valid only if the package exists; requested features must be checked against that package's declared features.

### FeatureSet

Set of package-qualified features. Smart constructor rejects feature names not declared by the selected package unless the proof plan marks them as migration blockers, not runnable evidence.

### CfgSet

Closed cfg markers used by verifier lanes. `loom` is valid only when dependency wiring makes exposed cfg code compile in the selected build target.

### TargetTriple

Closed value for execution target. Fuzz sanitizer lane requires `x86_64-unknown-linux-gnu`; musl plus address sanitizer is invalid.

### ApplicableCount

Refined unsigned integer with two constructors:

- `ZeroApplicable`
- `NonZeroApplicable(NonZeroU32)`

Contract: success evidence for behavior lanes accepts only `NonZeroApplicable`.

### EvidenceClassification

Closed enum:

- `SetupHealth`
- `Inventory`
- `BehaviorProof`
- `BehaviorTest`
- `ModelCheck`
- `FuzzSmoke`
- `Blocker`

Contract: `SetupHealth` and `Inventory` are never obligation-closing classifications.

### VerifierExit

Product type: `{ status_code, applicable_count, classification, blocker }`.

Illegal combinations:

- `status_code=0`, `applicable_count=ZeroApplicable`, `classification=BehaviorProof|BehaviorTest|ModelCheck|FuzzSmoke`.
- `status_code=0`, missing target/harness/model names, non-blocker classification.
- nonzero status with absent blocker code.

## Lane-Specific Contracts

### Kani

- Inventory uses `scripts/kani-list.sh <package> [...]` and emits `.evidence/kani-list/<package>/kani-list.json`.
- Execution evidence is separate unless wrapper pass-through is explicitly adopted.
- Requested `KANI_FEATURES` must match declared package features.
- Hardcoded dummy structures are forbidden downstream by repository GOD RULES; contract seeds require arbitrary/exhaustive generation semantics.

### Flux

- Package smoke command shape is `bash scripts/flux-check-package.sh <package>`.
- Unsupported target selectors are typed command-spec errors, not Flux failures.
- Package pass is setup/refinement smoke unless a named Flux artifact is wired to the crate or checked directly.

### TLA/TLC

- Runner resolves exactly one of: PATH `tlc`, `TLA2TOOLS_JAR`, or approved vendored jar.
- Missing `tools/tla2tools.jar` cannot be accepted as portable command evidence.
- Output truncation that hides invariant/deadlock status is invalid evidence.

### Proptest

- Cargo test filters must be paired with output parsing that detects selected/executed count.
- `running 0 tests` is `ZeroApplicable` and therefore blocker evidence even with exit 0.

### Fuzz

- Target name must be registered in `fuzz/Cargo.toml`/`cargo fuzz list` before execution.
- Sanitizer lane requires explicit GNU target or an approved equivalent.
- Missing target and target/sanitizer mismatch are separate typed blockers.

### Loom

- `RUSTFLAGS="--cfg loom"` is valid only if the library build graph can resolve `loom` wherever cfg-exposed modules import it.
- Integration tests cannot rely on dev-dependencies of a dependency crate being transitively available.
