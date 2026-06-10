#!/usr/bin/env bash
# scripts/check-kani-shape-vacuity.sh
# Bead: vb-scr01 — R1-A11 / R4-A11 missing check
#
# Asserts that no Kani harness under `crates/*/src/kani/` (and the
# `mod_compile_lowering/kani/` and `verification/kani/` sub-trees)
# has a vacuous body. A vacuous body is one that:
#
#   (1) Has no symbolic input. The harness body does not call
#       `kani::any::<T>()`, `kani::nondet::<T>()`, or
#       `kani::assume(...)`. Without symbolic input, the harness
#       reduces to a single concrete test path, and any
#       `kani::assert!(...)` call is a tautology.
#   (2) Only asserts on hardcoded constants. Every `kani::assert!`
#       call in the body has at least one operand that is a Rust
#       literal (`0`, `false`, `""`, etc.) on both sides of the
#       comparison. A non-literal operand signals "the assertion
#       actually depends on something."
#
# A harness that fails either (1) or (2) is considered vacuous. The
# check reports the file, the harness name, and a snippet of the body
# to make triage cheap. It does NOT mark the harness as broken; it
# only flags the shape so a reviewer can decide whether the harness
# is intentionally concrete (e.g. an end-to-end sanity check) or
# vacuous by mistake.
#
# Output:
#   - Exits 0 when no vacuous harnesses are found.
#   - Exits 1 when one or more vacuous harnesses are found.
#   - Exits 2 on usage or environment error.
#
# This script is read-only: it never modifies repository state. It
# does not require cargo; it uses awk-based harness extraction.

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

if [[ ! -d "$ROOT/crates" ]]; then
  printf 'check-kani-shape-vacuity: error: %s/crates is not a directory\n' "$ROOT" >&2
  exit 2
fi

if ! command -v awk >/dev/null 2>&1; then
  printf 'check-kani-shape-vacuity: error: awk is required on PATH\n' >&2
  exit 2
fi

KANI_ROOTS=(
  "$ROOT/crates"
  "$ROOT/verification/kani"
)

printf 'check-kani-shape-vacuity: scanning for vacuous #[kani::proof] harnesses\n' >&2

failed=0
scanned=0
vacuous=0

# Extract every `#[kani::proof]`-decorated function body. The body is
# the contiguous run of lines from the `fn name(...)` declaration
# up to the matching closing brace at column 0.
extract_harnesses() {
  local file="$1"
  awk '
    BEGIN { in_harness = 0; brace = 0; name = ""; sig_line = 0 }
    {
      line = $0
      # Look for the `#[kani::proof]` attribute (possibly alone on
      # its own line) followed by an `fn name(...)` declaration.
      if (!in_harness) {
        if (match(line, /^[[:space:]]*#\[kani::proof\][[:space:]]*$/)) {
          pending = 1
          next
        }
        if (pending && match(line, /^[[:space:]]*(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)/, m)) {
          in_harness = 1
          name = m[3]
          sig_line = NR
          # Start brace counting from the opening brace of the body
          # (may be on the same line or the next).
          brace = 0
          for (i = 1; i <= length(line); i++) {
            ch = substr(line, i, 1)
            if (ch == "{") brace++
            if (ch == "}") brace--
          }
          if (brace <= 0) {
            # body fits on one line and is closed; print + reset.
            print FILENAME "\t" name "\t" sig_line "\t" sig_line "\t" line
            in_harness = 0
            name = ""
            pending = 0
            next
          }
          pending = 0
          next
        }
        # a non-fn line after the attribute discards the attribute
        pending = 0
        next
      }
      # in_harness
      end = NR
      for (i = 1; i <= length(line); i++) {
        ch = substr(line, i, 1)
        if (ch == "{") brace++
        if (ch == "}") brace--
      }
      if (brace <= 0) {
        print FILENAME "\t" name "\t" sig_line "\t" end "\t" line
        in_harness = 0
        name = ""
      }
    }
  ' "$file"
}

scan_harness() {
  local file="$1"
  local name="$2"
  local sig_line="$3"
  local end_line="$4"
  local sample="$5"

  local body
  body=$(sed -n "${sig_line},${end_line}p" "$file" 2>/dev/null || true)

  local has_symbolic
  has_symbolic=$(printf '%s\n' "$body" | grep -cE 'kani::(any|nondet|assume)<|kani::(any|nondet|assume)\(' || true)

  # `kani::cover!(true, ...)` is a degenerate cover check that always
  # reports a hit. This is a shape defect.
  local has_degenerate_cover
  has_degenerate_cover=$(printf '%s\n' "$body" | grep -cE 'kani::cover!\([[:space:]]*true[[:space:]]*,' || true)

  # For the "asserts on hardcoded constants" check, we look for
  # `kani::assert!` calls and inspect each. An `kani::assert!` whose
  # argument contains only Rust literals, `==`, and parens (no
  # variable, no kani::any result) is degenerate.
  local degenerate_asserts=0
  while IFS= read -r assert_line; do
    [[ -z "$assert_line" ]] && continue
    # Extract the body of the assertion (between the first `(` after
    # `kani::assert!` and the matching `)`). We use a tolerant awk
    # that strips the prefix and then checks for variable-shaped
    # tokens.
    inner=$(printf '%s\n' "$assert_line" | sed -n 's/^[[:space:]]*kani::assert!(//p' | sed 's/);[[:space:]]*$//')
    [[ -z "$inner" ]] && continue
    # A non-degenerate assertion contains an identifier that is NOT a
    # Rust literal or a `==`/`!=`/etc operator. Allowed noise: digits,
    # `_`, `"`, `'`, `+`, `-`, `(`, `)`, `,`, space, `.`, `:`, `;`.
    if printf '%s' "$inner" | grep -qE '[A-Za-z_][A-Za-z0-9_]*[[:space:]]*==' ; then
      # Found identifier followed by `==`. Confirm the identifier is
      # not a constant (heuristic: ignore `true`, `false`, `None`,
      # `Some`, `Ok`, `Err`).
      ident=$(printf '%s' "$inner" | grep -oE '[A-Za-z_][A-Za-z0-9_]*' | grep -vE '^(true|false|None|Some|Ok|Err)$' | head -1 || true)
      if [[ -z "$ident" ]]; then
        degenerate_asserts=$((degenerate_asserts + 1))
      fi
    else
      # No `identifier ==` pattern at all; if there is also no
      # identifier anywhere, the assertion is purely a constant.
      if ! printf '%s' "$inner" | grep -qE '[A-Za-z_][A-Za-z0-9_]*' ; then
        degenerate_asserts=$((degenerate_asserts + 1))
      elif ! printf '%s' "$inner" | grep -qE '[A-Za-z_][A-Za-z0-9_]*[[:space:]]*[A-Za-z_]|[A-Za-z_][[:space:]]*[A-Za-z_]' ; then
        # A single identifier alone is also a tautology.
        degenerate_asserts=$((degenerate_asserts + 1))
      fi
    fi
  done < <(printf '%s\n' "$body" | grep -E 'kani::assert!|assert!' | head -20 || true)

  if [[ "$has_symbolic" -eq 0 ]]; then
    printf '  [VACUOUS] file=%s harness=%s sig_line=%s reason="no symbolic input (kani::any / kani::nondet / kani::assume not found in body)"\n' \
      "$file" "$name" "$sig_line" >&2
    failed=1
    vacuous=$((vacuous + 1))
    return 0
  fi

  if [[ "$has_degenerate_cover" -gt 0 ]]; then
    printf '  [VACUOUS] file=%s harness=%s sig_line=%s reason="kani::cover!(true, ...) (always-hit cover)"\n' \
      "$file" "$name" "$sig_line" >&2
    failed=1
    vacuous=$((vacuous + 1))
    return 0
  fi

  if [[ "$degenerate_asserts" -gt 0 ]]; then
    printf '  [VACUOUS] file=%s harness=%s sig_line=%s reason="asserts on hardcoded constants (degenerate_asserts=%d)"\n' \
      "$file" "$name" "$sig_line" "$degenerate_asserts" >&2
    failed=1
    vacuous=$((vacuous + 1))
    return 0
  fi
}

# Build the list of Kani harness files to scan. We look in the
# canonical `crates/*/src/kani/`, `crates/*/src/.../kani/`, and the
# `verification/kani/` directories.
mapfile -t KANI_FILES < <(
  for root in "${KANI_ROOTS[@]}"; do
    [[ -d "$root" ]] || continue
    find "$root" -type f -name '*.rs' -path '*/kani/*' 2>/dev/null || true
  done | sort -u
)

if [[ ${#KANI_FILES[@]} -eq 0 ]]; then
  printf 'check-kani-shape-vacuity: OK (no kani harness files found; nothing to check)\n'
  exit 0
fi

printf '  scanning %d kani harness files...\n' "${#KANI_FILES[@]}" >&2

for file in "${KANI_FILES[@]}"; do
  while IFS=$'\t' read -r file_ name sig_line end_line sample; do
    [[ -z "$name" ]] && continue
    scanned=$((scanned + 1))
    scan_harness "$file_" "$name" "$sig_line" "$end_line" "$sample" || true
  done < <(extract_harnesses "$file" | awk -v f="$file" 'BEGIN { OFS = "\t" } { $1 = f; print }' )
done

printf '\nSUMMARY: harnesses=%d vacuous=%d\n' "$scanned" "$vacuous"

if [[ "$failed" -ne 0 ]]; then
  printf '\ncheck-kani-shape-vacuity: FAILED — one or more harnesses are vacuous.\n' >&2
  printf '  A vacuous harness does not exercise any symbolic input or asserts\n' >&2
  printf '  on hardcoded constants; the proof is meaningless and the harness\n' >&2
  printf '  should be expanded to use kani::any / kani::nondet / kani::assume,\n' >&2
  printf '  or removed if it was a placeholder.\n' >&2
  exit 1
fi

printf '\ncheck-kani-shape-vacuity: OK (no vacuous kani harnesses)\n'
exit 0
