set dotenv-load := false

nightly := "nightly-2026-04-28"
strict_clippy := "-D warnings -W clippy::all -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::print_stdout -D clippy::print_stderr"
allowed_features := "try_blocks,portable_simd,allocator_api,generic_const_exprs"

fmt:
	cargo +{{nightly}} fmt --all --check

lint-src:
	cargo +{{nightly}} clippy --workspace --lib --bins --examples --all-features -- {{strict_clippy}}

check:
	moon run :check

nightly-feature-gate:
	bash scripts/check-nightly-features.sh

nightly-feature-cargo-probe:
	cargo +{{nightly}} -Zallow-features={{allowed_features}} check --workspace --all-targets --all-features

hardened-build:
	cargo +{{nightly}} build --workspace --all-features --profile hardened

test:
	moon run :test

doc:
	moon run :doc

supply-chain:
	moon run :supply-chain

feature-powerset:
	moon run :feature-powerset

miri:
	moon run :miri

coverage:
	moon run :coverage

mutants-smoke:
	moon run :mutants-smoke

fuzz-smoke:
	moon run :fuzz-smoke

sanitizer-address-check:
	RUSTFLAGS="-Zsanitizer=address" cargo +{{nightly}} test -Zbuild-std --target x86_64-unknown-linux-gnu --workspace --all-features --no-run

bench-build:
	moon run :bench-build

source-length:
	bash scripts/check-source-length.sh

benchmark-proof:
	cargo +{{nightly}} bench --workspace --all-features -- --save-baseline vb-current

pgo-instrument-build:
	RUSTFLAGS="-Cprofile-generate=target/pgo/profiles" cargo +{{nightly}} build --workspace --all-features --profile maxperf

pgo-optimized-build:
	RUSTFLAGS="-Cprofile-use=target/pgo/merged.profdata" cargo +{{nightly}} build --workspace --all-features --profile maxperf

maxperf:
	RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C lto=fat" cargo +{{nightly}} build --workspace --all-features --release

maxperf-native:
	RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C lto=fat -C target-cpu=native" cargo +{{nightly}} build --workspace --all-features --release

quick:
	moon run :quick

ci:
	moon ci
