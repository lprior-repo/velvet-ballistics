#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

TMP_BASE="${TMPDIR:-target/tmp}"
mkdir -p "$TMP_BASE"

ALLOW_FILE_DEFAULT="scripts/ignored-fallible-results.allow"

declare -A ALLOW_MAP=()

reset_allow_map() {
  ALLOW_MAP=()
}

validate_allow_file() {
  local allow_file="$1"
  reset_allow_map
  [[ -f "$allow_file" ]] || return 0

  local line_no=0
  local line=""
  while IFS= read -r line || [[ -n "$line" ]]; do
    line_no=$((line_no + 1))
    [[ -z "$line" || "${line:0:1}" == "#" ]] && continue

    IFS='|' read -r path class owner expiry follow_up reason extra <<< "$line"
    if [[ -n "${extra:-}" || -z "${path:-}" || -z "${class:-}" || -z "${owner:-}" || -z "${expiry:-}" || -z "${follow_up:-}" || -z "${reason:-}" ]]; then
      echo "MalformedException: $allow_file:$line_no expected path|class|owner=...|expiry=...|follow_up=...|reason=..." >&2
      return 3
    fi
    if [[ "$path" == *"*"* || "$path" == "." || "$path" == /* || "$path" != crates/*/src/*.rs && "$path" != crates/*/src/**/*.rs && "$path" != xtask/src/*.rs && "$path" != xtask/src/**/*.rs ]]; then
      echo "OverbroadException: $allow_file:$line_no path must target crates/*/src or xtask/src file" >&2
      return 3
    fi
    if [[ "$class" == "*" || "$class" == "ALL" || "$class" != DISCARD-* ]]; then
      echo "OverbroadException: $allow_file:$line_no class must be one DISCARD-* class" >&2
      return 3
    fi
    if [[ "$owner" != owner=* || "$expiry" != expiry=* || "$follow_up" != follow_up=* || "$reason" != reason=* ]]; then
      echo "MalformedException: $allow_file:$line_no missing owner/expiry/follow_up/reason field" >&2
      return 3
    fi

    ALLOW_MAP["$path|$class"]=1
  done < "$allow_file"
}

is_allowed() {
  local rel="$1"
  local class="$2"
  [[ -n "${ALLOW_MAP[$rel|$class]:-}" ]]
}

should_skip_file() {
  local rel="$1"
  case "$rel" in
    crates/*/src/kani_*.rs|crates/*/src/**/kani_*.rs) return 0 ;;
    crates/workspace_tests/src/*|crates/workspace_tests/src/**) return 0 ;;
    crates/*/src/*_tests.rs|crates/*/src/**/*_tests.rs) return 0 ;;
    crates/*/src/test_harness.rs|crates/*/src/**/test_harness.rs) return 0 ;;
    crates/*/src/**/tests/*.rs|crates/*/src/**/tests/**/*.rs) return 0 ;;
    crates/*/src/**/impl_tests/*.rs|crates/*/src/**/impl_tests/**/*.rs) return 0 ;;
    crates/*/src/**/lifecycle_tests/*.rs|crates/*/src/**/lifecycle_tests/**/*.rs) return 0 ;;
    *) return 1 ;;
  esac
}

brace_delta() {
  local text="$1"
  local no_open="${text//\{/}"
  local no_close="${text//\}/}"
  local opens=$(( ${#text} - ${#no_open} ))
  local closes=$(( ${#text} - ${#no_close} ))
  printf '%s\n' "$(( opens - closes ))"
}

is_nonproduction_cfg_line() {
  local line="$1"
  [[ "$line" =~ ^[[:space:]]*#\[cfg\((test|kani)\)\] || "$line" =~ ^[[:space:]]*#\[cfg\([^]]*(test|kani)[^]]*\)\] ]]
}

is_module_open_line() {
  local line="$1"
  [[ "$line" =~ ^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+[A-Za-z0-9_]+[[:space:]]*\{ ]]
}

is_fallible_lossy_conversion_source() {
  local compact="$1"
  local lossy_source='(fallible|write_|append|flush|send\(|recv\(|cancel|persist|commit|remove_|remove_dir|remove_file|create_|open_|save_|read_to_|from_bytes|to_allocvec|try_from_parts)'
  [[ "$compact" =~ $lossy_source ]]
}

record_violation() {
  local out="$1"
  local rel="$2"
  local line_no="$3"
  local class="$4"
  local text="$5"

  if is_allowed "$rel" "$class"; then
    printf 'JustifiedException|%s|%s|line=%s\n' "$class" "$rel" "$line_no" >> "$out"
    return 0
  fi

  printf 'ViolationFound|%s|%s|line=%s|%s\n' "$class" "$rel" "$line_no" "$text" >> "$out"
}

classify_line() {
  local out="$1"
  local rel="$2"
  local line_no="$3"
  local line="$4"
  local compact="${line//[[:space:]]/}"
  local fallible_call='(^|.*[^[:alnum:]_])(fallible[[:alnum:]_]*|try_[[:alnum:]_]*)\(.*\);'
  local let_fallible_call='=.*(fallible[[:alnum:]_]*|try_[[:alnum:]_]*|write_[[:alnum:]_]*|append[[:alnum:]_]*|flush[[:alnum:]_]*|send|recv|cancel[[:alnum:]_]*|persist[[:alnum:]_]*|commit[[:alnum:]_]*|remove_[[:alnum:]_]*|create_[[:alnum:]_]*|open_[[:alnum:]_]*|save_[[:alnum:]_]*|read_to_[[:alnum:]_]*)\(.*\);'
  local drop_call='(fallible|try_|write|append|flush|send\(|recv\(|cancel|persist|commit|remove_|remove_dir|remove_file|create_|open_|save_|read_to_)'
  local drop_shape='(^|.*[^[:alnum:]_])drop\(.*;'

  if [[ "$line" =~ allow\((unused_must_use|clippy::let_underscore_must_use)\) || "$line" == *"ignored-fallible"* || "$line" == *"silent discard"* ]]; then
    record_violation "$out" "$rel" "$line_no" "DISCARD-006" "$compact"
  fi
  if [[ "$line" =~ (^|[[:space:]])let[[:space:]]+_[[:space:]]*= && "$compact" =~ $let_fallible_call ]]; then
    record_violation "$out" "$rel" "$line_no" "DISCARD-002" "$compact"
  fi
  if [[ "$line" =~ \.(ok|err)\(\)[[:space:]]*\; && "$compact" != *"="* ]]; then
    record_violation "$out" "$rel" "$line_no" "DISCARD-003" "$compact"
  fi
  if [[ "$compact" == *".ok()"* || "$compact" == *".err()"* ]]; then
    if is_fallible_lossy_conversion_source "$compact"; then
      record_violation "$out" "$rel" "$line_no" "DISCARD-003" "$compact"
    fi
  fi
  if [[ "$compact" == *"Err(_)=>{}"* || "$compact" == *"Ok(())|Err(_)=>{}"* ]]; then
    record_violation "$out" "$rel" "$line_no" "DISCARD-004" "$compact"
  fi
  if [[ "$compact" =~ $drop_shape && "$compact" =~ $drop_call ]]; then
    record_violation "$out" "$rel" "$line_no" "DISCARD-005" "$compact"
  fi
  if [[ "$compact" != .* && "$compact" != *"="* && "$compact" != *"|"* && "$compact" != *"assert"* && "$compact" != *"expect("* && "$compact" != *"unwrap"* && "$compact" != *".ok();"* && "$compact" != *".err();"* && "$compact" =~ $fallible_call ]]; then
    record_violation "$out" "$rel" "$line_no" "DISCARD-001" "$compact"
  fi
}

scan_tree() {
  local scan_root="$1"
  local allow_file="$2"
  local report="$3"

  validate_allow_file "$allow_file" || return 3
  : > "$report"

  local roots=()
  local crate_src=""
  for crate_src in "$scan_root"/crates/*/src; do
    [[ -d "$crate_src" ]] && roots+=("$crate_src")
  done
  [[ -d "$scan_root/xtask/src" ]] && roots+=("$scan_root/xtask/src")

  if [[ "${#roots[@]}" -eq 0 ]]; then
    echo "UnreadableInput: no production scan roots found" >&2
    return 4
  fi

  local file=""
  local line=""
  local line_no=0
  local rel=""
  local compact=""
  local pending_cfg_nonproduction=0
  local skip_depth=0
  local pending_lossy_line_no=0
  local pending_lossy_text=""
  while IFS= read -r file; do
    [[ -r "$file" ]] || { echo "UnreadableInput: $file" >&2; return 4; }
    rel="${file#"$scan_root/"}"
    if should_skip_file "$rel"; then
      continue
    fi
    line_no=0
    pending_cfg_nonproduction=0
    skip_depth=0
    pending_lossy_line_no=0
    pending_lossy_text=""
    while IFS= read -r line || [[ -n "$line" ]]; do
      line_no=$((line_no + 1))
      if [[ "$skip_depth" -gt 0 ]]; then
        skip_depth=$((skip_depth + $(brace_delta "$line")))
        [[ "$skip_depth" -lt 0 ]] && skip_depth=0
        continue
      fi
      if is_nonproduction_cfg_line "$line"; then
        pending_cfg_nonproduction=1
        continue
      fi
      if [[ "$pending_cfg_nonproduction" -eq 1 ]]; then
        if [[ "$line" =~ ^[[:space:]]*#\[ || -z "${line//[[:space:]]/}" ]]; then
          continue
        fi
        if is_module_open_line "$line"; then
          skip_depth=$(brace_delta "$line")
          [[ "$skip_depth" -le 0 ]] && skip_depth=1
          pending_cfg_nonproduction=0
          continue
        fi
        pending_cfg_nonproduction=0
      fi
      compact="${line//[[:space:]]/}"
      if [[ "$pending_lossy_line_no" -ne 0 && "$compact" == .* && ( "$compact" == *".ok()"* || "$compact" == *".err()"* ) ]]; then
        record_violation "$report" "$rel" "$line_no" "DISCARD-003" "${pending_lossy_text}${compact}"
      fi
      if is_fallible_lossy_conversion_source "$compact" && [[ "$compact" != *".ok()"* && "$compact" != *".err()"* && "$compact" != *";"* && "$compact" != *"?"* ]]; then
        pending_lossy_line_no="$line_no"
        pending_lossy_text="$compact"
      elif [[ "$compact" != .* ]]; then
        pending_lossy_line_no=0
        pending_lossy_text=""
      fi
      classify_line "$report" "$rel" "$line_no" "$line"
    done < "$file"
  done < <(rg --files "${roots[@]}" -g '*.rs' | LC_ALL=C sort)

  if [[ -s "$report" ]]; then
    LC_ALL=C sort "$report"
    if rg -q '^ViolationFound\|' "$report"; then
      return 2
    fi
    return 0
  fi

  echo "NoViolationFound"
}

write_fixture() {
  local file="$1"
  local body="$2"
  mkdir -p "$(dirname "$file")"
  printf '%s\n' "$body" > "$file"
}

expect_status() {
  local label="$1"
  local expected="$2"
  local fixture_root="$3"
  local allow_file="$4"
  local report="$5"

  set +e
  scan_tree "$fixture_root" "$allow_file" "$report" >/dev/null 2>"$report.err"
  local actual="$?"
  set -e
  if [[ "$actual" -ne "$expected" ]]; then
    echo "FixtureFailure: $label expected=$expected actual=$actual" >&2
    [[ -s "$report" ]] && LC_ALL=C sort "$report" >&2
    [[ -s "$report.err" ]] && LC_ALL=C sort "$report.err" >&2
    return 1
  fi
  echo "FixturePass: $label exit=$actual"
}

run_self_tests() {
  local dir=""
  dir="$(mktemp -d "$TMP_BASE/ignored-fallible.XXXXXX")"
  trap 'rm -rf "$dir"' RETURN

  local report="$dir/report.txt"
  local allow="$dir/allow.txt"
  : > "$allow"

  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn clean() -> Result<(), ()> { fallible_result()?; Ok(()) }'
  expect_status "clean production-like fixture" 0 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn bare() { fallible_result(); }'
  expect_status "DISCARD-001 bare fallible call" 2 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn let_under() { let _ = fallible_result(); }'
  expect_status "DISCARD-002 let underscore" 2 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn ok_lossy() { fallible_result().ok(); }'
  expect_status "DISCARD-003 ok err lossy" 2 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn embedded_ok_lossy(bytes: &[u8]) { let parsed = postcard::from_bytes::<u8>(bytes).ok().unwrap_or(0); }'
  expect_status "DISCARD-003 embedded ok lossy" 2 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" $'pub fn split_ok_lossy(bytes: &[u8]) {\n    let parsed = postcard::from_bytes::<u8>(bytes)\n        .ok()\n        .unwrap_or(0);\n}'
  expect_status "DISCARD-003 split ok lossy" 2 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn swallow(r: Result<(), ()>) { match r { Ok(()) | Err(_) => {} } }'
  expect_status "DISCARD-004 swallowed Err" 2 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn drop_lossy() { drop(write_result()); }'
  expect_status "DISCARD-005 drop fallible" 2 "$dir" "$allow" "$report"

  write_fixture "$dir/crates/demo/src/lib.rs" '#[allow(unused_must_use)] pub fn marker() { fallible_result(); }'
  expect_status "DISCARD-006 undocumented allow marker" 2 "$dir" "$allow" "$report"

  printf '%s\n' 'crates/demo/src/lib.rs|DISCARD-001|owner=proof|expiry=2026-12-31|follow_up=vb-qi37.12.4|reason=fixture' > "$allow"
  write_fixture "$dir/crates/demo/src/lib.rs" 'pub fn bare() { fallible_result(); }'
  expect_status "path-bound justified exception" 0 "$dir" "$allow" "$report"

  printf '%s\n' 'crates/demo/src/lib.rs|ALL|owner=proof|expiry=2026-12-31|follow_up=vb-qi37.12.4|reason=fixture' > "$allow"
  expect_status "overbroad exception rejected" 3 "$dir" "$allow" "$report"

  printf '%s\n' 'crates/demo/src/lib.rs|DISCARD-001|owner=proof|reason=fixture' > "$allow"
  expect_status "malformed exception rejected" 3 "$dir" "$allow" "$report"
}

run_self_tests

PROD_REPORT="$TMP_BASE/ignored-fallible-results.production.txt"
echo "ScanDomain: crates/*/src xtask/src"
echo "NonProductionExcluded: tests benches examples fuzz target .beads fixtures"
scan_tree "$ROOT" "$ALLOW_FILE_DEFAULT" "$PROD_REPORT"
