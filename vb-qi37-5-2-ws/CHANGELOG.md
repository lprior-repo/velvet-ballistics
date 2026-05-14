# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Documentation and Release Gates**: Final documentation review, CHANGELOG, and RELEASE_CHECKLIST.
- **Architectural Drift Register**: Phases 43-46 from black-hat review with normative enforcement.
- **Plan Verifier Architecture**: Phases 37-42 covering contract parity, Farley constraints, and functional Rust governance.
- **Recovery Hydration Infrastructure**: Full live-frame replay support for durable recovery (Phase 33).
- **Generated-vs-IR Comparison Benchmarks**: Criterion and IAI Callgrind benchmarks comparing generated Rust to interpreter paths (Phase 34).
- **Trybuild Compile-Fail Tests**: Infrastructure for verifying generated Rust code compiles correctly.
- **13 Missing Expression Operators**: Complete set of expression operations in `vb_core` including string, collection, and math helpers.
- **Diagnostic Code Surfaces**: Stable diagnostic codes for `RuntimeError`, `IpcError`, and `JournalError`.
- **Fjall Safety Tuning**: Performance and safety improvements for LSM-tree storage backend.
- **IndexMap Integration**: Deterministic `IndexMap` replacing `HashMap` for O(1) object field lookups.

### Changed

- **DRY Master Document Rewrite**: Eliminated struct/function duplication in `velvet-ballistics-MASTER.md`.
- **Test Suite Remediation**: Sharpened 302 hollow `is_ok()` / 13 `is_err()` assertions across 16 files.
- **Plan Verifier Fixes**: Critical inconsistencies in sections 63-66 resolved.
- **Expression Evaluation Bugs**: Fixed formatting, storage drop behavior, and README alignment.
- **IPC Alignment**: Medium-severity issues fixed and YAML/compile parsers aligned.
- **Runtime Refactoring**: Extracted shared helpers across runtime and IPC modules.

### Security

- **`#![forbid(unsafe_code)]`**: Enforced across all 10 first-party crates.
- **Adversarial Test Suite**: 700+ BDD-style tests across 9 crates with adversarial inputs.
- **Taint Tracking**: Full secret-taint lattice preventing information leakage through control flow.

## [0.1.0] - 2026-05-02

### Added

- **10-Crate Workspace**: Full workspace implementation with 503 passing tests.
- **YAML Compiler Pipeline**: Strict profile parsing, AST construction, validation, and IR lowering.
- **Expression System**: Lexer, Pratt parser, type checker, and bounded-stack bytecode compiler.
- **Runtime Engine**: Shard-owned in-memory execution with native action dispatch.
- **Fjall Storage**: 9-keyspace append-only journal with blake3+crc32c envelope verification.
- **IPC Transport**: Unix domain socket server/client with bounded queues and binary protocol.
- **Code Generation**: maxperf mode generating native Rust from compiled workflow IR.
- **Master Contract**: 62+ normative sections covering semantics, grammar, security, and lifecycle.

[Unreleased]: https://github.com/priorlewis43/velvet-ballistics/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/priorlewis43/velvet-ballistics/releases/tag/v0.1.0
