#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-fast}"
NIGHTLY="${RUST_NIGHTLY:-nightly-2026-04-28}"

case "$MODE" in
  fast|standard|deep|proof|all) ;;
  *)
    printf 'usage: %s {fast|standard|deep|proof|all}\n' "$0" >&2
    exit 2
    ;;
esac

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

SEARCH_PATHS=(Cargo.toml crates fuzz scripts)
for optional_path in tests benches examples proofs; do
  if [ -e "$optional_path" ]; then
    SEARCH_PATHS+=("$optional_path")
  fi
done

log() {
  printf '\n[verify:%s] %s\n' "$MODE" "$*"
}

run() {
  log "$*"
  "$@"
}

run_shell() {
  log "$*"
  bash -lc "$*"
}

moon_task() {
  run moon run ":$1"
}

source_has() {
  rg -q "$1" "${SEARCH_PATHS[@]}" 2>/dev/null
}

cargo_subcommand_available() {
  rustup run "$NIGHTLY" cargo "$1" --help >/dev/null 2>&1
}

verify_fast() {
  moon_task fmt
  moon_task lint-src
  moon_task check
}

verify_standard() {
  verify_fast
  moon_task test
  moon_task doc-test
}

verify_cargo_careful() {
  if cargo_subcommand_available careful; then
    run rustup run "$NIGHTLY" cargo careful test --workspace --all-features
    return
  fi

  if [ "${CAREFUL_REQUIRED:-0}" = "1" ] || source_has '(^|[^A-Za-z0-9_])unsafe([[:space:]]|\{|$)|extern "C"'; then
    printf 'cargo-careful is required but cargo careful is unavailable. Install cargo-careful or set CAREFUL_REQUIRED=0 only with an approved waiver.\n' >&2
    exit 1
  fi

  log 'cargo-careful unavailable and no unsafe/FFI marker found; skipped'
}

verify_bolero() {
  if ! source_has 'bolero::|cargo[ -]bolero|\[dependencies\][[:space:][:print:]]*bolero'; then
    log 'no Bolero marker found; skipped'
    return
  fi

  if cargo_subcommand_available bolero; then
    run rustup run "$NIGHTLY" cargo bolero test --workspace
  else
    run rustup run "$NIGHTLY" cargo test --workspace --all-features bolero
  fi
}

verify_loom() {
  if ! source_has 'loom::|cfg\(loom\)|feature.*loom'; then
    log 'no Loom marker found; skipped'
    return
  fi

  run rustup run "$NIGHTLY" cargo test --workspace --all-features loom
}

verify_lockbud() {
  if [ -n "${LOCKBUD_CMD:-}" ]; then
    run_shell "$LOCKBUD_CMD"
    return
  fi

  if ! source_has 'Arc<Mutex|Mutex<|RwLock<|parking_lot|std::sync|tokio::sync'; then
    log 'no Lockbud concurrency marker found; skipped'
    return
  fi

  if command -v lockbud >/dev/null 2>&1; then
    printf 'lockbud is installed, but this repo requires an explicit LOCKBUD_CMD because lockbud invocation is project-specific.\n' >&2
  else
    printf 'Lockbud is required by concurrency markers, but lockbud is unavailable. Install lockbud or set LOCKBUD_CMD to the approved command.\n' >&2
  fi
  exit 1
}

verify_kani() {
  if [ -n "${KANI_CMD:-}" ]; then
    run_shell "$KANI_CMD"
    return
  fi

  if cargo_subcommand_available kani; then
    run rustup run "$NIGHTLY" cargo kani
    return
  fi

  if [ "${KANI_REQUIRED:-0}" = "1" ] || source_has 'kani::|\[kani::|proof_for_contract'; then
    printf 'Kani is required but cargo kani is unavailable. Install Kani or set KANI_CMD to the approved command.\n' >&2
    exit 1
  fi

  log 'cargo-kani unavailable and no Kani marker found; skipped'
}

verify_deep() {
  verify_standard
  moon_task miri
  verify_cargo_careful
  moon_task fuzz-smoke
  verify_bolero
  verify_loom
  verify_lockbud
  moon_task mutants-smoke
  moon_task coverage
}

verify_proof() {
  verify_kani
  run bash scripts/verify-lean.sh
}

case "$MODE" in
  fast) verify_fast ;;
  standard) verify_standard ;;
  deep) verify_deep ;;
  proof) verify_proof ;;
  all)
    verify_deep
    verify_proof
    ;;
esac
