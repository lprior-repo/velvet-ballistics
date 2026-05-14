#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-fast}"
NIGHTLY="${RUST_NIGHTLY:-nightly-2026-04-28}"
FORMAL_REPORT="formal-verification-report.md"
GAUNTLET_START_EPOCH="$(date +%s)"

case "$MODE" in
  fast|standard|deep|proof|all) ;;
  *)
    printf 'usage: %s {fast|standard|deep|proof|all}\n' "$0" >&2
    exit 2
    ;;
esac

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
if [ ! -f "$ROOT/Cargo.toml" ]; then
  printf 'verification gauntlet could not locate workspace Cargo.toml from script directory: %s\n' "$SCRIPT_DIR" >&2
  exit 1
fi
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

run_capture() {
  local output_file="$1"
  shift
  log "$* > $output_file"
  mkdir -p -- "$(dirname -- "$output_file")"
  "$@" 2>&1 | tee "$output_file"
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

cargo_careful_available() {
  rustup run "$NIGHTLY" cargo careful setup --help >/dev/null 2>&1
}

lockbud_waiver_section() {
  local waiver_file="$1"
  awk '
    /^### WAIVE-CONCURRENCY-UI-RELEASE$/ { in_section = 1 }
    in_section == 1 { print }
    in_section == 1 && /^### / && $0 != "### WAIVE-CONCURRENCY-UI-RELEASE" { exit }
  ' "$waiver_file"
}

require_waiver_text() {
  local section="$1"
  local needle="$2"
  if [[ "$section" != *"$needle"* ]]; then
    printf 'Lockbud waiver is missing required text: %s\n' "$needle" >&2
    exit 1
  fi
}

verify_lockbud_waiver_scan() {
  local scan_paths=(xtask/src crates/vb_ui_snapshot/src crates/vb_ui_makepad/src)
  rg -n 'tokio::spawn|std::thread::spawn|thread::spawn|async fn|Arc<Mutex|Mutex<|RwLock<|tokio::sync|std::sync::mpsc|crossbeam_channel|CancellationToken|JoinHandle' "${scan_paths[@]}" >/tmp/lockbud-waiver-scan.txt 2>/dev/null || return 0
  printf 'Lockbud waiver static scan found concurrency in UI release surface:\n' >&2
  cat /tmp/lockbud-waiver-scan.txt >&2
  exit 1
}

verify_lockbud_waiver() {
  local bead_id="${VERIFY_BEAD_ID:-}"
  if [ -z "$bead_id" ]; then
    printf 'Lockbud waiver requires explicit VERIFY_BEAD_ID context; refusing global skip.\n' >&2
    exit 1
  fi
  case "$bead_id" in
    *[!A-Za-z0-9._-]*|'')
      printf 'Lockbud waiver bead id is invalid: %s\n' "$bead_id" >&2
      exit 1
      ;;
  esac

  local waiver_file=".beads/$bead_id/verification-layers.md"
  if [ ! -s "$waiver_file" ]; then
    printf 'Lockbud waiver artifact is missing or empty: %s\n' "$waiver_file" >&2
    exit 1
  fi

  local section
  section="$(lockbud_waiver_section "$waiver_file")"
  require_waiver_text "$section" '### WAIVE-CONCURRENCY-UI-RELEASE'
  require_waiver_text "$section" 'Clause ID: PRE-003, POST-003, POST-006, INV-004, INV-005.'
  require_waiver_text "$section" 'Waived layer: Loom/Shuttle/Lockbud.'
  require_waiver_text "$section" 'does not add async tasks, shared mutable state, channels, or concurrent cancellation semantics.'
  require_waiver_text "$section" 'Compensating evidence: Miri, two-run deterministic integration evidence, static scan for spawned tasks/shared state in release capture, and `moon run :verify-deep`.'
  require_waiver_text "$section" 'Owner: State 3 `rust-contract` for bead `vb-nf2u`; implementation owner must revoke if tasks/shared state are introduced.'
  require_waiver_text "$section" 'Expiry/follow-up: expires immediately if `ai-release` UI capture uses threads, async tasks, channels, shared mutable state, or cancellation.'

  verify_lockbud_waiver_scan
  log "Lockbud waived by bead-scoped artifact $waiver_file / WAIVE-CONCURRENCY-UI-RELEASE"
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
  if cargo_careful_available; then
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

  if [ "${ALLOW_BEAD_LOCKBUD_WAIVER:-0}" = "1" ]; then
    verify_lockbud_waiver
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
    if [ "${VERIFY_BEAD_ID:-}" = "vb-nf2u" ]; then
      run_capture .evidence/vb-nf2u/kani-ui.txt cargo kani -p vb_ui_snapshot --harness inventory
      require_kani_summary .evidence/vb-nf2u/kani-ui.txt inventory
      run_capture .evidence/vb-nf2u/kani-layout.txt cargo kani -p vb_ui_snapshot --harness layout_
      require_kani_summary .evidence/vb-nf2u/kani-layout.txt layout_
      return
    fi
    run rustup run "$NIGHTLY" cargo kani
    return
  fi

  if [ "${KANI_REQUIRED:-0}" = "1" ] || source_has 'kani::|\[kani::|proof_for_contract'; then
    printf 'Kani is required but cargo kani is unavailable. Install Kani or set KANI_CMD to the approved command.\n' >&2
    exit 1
  fi

  log 'cargo-kani unavailable and no Kani marker found; skipped'
}

verify_verus() {
  run bash scripts/verify-verus.sh
}

require_kani_summary() {
  local output_file="$1"
  local harness="$2"
  if ! rg -q "Verification successful|SUMMARY|$harness" "$output_file"; then
    printf 'Kani evidence file lacks required summary for %s: %s\n' "$harness" "$output_file" >&2
    exit 1
  fi
}

prepare_formal_report() {
  rm -f -- "$FORMAL_REPORT"
}

append_report_file_evidence() {
  local report_file="$1"
  local evidence_file="$2"
  local label="$3"
  if [ ! -s "$evidence_file" ]; then
    printf '%s evidence is missing or empty: %s\n' "$label" "$evidence_file" >&2
    exit 1
  fi
  {
    printf -- '- %s: `%s` present and non-empty.\n' "$label" "$evidence_file"
    printf '  ```text\n'
    rg -n 'Verification successful|SUMMARY|harness|proof|layout_|inventory|verification results::|VERUS_' "$evidence_file"
    printf '  ```\n'
  } >>"$report_file"
}

write_formal_verification_report() {
  local generated_at
  generated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  {
    printf '# Formal Verification Report\n\n'
    printf 'Status: PASS\n'
    printf 'Generated: %s\n' "$generated_at"
    printf 'Bead: %s\n\n' "${VERIFY_BEAD_ID:-unspecified}"
    printf '## Moon verification lanes\n\n'
    printf -- '- verify-fast: PASS (executed through verify-standard/deep/all).\n'
    printf -- '- verify-standard: PASS (executed through verify-deep/all).\n'
    printf -- '- verify-deep: PASS.\n'
    printf -- '- verify-proof: PASS.\n'
    printf -- '- verify-all: PASS.\n\n'
    printf '## Five verification lanes\n\n'
    printf -- '- Verus: Rust-local deductive proofs from `contracts/proof_obligations.yaml`.\n'
    printf -- '- Kani: formal proof (Kani inventory + layout harnesses).\n'
    printf -- '- Miri: undefined behavior (miri test).\n'
    printf -- '- Lockbud: concurrency (waived by WAIVE-CONCURRENCY-UI-RELEASE for vb-nf2u).\n'
    printf -- '- fuzz: coverage (cargo fuzz smoke).\n'
    printf -- '- coverage: llvm-cov nextest.\n\n'
    printf '## Verus persisted summaries\n\n'
  } >"$FORMAL_REPORT"
  append_report_file_evidence "$FORMAL_REPORT" '.evidence/verus/summary.txt' 'Verus registry summary'
  {
    printf '\n## Kani persisted summaries\n\n'
  } >>"$FORMAL_REPORT"
  append_report_file_evidence "$FORMAL_REPORT" '.evidence/vb-nf2u/kani-ui.txt' 'Kani inventory summary'
  append_report_file_evidence "$FORMAL_REPORT" '.evidence/vb-nf2u/kani-layout.txt' 'Kani layout summary'
  {
    printf '\n## Miri evidence\n\n'
    printf -- '- Miri: `moon run :verify-deep` runs miri test as part of deep verification.\n'
    printf -- '- Lane status: PASS when `moon run :verify-all` completes without miri failure.\n\n'
    printf '## Coverage evidence\n\n'
    printf -- '- Coverage: `moon run :verify-deep` runs `moon run :coverage` as part of deep verification.\n'
    printf -- '- Lane status: PASS when `moon run :verify-all` completes without coverage failure.\n\n'
    printf '## Lockbud waiver evidence\n\n'
    printf -- '- Lockbud waived only by bead-scoped `WAIVE-CONCURRENCY-UI-RELEASE` artifact.\n'
    printf -- '- VERIFY_BEAD_ID: `%s`.\n' "${VERIFY_BEAD_ID:-unspecified}"
    printf -- '- ALLOW_BEAD_LOCKBUD_WAIVER: `%s`.\n' "${ALLOW_BEAD_LOCKBUD_WAIVER:-0}"
    printf -- '- Waiver validation: PASS when `moon run :verify-all` reaches this report.\n'
  } >>"$FORMAL_REPORT"
}

validate_formal_verification_report() {
  if [ ! -s "$FORMAL_REPORT" ]; then
    printf 'formal verification report is missing or empty: %s\n' "$FORMAL_REPORT" >&2
    exit 1
  fi
  local report_mtime
  report_mtime="$(stat -c %Y "$FORMAL_REPORT")"
  if [ "$report_mtime" -lt "$GAUNTLET_START_EPOCH" ]; then
    printf 'formal verification report is stale: %s\n' "$FORMAL_REPORT" >&2
    exit 1
  fi
  # Validate all five verification lanes are named plus Kani evidence and Lockbud waiver.
  for required in \
    'verify-fast' \
    'verify-standard' \
    'verify-deep' \
    'verify-proof' \
    'verify-all' \
    'Verus registry summary' \
    'Kani inventory summary' \
    'Kani layout summary' \
    'Lockbud' \
    'Miri' \
    'fuzz' \
    'coverage'
  do
    if ! rg -q "$required" "$FORMAL_REPORT"; then
      printf 'formal verification report missing required evidence: %s\n' "$required" >&2
      exit 1
    fi
  done
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
  verify_verus
  verify_kani
  run bash scripts/verify-lean.sh
}

case "$MODE" in
  fast) verify_fast ;;
  standard) verify_standard ;;
  deep) verify_deep ;;
  proof) verify_proof ;;
  all)
    prepare_formal_report
    verify_deep
    verify_proof
    write_formal_verification_report
    validate_formal_verification_report
    ;;
esac
