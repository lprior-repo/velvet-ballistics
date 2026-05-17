# Lean Contract Projection: vb-kkvb

## Boundary
- Lean-owned kernel: deterministic command registry, normalized public command names, route selection, placeholder status schema, and status classification.
- Rust/runtime shell: Clap integration, argv collection, stdout/stderr writing, filesystem reads, workspace manifest parsing, process exit status, and any future tool execution.
- External systems excluded from Lean proof: terminal/TTY behavior, OS argv encoding, filesystem, Moon, Cargo, network, wall-clock time, and subprocesses.

## Model Types
- `CommandName`: abstract normalized kebab-case string atom.
- `CommandFamily`: closed enumeration containing the 20 required command families.
- `CommandRegistry`: finite list of `(CommandName, CommandFamily)` entries.
- `RouteResult`: `known CommandFamily` or `unknown CommandName`.
- `StructuredStatusModel`: record with `command`, `status`, `message`, and `next_steps` fields.
- `StatusKind`: `ok`, `deferred`, or `error`.

## Lean-Owned Clauses
- POST-001 -> `VbKkvb.CommandRegistry.help_lists_required_families_exactly_once`
- POST-002 -> `VbKkvb.CommandRegistry.route_known_family_to_distinct_variant`
- POST-003 -> `VbKkvb.StructuredStatus.status_has_required_fields`
- POST-004 -> `VbKkvb.StructuredStatus.placeholder_is_deferred_not_ok`
- INV-001 -> `VbKkvb.CommandRegistry.routing_is_deterministic`
- INV-002 -> `VbKkvb.CommandRegistry.registry_names_unique`
- INV-003 -> `VbKkvb.CommandRegistry.required_names_are_stable_kebab_case`
- INV-004 -> `VbKkvb.CommandRegistry.route_result_is_closed_variant`
- INV-006 -> `VbKkvb.StructuredStatus.status_schema_stable_across_families`

## Theorem Obligations

### THM-POST-001
- Contract clause: POST-001
- Rust/spec target: `xtask::required_command_families` and CLI help route model
- Lean module: `VbKkvb.CommandRegistry`
- Theorem shape: `help_lists_required_families_exactly_once`
- Model: finite required-family list and generated help-name list
- Refinement: Rust `CommandFamilySpec` list extracts to the Lean registry model; help output names are compared as normalized public names.
- Shell exclusions: Clap rendering details, terminal width, stdout I/O.
- Evidence command: `moon run :verify-proof`

### THM-POST-002
- Contract clause: POST-002
- Rust/spec target: `xtask::parse_xtask_command`
- Lean module: `VbKkvb.CommandRegistry`
- Theorem shape: `route_known_family_to_distinct_variant`
- Model: command-name lookup over a unique finite registry
- Refinement: Rust enum variants correspond one-to-one with Lean `CommandFamily` constructors.
- Shell exclusions: OS argv encoding and Clap parser internals.
- Evidence command: `moon run :verify-proof`

### THM-POST-003
- Contract clause: POST-003
- Rust/spec target: `xtask::StructuredStatus`
- Lean module: `VbKkvb.StructuredStatus`
- Theorem shape: `status_has_required_fields`
- Model: abstract status record with required field set
- Refinement: Rust `StructuredStatus` serializable/renderable model maps fields to Lean record projections.
- Shell exclusions: concrete JSON/key-value renderer and stdout writes.
- Evidence command: `moon run :verify-proof`

### THM-POST-004
- Contract clause: POST-004
- Rust/spec target: `xtask::placeholder_status`
- Lean module: `VbKkvb.StructuredStatus`
- Theorem shape: `placeholder_is_deferred_not_ok`
- Model: placeholder command and deferred reason producing a `StatusKind.deferred`
- Refinement: Rust placeholder status kind maps exactly to Lean `deferred` and cannot refine to `ok`.
- Shell exclusions: wording of human-readable message.
- Evidence command: `moon run :verify-proof`

### THM-INV-001
- Contract clause: INV-001
- Rust/spec target: `xtask::parse_xtask_command`
- Lean module: `VbKkvb.CommandRegistry`
- Theorem shape: `routing_is_deterministic`
- Model: pure lookup function from normalized argv head to route result
- Refinement: Rust parse model normalizes the same command token before registry lookup.
- Shell exclusions: argv collection, invalid Unicode representation details.
- Evidence command: `moon run :verify-proof`

### THM-INV-002
- Contract clause: INV-002
- Rust/spec target: `xtask::validate_command_registry`
- Lean module: `VbKkvb.CommandRegistry`
- Theorem shape: `registry_names_unique`
- Model: finite list with no duplicate `CommandName`
- Refinement: Rust validation rejects duplicate public names before constructing `ValidatedCommandRegistry`.
- Shell exclusions: allocation and slice iteration mechanics.
- Evidence command: `moon run :verify-proof`

### THM-INV-003
- Contract clause: INV-003
- Rust/spec target: `xtask::required_command_families`
- Lean module: `VbKkvb.CommandRegistry`
- Theorem shape: `required_names_are_stable_kebab_case`
- Model: exact set of required command-name atoms
- Refinement: Rust static specs match the Lean exact set.
- Shell exclusions: help formatting.
- Evidence command: `moon run :verify-proof`

### THM-INV-004
- Contract clause: INV-004
- Rust/spec target: `xtask::XtaskCommand`
- Lean module: `VbKkvb.CommandRegistry`
- Theorem shape: `route_result_is_closed_variant`
- Model: route result returns only a closed `CommandFamily` constructor or unknown error
- Refinement: Rust enum variants are total over the required families and do not dispatch to arbitrary string handlers after validation.
- Shell exclusions: handler side effects.
- Evidence command: `moon run :verify-proof`

### THM-INV-006
- Contract clause: INV-006
- Rust/spec target: `xtask::StructuredStatus` and `xtask::placeholder_status`
- Lean module: `VbKkvb.StructuredStatus`
- Theorem shape: `status_schema_stable_across_families`
- Model: closed `CommandFamily` enumeration and `StructuredStatusModel` record with required field set
- Refinement: every Rust status constructor for success or deferred command families maps to the same Lean record field set.
- Shell exclusions: concrete renderer, field ordering, JSON/key-value syntax, stdout writes.
- Evidence command: `moon run :verify-proof`

## Waivers
- LEAN-WAIVER-001:
  - Contract clauses: PRE-001, PRE-002, PRE-003, POST-005, POST-006, ERR-001, ERR-002, ERR-003
  - Verification layer waived: Lean
  - Owner: contract agent for State 1; downstream owner is vb-kkvb implementer
  - Reason: hostile CLI argv parsing, Clap classification, process exit status, and diagnostics are Rust shell behavior rather than pure deterministic kernel proof targets.
  - Compensating evidence: Bolero/cargo-fuzz hostile argv obligation, proptest, Kani where bounded route lookup applies, integration CLI scenarios, mutation, and manual QA.
  - Expiry/follow-up: expires if a first-party pure parser replaces Clap or if command parsing is factored into a pure kernel; then add Lean parser grammar obligations before implementation proceeds.
- LEAN-WAIVER-002:
  - Contract clauses: PRE-004, POST-008, INV-008, ERR-005
  - Verification layer waived: Lean
  - Owner: contract agent for State 1; downstream owner is vb-kkvb implementer
  - Reason: workspace manifest dependency analysis is filesystem/Cargo metadata shell behavior, not a Lean kernel.
  - Compensating evidence: static dependency scan, `cargo tree`, `cargo deny`, release-provenance evidence, and gauntlet-standard/all lanes.
  - Expiry/follow-up: expires if dependency-boundary checking is represented as a pure manifest graph model; then add Lean graph invariant obligations.
- LEAN-WAIVER-003:
  - Contract clauses: PRE-005, INV-005, INV-007, INV-009, ERR-004, POST-007
  - Verification layer waived: Lean
  - Owner: contract agent for State 1; downstream owner is vb-kkvb implementer
  - Reason: ownership discipline, forbidden Rust constructs, non-interactive shell behavior, unsafe scanning, renderer I/O failure, and legacy command behavior are Rust/runtime-shell or toolchain properties.
  - Compensating evidence: Rust type checking, Miri/cargo-careful, clippy/static scans, cargo-geiger, regression tests, coverage, and manual QA.
  - Expiry/follow-up: expires if any listed behavior is refactored into a pure deterministic kernel with a stable model.
