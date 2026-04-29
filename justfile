set dotenv-load := false

nightly := "nightly-2026-04-28"
strict_clippy := "-D warnings -W clippy::all -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::print_stdout -D clippy::print_stderr"

fmt:
	cargo +{{nightly}} fmt --all --check

lint-src:
	cargo +{{nightly}} clippy --workspace --lib --bins --examples --all-features -- {{strict_clippy}}

check:
	moon run :check

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

bench-build:
	moon run :bench-build

maxperf:
	RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C lto=fat" cargo +{{nightly}} build --workspace --all-features --release

maxperf-native:
	RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C lto=fat -C target-cpu=native" cargo +{{nightly}} build --workspace --all-features --release

quick:
	moon run :quick

ci:
	moon ci
