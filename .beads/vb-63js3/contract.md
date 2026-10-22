# Bead vb-63js3: Flux Lane in xtask Proof Runner

## Scope

Route the existing `flux` lane in `xtask/src/lanes.rs` through the existing Flux wrapper script.
The production routing changes; proof closure and performance improvements are outside this scope.

## Implementation

- `lane_command("flux", crate_name, workspace_root)` returns exactly three argv elements:
  - `bash`
  - `<workspace_root>/scripts/flux-check-package.sh`
  - `<package>` (the crate name passed through verbatim)
- All other lanes, error handling, and the existing API surface are unchanged.
- No `execute_lane` function is added or modified; `lane_command` returns `Vec<String>`.

## Wrapper Behavior

The wrapper script `scripts/flux-check-package.sh` (unchanged by this bead) internally
runs: `cargo flux -p <package> --message-format human`. The wrapper adds the
`--message-format human` argument; the xtask command itself passes exactly three argv
to bash. The effective command format differs from the xtask argv list due to this
wrapper routing.

## Path Handling

- `workspace_root` is joined with `scripts/flux-check-package.sh` using `std::path::Path::join`.
- The workspace root is NOT made absolute; relative roots produce relative command paths.
- `script_path.display().to_string()` is used for the script path. This is lossy for
  non-UTF-8 paths; the implementation explicitly makes no guarantee that arbitrary
  non-UTF-8 workspace paths will execute correctly.

## Tests

Seven tests total in `xtask/src/lanes.rs` (three pre-existing, four new):

1. `test_lane_command_test` - cargo test lane (pre-existing)
2. `test_lane_command_clippy` - cargo clippy lane (pre-existing)
3. `test_required_lanes_always_available` - required lanes filter (pre-existing)
4. `test_flux_command_uses_wrapper_script` - flux lane uses bash wrapper, exact Vec equality
5. `test_flux_command_workspace_root_with_spaces` - workspace root with spaces preserved
6. `test_flux_command_workspace_root_with_metacharacters` - shell metacharacters in workspace root
7. `test_flux_command_package_metacharacters_retained_as_one_argv` - package name with metacharacters
   (e.g., `vb_core;$(printf nope)`) retained as a single argv element, exact Vec equality

No actual command execution in tests; all verify command construction only.
No unchecked indexing; all tests use `assert_eq!` with complete expected vectors.

## Verification

Root runs: `cargo test --manifest-path xtask/Cargo.toml --lib lanes::tests --locked`

## Explicit Non-Goals

- No proof closure (no Verus/Flux/Kani/TLA+ proof obligations closed)
- No performance benchmarks
- No bulk Moon lane execution or restoration
- No changes to the wrapper script
- No absolute path guarantees
- No non-UTF-8 path execution guarantees
