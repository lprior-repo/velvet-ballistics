# Dependency Policy

`velvet-ballistics` is a performance-critical workflow orchestration engine. This document
defines the criteria and process for adding dependencies to the project.

## License Policy

### Allowed Licenses

We allow dependencies with the following permissive licenses:

| License | SPDX Expression | Notes |
|---------|-----------------|-------|
| MIT | `MIT` | Standard permissive license |
| Apache License 2.0 | `Apache-2.0` | Apache 2.0 with optional patent grants |
| BSD 2-Clause | `BSD-2-Clause` | Simplified 2-clause BSD |
| BSD 3-Clause | `BSD-3-Clause` | Standard 3-clause BSD |
| ISC | `ISC` | Internet Systems Consortium license |
| Zlib | `Zlib` | Liberal zlib license |

All allowed licenses share common characteristics:
- Permissive redistribution requirements
- No copyleft or patent retaliation clauses
- Minimal constraints on derived works

### Banned Licenses

The following licenses are **explicitly prohibited** due to licensing incompatibility:

| License | Reason |
|---------|--------|
| GPL-2.0, GPL-3.0, or any `GPL-*` variant | Strong copyleft — requires derived works to be GPL |
| LGPL-2.0, LGPL-3.0, or any `LGPL-*` variant | Weak copyleft — problematic for dynamic linking |
| AGPL-3.0 or any `AGPL-*` variant | Strong copyleft with network use restrictions |
| SSPL-* | Server-side copyleft — restrictive for service use |
| Commons Clause | Not a true license — restricts commercial use |

### Multi-License Policy

Any crate using **multiple licenses** where one or more is on the banned list is
**rejected**. The `cargo-deny` configuration enforces:

```toml
[bans.multi]
version = 2
retain = []
# Reject any crate with multiple license options if any is banned
```

## Criteria for Adding a Dependency

A dependency may be added if it meets **all** of the following criteria:

### 1. Performance or Safety Necessity

The dependency must solve a problem where:
- **Performance**: No pure-Rust alternative exists with comparable performance
- **Safety**: The functionality requires formal verification or audited unsafe code
- **Complexity**: The cost of maintaining in-house implementation exceeds benefit

Examples of acceptable justifications:
- `fjall`: LSM-tree storage with decades of academic research behind it
- `blake3`: SIMD-optimized hash function that would require inline assembly otherwise
- `postcard`: Zero-copy serialization with proven correctness properties

### 2. No HTTP Egress Requirement

Dependencies must not require:
- Outbound HTTP requests during normal operation
- DNS resolution or network connectivity
- License check-in or telemetry

**Rationale**: `velvet-ballistics` is designed for edge deployment where network
connectivity may be limited or nonexistent.

### 3. No Async in Core Runtime Path

For dependencies in the core execution path:
- Async runtime dependencies (`tokio`, `async-std`, `smol`) are **banned** in core
- Async-capable dependencies are acceptable if:
  - They work in sync mode (no spawned tasks)
  - The async surface is behind a feature flag
  - Only used for non-critical operations (logging, metrics)

**Rationale**: Our execution model is built on explicit frame advancement
and does not use async/await. Adding an async runtime would complicate the
core execution model significantly.

### 4. Auditability

The dependency must be:
- Open source with publicly auditable source code
- Published to crates.io with a verified maintainer
- Not known to have malicious code or supply-chain issues

## Exception Process

### Process for Requesting an Exception

1. **Create an Issue**: File a dependency request issue with:
   - Crate name and version
   - Justification addressing all 4 criteria
   - License information
   - Alternative solutions considered

2. **Security Review**: For any dependency with transitive dependencies
   containing unsafe code, provide:
   - `cargo-geiger` output showing unsafe usage
   - Explanation of why the unsafe usage is safe in context

3. **Approval**: Exceptions require approval from:
   - One maintainer with security domain expertise
   - One maintainer with architecture expertise

4. **Add to `cargo-vet.toml`**: Once approved, add exemption to:
   ```toml
   [[exemptions]]
   name = "crate-name"
   version = "x.y.z"
   kind = "direct"  # or "indirect" or "dev"
   notes = "Approval issue link and justification summary"
   ```

### Pre-Approved Exceptions

The following dependencies have pre-approved exemptions (documented in `cargo-vet.toml`):

| Crate | Version | Reason |
|-------|---------|--------|
| `fjall` | 3.1.4 | Performance-critical LSM-tree storage |
| `saphyr` | 0.0.6 | YAML parsing for workflow definitions |
| `saphyr-parser` | 0.0.6 | YAML parsing for workflow definitions |
| `postcard` | 1.1.1 | Binary serialization for IPC frames |
| `crossbeam-queue` | 0.3.* | Concurrent queue implementations |
| `rtrb` | 0.4.* | Lock-free ring buffer for IPC |
| `arrayvec` | 0.7.* | Stack-allocated arrays for performance |
| `blake3` | 1.8.* | Fast cryptographic hash |
| `bytes` | 1.10.* | Zero-copy byte handling |
| `thiserror` | 2.0.* | Error type derivation |
| `tempfile` | 3.23.* | Temporary files for test fixtures only |

## Review Schedule

Dependencies are reviewed:
- **At addition time**: Full criteria evaluation
- **Quarterly**: Re-evaluation of existing exemptions
- **On security advisory**: Immediate re-evaluation if affected

## Enforcement

This policy is enforced by:

1. **`cargo-deny`**: Checks license compatibility at CI time
2. **`cargo-vet`**: Verifies supply-chain audit status
3. **`cargo-geiger`**: Tracks unsafe code usage
4. **Code review**: Human review of dependency additions

CI will fail if:
- A new dependency has a banned license
- A dependency is not covered by `cargo-vet` exemption
- Unsafe code appears in first-party crates

## Summary

`velvet-ballistics` maintains a minimal, audited dependency surface. Every dependency
exists because it solves a problem that cannot be reasonably solved in-house.
We prefer pure Rust, permissive licensing, and auditable code.
