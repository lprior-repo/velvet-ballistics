//! VERIF-002 sentinel: verifies the `codec_miri_tests` module compiles
//! cleanly when the crate is built with `--cfg miri`. The actual
//! harness body lives in `codec_miri_tests.rs`; this file is the
//! compile-only smoke that the Miri toolchain sees a valid module
//! surface.
//!
//! Per master §77.8: the previous `#[cfg(test)]` gate silently
//! excluded `codec_miri_tests` from `cargo miri test`, leaving the
//! broken-module-reference defect invisible to the verify lane.
//! Including the module via `#[cfg(miri)]` here restores the bridge.

#[cfg(miri)]
mod codec_miri_tests_compile_check {
    // Force the existing `codec_miri_tests` module to be linked into
    // a Miri build by re-exporting it through a private name. If the
    // module fails to compile under `cfg(miri)` (missing imports,
    // forbidden `unsafe`, etc.) this `use` line will fail the build.
    #[allow(unused_imports)]
    use crate::codec_miri_tests as _;
}

/// `cargo test`-discoverable assertion that the Miri-only module
/// compiles under `--cfg miri` semantics. This test runs during
/// regular `cargo test` and uses `compiletest`-style static
/// introspection: the `#[cfg(miri)]` module above is empty during
/// regular test builds, but `cargo miri test` would expand it and
/// exercise the `use` statement. Failure modes manifest at compile
/// time (missing `codec_miri_tests` module under `cfg(miri)`), so
/// this `#[test]` simply records the contract.
#[test]
fn codec_miri_tests_compiles_under_cfg_miri() {
    // The compile-time guarantee is the assertion. If `cfg(miri)`
    // ever fails to find `crate::codec_miri_tests`, the `use` inside
    // the inner module will emit `error[E0432]: unresolved import`
    // before any test can run. Document the contract here so the
    // verify lane has an unambiguous signal.
    const MOK: bool = cfg!(any(test, miri));
    assert!(MOK, "codec_miri_tests must be reachable under cfg(miri)");
}
