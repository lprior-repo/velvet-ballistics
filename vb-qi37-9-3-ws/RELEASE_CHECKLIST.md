# Release Checklist

This document tracks the steps required for each release of `velvet-ballistics`.

## Pre-Release

- [ ] Review and update `CHANGELOG.md` with all changes since last release
- [ ] Ensure version numbers are consistent across all `Cargo.toml` files
- [ ] Verify `workspace.package.version` in root `Cargo.toml` matches intended release version
- [ ] Run `cargo +nightly fmt --all` to ensure consistent formatting
- [ ] Run `cargo +nightly clippy --workspace --tests -- -D warnings` (zero tolerance)
- [ ] Run `cargo +nightly test --workspace` and confirm all tests pass
- [ ] Run `cargo +nightly doc --workspace` and confirm no documentation errors
- [ ] Verify all public API modules have crate-level rustdoc documentation
- [ ] Check that `README.md` is up to date with current features and instructions
- [ ] Verify `velvet-ballistics-MASTER.md` architecture contract is current
- [ ] Run `moon ci` if available for full gate matrix
- [ ] Ensure no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` in source
- [ ] Verify no secrets or credentials are committed

## Documentation

- [ ] Update `CHANGELOG.md` with release date and version
- [ ] Update `README.md` if feature set or build instructions changed
- [ ] Ensure all new public APIs have rustdoc comments
- [ ] Verify architecture diagram in `README.md` matches current implementation
- [ ] Update any version references in documentation

## Testing

- [ ] Full workspace test pass: `cargo +nightly test --workspace`
- [ ] Documentation build: `cargo +nightly doc --workspace`
- [ ] Benchmark build: `cargo +nightly bench --no-run`
- [ ] Hardened build: `cargo +nightly build --profile hardened`
- [ ] Maxperf build: `cargo +nightly build --profile maxperf`
- [ ] Fuzz smoke test if applicable

## beads Workflow

- [ ] Run `bd dolt pull` to sync latest beads state
- [ ] Review `bd ready` for any unblocked work that should be included
- [ ] Close any completed beads with `bd close <id> --reason "Completed"`
- [ ] Run `bd dolt push` to sync beads state

## Git

- [ ] Ensure all changes are committed
- [ ] Tag the release: `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
- [ ] Push tags: `git push origin vX.Y.Z`
- [ ] Verify CI passes on the tag

## Post-Release

- [ ] Create GitHub release with changelog excerpt
- [ ] Publish crates to crates.io if applicable
- [ ] Announce release in relevant channels
- [ ] Update `CHANGELOG.md` "Unreleased" section for next development cycle

## Version Bump Template

When bumping version, update these locations:

1. Root `Cargo.toml`: `workspace.package.version = "X.Y.Z"`
2. All crates use `version.workspace = true` (no individual changes needed)
3. `CHANGELOG.md`: Add new version section
4. Any hardcoded version strings in documentation

## Emergency Release

For hotfixes or security releases:

1. Create hotfix branch from release tag
2. Apply minimal fix
3. Run abbreviated test matrix (test, clippy, doc)
4. Bump patch version
5. Tag and release
6. Merge hotfix back to main
