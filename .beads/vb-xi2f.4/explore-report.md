# Explore Report: vb-xi2f.4

## Scope
Compiler emission path in vb_compile crate.

## Key Files
- crates/vb_compile/src/mod_compile_lowering/part_01.rs (unchecked site)
- crates/vb_compile/src/mod_compile_core.rs (public API)
- crates/vb_compile/Cargo.toml (feature flag issue)
- crates/vb_core/src/workflow/mod.rs (try_from_parts)
- crates/vb_core/src/compiled_workflow.rs (try_from_parts re-export)

## Current State
- compile_source() in part_01.rs uses from_parts_unchecked
- YamlCompiler::compile() and compile_workflow() are unchecked public APIs
- compile/mod.rs path already uses try_from_parts
- test-util feature enabled in production dependency enables from_parts_unchecked

## Risks
- Low scope, single emission site
- Validation infrastructure already exists
- No new dependencies
