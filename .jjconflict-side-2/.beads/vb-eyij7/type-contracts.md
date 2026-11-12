# Type and Package Contracts

## Canonical workspace value

`CanonicalWorkspace` is the ordered set in `velvet-ballistics-MASTER.md` §34. Build tooling must derive its expected members from this contract and compare exact membership, not only required-member inclusion. The workspace manifest is the boundary parser; its effective member set is observed through Cargo metadata.

## Package identities

- YAML package: `vb_yaml`, path `crates/vb_yaml`, owns YAML source parsing, profile validation, source maps, and fixture loading required by §25.
- Validation package: `vb_validate`, path `crates/vb_validate`, remains the existing distinct validation owner.
- Expression package: `vb_expr`, path `crates/vb_expr`, owns lexer, parser, type checking, compilation to bytecode, and expression evaluation required by §27.
- Product package: `velvet-ballistics`, path `crates/velvet_ballistics`; Rust library target/module is `velvet_ballistics`, and product binary is `velvet-ballistics`.
- `vb_compile` owns workflow compilation and lowering, and depends on parser/validation crates through declared package interfaces rather than private source ownership.

## Constructor and boundary rules

- Cargo metadata and manifests are the only authority for package identity and dependency membership; directory names alone do not establish an active member.
- External YAML/expression input must enter through the canonical parser APIs. Existing API paths in `vb_compile` may remain as thin compatibility re-exports during migration.
- `fuzz` is a required root member. Its current nested standalone workspace declaration must be reconciled so Cargo has one unambiguous root membership graph; no dual-root interpretation is permitted.
- Current `vb_cli` product sources migrate behind `crates/velvet_ballistics` package ownership while preserving the `velvet-ballistics` executable interface. The `vb_queue_semantics` placeholder is not an active master member; it may not remain an untracked extra root member.

## Invariants

Exact package/member sets and API ownership must be mechanically inspectable. A missing package, duplicate owner, dropped test target, unresolved path dependency, or silently changed CLI behavior is a contract failure.
