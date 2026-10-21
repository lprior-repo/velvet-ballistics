The dependency key `vb_cli` must match the import name used in tests. Since all four affected test targets use `use velvet_ballistics::...` and no code uses `vb_cli::`, rename the dependency key to `velvet_ballistics` while keeping the `package` override.

**Corrected line in `crates/workspace_tests/Cargo.toml`:**

```toml
velvet_ballistics = { package = "velvet-ballistics", path = "../velvet_ballistics" }
```

This is the minimal fix: it aligns the Cargo dependency key with the actual import identifier, requires no source changes, introduces no circular dependency, and adds no duplicate aliases.