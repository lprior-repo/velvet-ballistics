#!/usr/bin/env bash
# check-codes-registry-assembly: CI guard for the assembler pattern + data-file
# format agreement in crates/vb_core/src/diagnostic/{codes.rs,codes/}.
#
# Background (bead vb-q7d5c): Wave 1 verification observed three
# mutually-incompatible states of codes.rs and its 20 sibling data files
# within a 5-minute window:
#   (A) const ENTRIES + const fn build_registry — compiles
#   (B) include!() pattern with flat data files  — compiles
#   (C) const pattern but accessor.rs:106:4 unclosed delimiter — fails
# This script detects transient refactor states where codes.rs switches the
# assembler pattern but the data files have not been migrated (or vice
# versa), so reviewers and CI catch the drift before merge.
#
# Hard invariants enforced:
#   I-1: Every `mod X;` declaration in codes.rs has a corresponding
#        `codes/X.rs` file.
#   I-2: Every codes/X.rs file declares exactly one `pub(super) const ENTRIES`
#        with type `&[super::CodeEntry]`.
#   I-3: codes.rs declares exactly one assembler pattern. Either the
#        `const fn build_registry()` pattern (State A) or the `include!()`
#        pattern (State B). Mixing both patterns is transient refactor state.
#   I-4: When codes.rs uses the const-fn assembler, every `mod X;` module
#        appears in exactly one `copy_slice(X::ENTRIES, …)` call inside
#        `build_registry`.
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

CODES_RS="crates/vb_core/src/diagnostic/codes.rs"
CODES_DIR="crates/vb_core/src/diagnostic/codes"

if [[ ! -f "$ROOT/$CODES_RS" ]]; then
  echo "ViolationFound: $CODES_RS is missing" >&2
  exit 2
fi
if [[ ! -d "$ROOT/$CODES_DIR" ]]; then
  echo "ViolationFound: $CODES_DIR is missing" >&2
  exit 2
fi

fail=0

report() {
  local kind="$1"
  local detail="$2"
  echo "  [$kind] $detail" >&2
  fail=1
}

echo "CWD: $ROOT"
echo "CommitSHA: $(git rev-parse HEAD 2>/dev/null || jj log -r @ --no-graph -T 'commit_id.short(12) ++ "\n"' 2>/dev/null || echo unknown)"
echo "Toolchain: $(rustc --version)"
echo "ScanDomain: $CODES_RS, $CODES_DIR"
echo "Invariants: I-1 mod declarations match data files; I-2 every data file exposes ENTRIES; I-3 exactly one assembler pattern; I-4 const-fn assembler references every module"

# ---------------------------------------------------------------------------
# I-1: mod declarations in codes.rs ↔ data files in codes/
# ---------------------------------------------------------------------------

mapfile -t declared_mods < <(
  awk '
    /^[[:space:]]*mod[[:space:]]+[A-Za-z0-9_]+;[[:space:]]*$/ {
      match($0, /mod[[:space:]]+([A-Za-z0-9_]+);/, m)
      if (m[1] != "") print m[1]
    }
  ' "$ROOT/$CODES_RS" | sort -u
)

mapfile -t present_files < <(
  find "$ROOT/$CODES_DIR" -maxdepth 1 -type f -name '*.rs' -printf '%f\n' \
    | sed 's/\.rs$//' \
    | sort -u
)

echo "DeclaredMods: ${#declared_mods[@]}"
echo "DataFiles:    ${#present_files[@]}"

for mod in "${declared_mods[@]}"; do
  if [[ ! -f "$ROOT/$CODES_DIR/$mod.rs" ]]; then
    report I-1 "codes.rs declares 'mod $mod;' but $CODES_DIR/$mod.rs is missing"
  fi
done

for file in "${present_files[@]}"; do
  found=0
  for mod in "${declared_mods[@]}"; do
    if [[ "$mod" == "$file" ]]; then
      found=1
      break
    fi
  done
  if [[ "$found" -eq 0 ]]; then
    report I-1 "$CODES_DIR/$file.rs exists but is not declared via 'mod $file;' in codes.rs"
  fi
done

# ---------------------------------------------------------------------------
# I-2: every data file exposes pub(super) const ENTRIES: &[super::CodeEntry]
# ---------------------------------------------------------------------------

entries_pat='pub\(super\)[[:space:]]+const[[:space:]]+ENTRIES[[:space:]]*:[[:space:]]*&\[super::CodeEntry\]'

for file in "${present_files[@]}"; do
  path="$ROOT/$CODES_DIR/$file.rs"
  count=$(grep -cE "$entries_pat" "$path" || true)
  if [[ "$count" -ne 1 ]]; then
    report I-2 "$CODES_DIR/$file.rs must declare exactly one '$entries_pat' (found $count)"
  fi
done

# ---------------------------------------------------------------------------
# I-3: codes.rs uses exactly one assembler pattern
# ---------------------------------------------------------------------------

has_build_registry=0
has_include_macro=0
if grep -qE 'const[[:space:]]+fn[[:space:]]+build_registry\b' "$ROOT/$CODES_RS"; then
  has_build_registry=1
fi
if grep -qE '^[[:space:]]*include!\(' "$ROOT/$CODES_RS"; then
  has_include_macro=1
fi

echo "AssemblerPattern: build_registry=$has_build_registry include!=$has_include_macro"

if [[ "$has_build_registry" -eq 1 && "$has_include_macro" -eq 1 ]]; then
  report I-3 "codes.rs mixes const fn build_registry() with include!() — pick one assembler pattern per commit"
fi

# ---------------------------------------------------------------------------
# I-4: build_registry references every declared module via copy_slice
# ---------------------------------------------------------------------------

if [[ "$has_build_registry" -eq 1 ]]; then
  mapfile -t referenced < <(
    awk '
      /copy_slice\(/ {
        match($0, /copy_slice\(([A-Za-z0-9_]+)::/, m)
        if (m[1] != "") print m[1]
      }
    ' "$ROOT/$CODES_RS" | sort -u
  )

  for mod in "${declared_mods[@]}"; do
    found=0
    for ref in "${referenced[@]}"; do
      if [[ "$ref" == "$mod" ]]; then
        found=1
        break
      fi
    done
    if [[ "$found" -eq 0 ]]; then
      report I-4 "build_registry() does not reference '$mod::ENTRIES' — every 'mod $mod;' must have a matching copy_slice($mod::ENTRIES, ...)"
    fi
  done

  for ref in "${referenced[@]}"; do
    found=0
    for mod in "${declared_mods[@]}"; do
      if [[ "$mod" == "$ref" ]]; then
        found=1
        break
      fi
    done
    if [[ "$found" -eq 0 ]]; then
      report I-4 "build_registry() references '$ref::ENTRIES' but 'mod $ref;' is not declared in codes.rs"
    fi
  done
fi

# ---------------------------------------------------------------------------
# Verdict
# ---------------------------------------------------------------------------

if [[ "$fail" -ne 0 ]]; then
  echo "ViolationFound: codes-registry assembler pattern and data-file format are out of sync" >&2
  echo "ExitCode: 2"
  exit 2
fi

echo "NoViolationFound"
echo "ExitCode: 0"
exit 0
