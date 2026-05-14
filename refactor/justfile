set dotenv-load := false

nightly := "nightly-2026-04-28"
strict_clippy := "-D warnings -W clippy::all -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::print_stdout -D clippy::print_stderr"
allowed_features := "try_blocks,portable_simd,allocator_api,generic_const_exprs"
pgo_profile_dir := "target/pgo/profiles"
pgo_merged_profile := "target/pgo/merged.profdata"
pgo_binary := "./target/maxperf/velvet-ballistics"
pgo_build_args := "-p velvet_ballastics --bin velvet-ballastics --all-features --profile maxperf"

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

pgo:
	just pgo-instrument-build
	just pgo-run-workload
	just pgo-merge-profiles
	just pgo-optimized-build

pgo-instrument-build:
	rm -rf {{pgo_profile_dir}} {{pgo_merged_profile}}
	mkdir -p {{pgo_profile_dir}}
	RUSTFLAGS="-Cprofile-generate=$PWD/{{pgo_profile_dir}}" cargo +{{nightly}} build {{pgo_build_args}}

pgo-run-workload:
	test -x {{pgo_binary}}
	{{pgo_binary}} bench-run tests/fixtures/pgo/minimal_save.yaml --json
	{{pgo_binary}} bench-run tests/fixtures/pgo/choose_true.yaml --json

pgo-merge-profiles:
	test -n "$(find {{pgo_profile_dir}} -type f -name '*.profraw' -print -quit)" || { printf '%s\n' "no PGO profile data found under {{pgo_profile_dir}}; run 'just pgo-run-workload' after 'just pgo-instrument-build'"; exit 1; }
	rustup run {{nightly}} llvm-profdata merge -o {{pgo_merged_profile}} {{pgo_profile_dir}}

pgo-optimized-build:
	test -s {{pgo_merged_profile}} || { printf '%s\n' "missing merged PGO profile at {{pgo_merged_profile}}; run 'just pgo' or 'just pgo-merge-profiles' first"; exit 1; }
	RUSTFLAGS="-Cprofile-use=$PWD/{{pgo_merged_profile}} -Cllvm-args=-pgo-warn-missing-function" cargo +{{nightly}} build {{pgo_build_args}}

maxperf-release:
	cargo +{{nightly}} build --workspace --all-features --profile maxperf

maxperf:
	RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C lto=fat" cargo +{{nightly}} build --workspace --all-features --release

maxperf-native:
	RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C lto=fat -C target-cpu=native" cargo +{{nightly}} build --workspace --all-features --release

quick:
	moon run :quick

ci:
	moon ci
