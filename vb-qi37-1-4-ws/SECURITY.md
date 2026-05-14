# Security Policy

## Dependency Pinning

### saphyr YAML Parser

The `saphyr` crate family is pinned to exact versions in `Cargo.toml`:

- `saphyr = "=0.0.6"`
- `saphyr-parser = "=0.0.6"`
- `serde-saphyr = "=0.0.25"`

This prevents automatic patch updates that could introduce API breakage or supply-chain risks.

**Tradeoff:** saphyr is pure Rust (no `unsafe`, no C dependencies), which eliminates memory-safety risks from FFI boundaries. However, it is at 0.0.x semver, meaning patch releases may change APIs. Pinning exact versions protects against unexpected breakage, but manual review is required before any saphyr upgrade.

**Monitoring:** Run `cargo audit` regularly (already integrated in CI supply-chain gates) to check for CVEs in pinned dependencies.
