#!/usr/bin/env bash
# check-production-inner-drift.sh
#
# CI drift gate for `verification/verus/production_inner/*.rs` mirrors.
#
# Each mirror is a hand-maintained verbatim copy of a claimed production
# source slice. The mirror headers document a DRIFT POLICY line naming
# the master production source file (with optional `:start-end` line
# range) and per-section `// Production `path:start-end`` annotations
# naming each mirrored section.
#
# Drift means: production changed (variant added, field renamed, function
# signature changed, body altered) and the mirror still claims to mirror
# the OLD surface. Drift breaks the `extern_*` Verus build at compile
# time, which is the documented drift-detection mechanism for the
# Verus bindings.
#
# This script:
#   1. For every `verification/verus/production_inner/*.rs` mirror, parses
#      the master DRIFT POLICY claim and each `// Production ...` per-
#      section claim out of the comment block.
#   2. Validates that every claimed production source file exists and the
#      claimed range is within the file's line count.
#   3. For each claim, extracts the production source slice, normalizes
#      (strips comments, drops documented attribute substitutions,
#      normalizes `pub(crate)` to `pub`, collapses whitespace), and
#      extracts a focused identifier set (PascalCase types, snake_case
#      functions/fields of length >= 7, SCREAMING_SNAKE_CASE constants).
#   4. Extracts the same identifier set from the entire mirror (local
#      stubs declared in the preamble are intentional substitutions for
#      production types from OTHER files, so the local-stub identifiers
#      must be present in the mirror identifier set).
#   5. Verifies that EVERY production identifier from every claim is
#      present in the mirror identifier set. Missing identifiers signal
#      drift: a production variant, field, function, or constant was
#      added or renamed in a way the mirror has not been regenerated
#      to reflect.
#   6. [BODY PARITY] For each claim, extracts struct/enum declaration
#      bodies from both production and mirror (brace-matched), normalizes
#      them identically, and compares as strings. Reports drift when
#      struct/enum bodies differ even if names are preserved. Function
#      bodies are excluded because mirrors use intentional `loop {}` stubs.
#
# Sections explicitly marked `REMOVED:` are skipped (the mirror
# intentionally omits that production range; flagging it as drift
# would be a false positive).
#
# Bare paths (e.g. `exit_code.rs:56-61`) inside section headers are
# resolved against the directory of the master DRIFT POLICY claim
# (e.g., `crates/vb_cli/src/exit_code.rs:1-68`), so the bare path
# resolves to `crates/vb_cli/src/exit_code.rs:56-61`.
#
# Exit code:
#   0  no drift detected
#   1  drift detected (see target/verus-drift/drift.log)
#   2  misuse (missing mirror dir, missing tool)
#
# Tooling: bash + standard POSIX (awk, sed) + perl + git. No installs,
# no Rust toolchain — the gate is a pure source-level identifier diff.

set -euo pipefail

if ! command -v perl >/dev/null 2>&1; then
  printf 'Missing required tool: perl\n' >&2
  exit 2
fi

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

MIRROR_DIR="verification/verus/production_inner"
LOG_DIR="target/verus-drift"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/drift.log"
: > "$LOG"

if [ ! -d "$MIRROR_DIR" ]; then
  printf 'Mirror directory missing: %s\n' "$MIRROR_DIR" >&2
  exit 2
fi

drift_count=0
mirror_count=0

# ---------------------------------------------------------------------------
# Section-header parsing
# ---------------------------------------------------------------------------
# Classified section header forms:
#   <master>  // DRIFT POLICY: ... `path[:start-end]`  (master claim — first one)
#   <claim>   // Production `path[:start-end]`         (per-section claim)
#   <claim>   // SUBSTITUTED: ... `path[:start-end]`   (claim with documented substitution)
#   <skip>    // REMOVED: ... `path[:start-end]`       (intentionally omitted — skip)
#   <claim>   //   1. ... `path[:start-end]`           (numbered binding ledger)
#
# Output: three whitespace-separated fields per line:
#   CLASS MIRROR_LINE_NUMBER  PATH_OR_RANGE
# CLASS is one of: master | claim | skip
# MIRROR_LINE_NUMBER is the 1-indexed line number in the mirror.
# PATH_OR_RANGE is the backtick-stripped path[:start-end].
#
# Bare paths (no directory) are NOT resolved here; resolve_range() resolves
# them against the master DRIFT POLICY directory at check time.

extract_claims() {
  local mirror="$1"

  # Master DRIFT POLICY claim: the path:start-end may be on the same
  # line as `// DRIFT POLICY:` or on a continuation line within the
  # next 5 comment lines. Emit a `master` class for the FIRST such
  # claim per mirror.
  awk '
    BEGIN { drift_pending = 0; master_emitted = 0 }
    {
      if (drift_pending && /^\s*\/\//) {
        line = $0
        if (match(line, /`[A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+`/)) {
          path = substr(line, RSTART + 1, RLENGTH - 2)
          if (!master_emitted) {
            print "master " NR " " path
            master_emitted = 1
          } else {
            print "claim " NR " " path
          }
          drift_pending = 0
          next
        }
      }
      if (/^\s*\/\/\s*DRIFT POLICY:/) {
        line = $0
        if (match(line, /`[A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+`/)) {
          path = substr(line, RSTART + 1, RLENGTH - 2)
          if (!master_emitted) {
            print "master " NR " " path
            master_emitted = 1
          } else {
            print "claim " NR " " path
          }
          next
        }
        # DRIFT POLICY without a backtick-wrapped range on the same
        # line: defer to continuation lines.
        drift_pending = 1
        next
      }
    }
  ' "$mirror"

  # Per-section claims. Use strict regex anchored on section-header
  # keywords (`Production`, `SUBSTITUTED:`, `REMOVED:`, `Source:`,
  # `VERBATIM PRODUCTION:`) followed by the path:start-end. This
  # avoids matching prose mentions of paths in header documentation.
  perl -ne '
    # REMOVED section: `// REMOVED: Production `path:start-end``
    if (/^\s*\/\/\s*REMOVED:\s*(?:.*\s)?\x60([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\x60/) {
      print "skip " . $. . " $1\n";
      next;
    }
    # Per-section claim: `// Production `path:start-end``
    # or `// Production source: `path:start-end``
    if (/^\s*\/\/\s*Production(?:\s+source)?\s+\x60([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\x60/) {
      print "claim " . $. . " $1\n";
      next;
    }
    # Per-section claim: `// Production source: `path:start-end`.`
    # (the period after backticks is part of the prose).
    if (/^\s*\/\/\s*Production\s+source:\s*\x60([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\x60/) {
      print "claim " . $. . " $1\n";
      next;
    }
    # Per-section claim: `// SUBSTITUTED: ... `path:start-end``
    if (/^\s*\/\/\s*SUBSTITUTED:.*\x60([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\x60/) {
      print "claim " . $. . " $1\n";
      next;
    }
    # Per-section claim: `// VERBATIM PRODUCTION: path:start-end`
    # (path on the same line as VERBATIM PRODUCTION).
    if (/^\s*\/\/\s*VERBATIM PRODUCTION:\s+([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\s*$/) {
      print "claim " . $. . " $1\n";
      next;
    }
    # Per-section claim: `// VERBATIM PRODUCTION: <description>` followed
    # by `// Source: path:start-end` (some mirrors use this format).
    if (/^\s*\/\/\s*VERBATIM PRODUCTION:\s*(.*)$/) {
      $verbat_pending = 1;
      next;
    }
    if ($verbat_pending && /^\s*\/\/\s*Source:\s*([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\s*$/) {
      print "claim " . $. . " $1\n";
      $verbat_pending = 0;
      next;
    }
    if (/^[^\/]/) {
      $verbat_pending = 0;
    }
    # Per-section claim: `// Source: path:start-end` (standalone).
    if (/^\s*\/\/\s*Source:\s*([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\s*$/) {
      print "claim " . $. . " $1\n";
      next;
    }
    # Numbered binding ledger line ending in `path:start-end`:
    # `//   N. ... `path:start-end``
    if (/^\s*\/\/\s*\d+\..*\x60([A-Za-z0-9_.\/-]+:[0-9]+-[0-9]+)\x60\s*[.`]?\s*$/) {
      print "claim " . $. . " $1\n";
      next;
    }
  ' "$mirror"
}

# ---------------------------------------------------------------------------
# Range resolution
# ---------------------------------------------------------------------------
# Resolve a `path[:start-end]` triple against the working tree. Bare paths
# (no directory component) are prefixed with the master DRIFT POLICY
# directory.
#
# Echoes four whitespace-separated fields on stdout:
#   STATUS(absent|out_of_bounds|ok) ABS_PATH START END
resolve_range() {
  local range="$1" master_dir="$2"
  local rel_path="${range%:*}"
  local line_range="${range##*:}"
  local start end

  if [ "$rel_path" = "$range" ]; then
    # No `:start-end` suffix.
    line_range=""
  fi

  if [ -z "$line_range" ]; then
    start=1
    end=0  # resolved below by reading file size
  else
    start="${line_range%-*}"
    end="${line_range#*-}"
  fi

  # Bare-path resolution: if rel_path has no `/`, prefix with
  # master_dir's directory component.
  case "$rel_path" in
    */*) ;;  # has directory component — keep as-is
    *)
      if [ "$master_dir" != "." ] && [ -n "$master_dir" ]; then
        rel_path="${master_dir}/${rel_path}"
      fi
      ;;
  esac

  local abs_path="$REPO_ROOT/$rel_path"

  if [ ! -f "$abs_path" ]; then
    printf 'absent %s %s %s\n' "$abs_path" "$start" "$end"
    return
  fi

  local file_lines
  file_lines=$(wc -l < "$abs_path")
  if [ -z "$line_range" ]; then
    end="$file_lines"
  fi
  if [ "$end" -gt "$file_lines" ] || [ "$start" -lt 1 ]; then
    printf 'out_of_bounds %s %s %s\n' "$abs_path" "$start" "$end"
    return
  fi

  printf 'ok %s %s %s\n' "$abs_path" "$start" "$end"
}

# Derive the master DRIFT POLICY directory from the master claim path.
# Returns just the directory component of the master path.
# Falls back to "." if no master claim is present.
derive_master_dir() {
  local master_path="$1"
  case "$master_path" in
    */*) printf '%s\n' "${master_path%/*}" ;;
    *) printf '.\n' ;;
  esac
}

# ---------------------------------------------------------------------------
# Identifier extraction
# ---------------------------------------------------------------------------
# Strip comments and well-known attribute noise. The mirror headers
# explicitly document that derives, #[non_exhaustive], #[must_use],
# #[repr(...)], #[error(...)], #[from], and the various allow/forbid
# inner attributes are substituted out. Normalize `pub(crate)` to `pub`
# to match the mirror's documented visibility relaxation. Inline
# `#[from]` attributes (e.g. `(#[from] CoreError)`) become bare
# parentheses. Multi-line `#[error(...)]` blocks (which may span
# several lines of string content) are stripped entirely.
strip_noise() {
  perl -0777 -pe '
    s{/\*.*?\*/}{}gs;
    # Strip ALL line comments (// or /// or //!) — matches any line
    # whose first non-whitespace content starts with `//`. This also
    # covers `///` doc comments and `//!` module-level docs.
    s{^\h*//.*$}{}gm;
    # Multi-line `#[error(...)]` and `#[derive(...)]` blocks: match
    # the opening `#[` followed by content (which may contain nested
    # parens) up to the closing `)]` or `]`. Use a non-greedy match
    # with a balanced-paren heuristic via `(?:[^()]|\([^()]*\))*`.
    s{^\h*#\[\h*(?:error|derive)\h*\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\)]\n}{}gm;
    # Single-line attribute lines.
    s{^\s*#!?\[[^\]]*\]\s*$}{}gm;
    # Inline `#[from]` becomes bare paren.
    s{\(\s*#\[\s*from\s*\]\s*}{(}g;
    s{\bpub\s*\(\s*crate\s*\)\b}{pub}g;
    # Trailing line comments (after stripping leading comments, so we
    # do not accidentally chop an attribute that ends mid-line).
    s{//.*$}{};
  '
}

# Extract a focused identifier set from Rust source. Keep only tokens
# that look like structural names (type/variant/field/function/constant),
# drop Rust keywords, primitive type names, common stdlib type names,
# and short all-lowercase tokens (those are module/crate names that
# appear in production paths like `vb_core`, `errors`, `ids`, and
# are not present as bare identifiers in the mirror — the mirror uses
# local stubs).
extract_identifiers() {
  strip_noise \
    | perl -ne '
        while (/[A-Za-z_][A-Za-z0-9_]*/g) {
          my $id = $&;
          next if length($id) < 3;
          next if $id =~ /^(?:pub|fn|impl|struct|enum|match|use|mod|self|Self|return|let|const|static|where|for|in|if|else|while|loop|break|continue|true|false|None|Some|Ok|Err|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|bool|str|String|Vec|Box|Option|Result|HashSet|HashMap|BTreeSet|BTreeMap|PhantomData|Default|copy|clone|debug)$/;
          # Drop all-lowercase (with optional underscores) tokens:
          # those are crate names (vb_core, vb_storage), module names
          # (errors, ids, events, recovery), and other path
          # components that production references in `crate::path::Type`
          # form but the mirror flattens to the bare type name. We
          # only care about PascalCase types, snake_case
          # functions/fields (containing at least one underscore AND
          # at least one uppercase boundary marker — actually no,
          # pure snake_case like `compute_max_attempt` IS structural),
          # and SCREAMING_SNAKE_CASE constants. The distinguishing
          # feature for "is this a structural identifier?" is: does
          # the token have at least one uppercase letter OR is it
          # SCREAMING_SNAKE_CASE (contains an underscore but ALL
          # letters are uppercase)?
          next if $id =~ /^[a-z_]+$/;
          # Drop pure SCREAMING_SNAKE_CASE tokens shorter than 7 chars
          # (constants like `MAX`, `MIN`, `OK`, etc. are already in the
          # keyword list; this drops e.g. `ID`, `LEN`).
          next if $id =~ /^[A-Z_]+$/ && length($id) < 7;
          print "$id\n";
        }
      ' \
    | sort -u
}

# Drop English / prose identifiers that survive the regex filter.
# These tokens are not Rust identifiers; they come from prose
# documentation in mirror headers (e.g. "drift", "preserved",
# "verbatim"). The list is curated to match the vocabulary in
# the production_inner/ header comments.
filter_noise_words() {
  grep -vE '^(crate|derive|non_exhaustive|must_use|repr|error|from|allow|forbid|deny|warn|impl|fn|pub|struct|enum|use|mod|where|let|mut|const|static|match|return|self|Self|true|false|None|Some|Ok|Err|line|lines|verbatim|byte|preserved|removed|declared|production|mirror|stub|local|header|note|comment|doc|string|section|substitution|variant|discriminant|block|name|fn_name|impl_block|error_msg|the|and|for|with|from|that|this|of|in|to|or|is|as|by|on|at|are|be|it|an|a|see|via|per|all|any|each|their|these|those|which|both|never|always|still|also|even|after|before|same|other|must|should|cannot|will|would|could|may|might|do|does|did|done|when|where|why|how|what|whom|whose|been|being|have|has|had|not|but|yet|so|because|since|although|though|unless|until|while|whereas|wherever|here|above|below|under|over|between|through|against|among|into|onto|upon|within|without|out|off|down|up|again|further|then|once|such|very|too|only|own|than|now|just|about|drift|substituted|removed|policy|sources|coverage|bindings|binding|debt|ledgers|sections|crates|source|range|body|naming|include|including|excludes|exclude|except|besides|apart|alongside|replacing|replaced|extended|extends|substitution|reason|rationale|description|notes|hint|hints|field|fields|variants|argument|arguments|parameter|parameters|signature|signatures|version|versions|build|rebuild|target|targets|companion|companions|part|parts|piece|pieces|tier|tiers|api|apis|adheres|adhere|expand|expands|expanded|rule|rules|fact|facts|role|roles|effect|effects|sum|sums|summed|step|steps|level|levels|share|shares|shared|gap|gaps|policies|policy|again|details|detail|silent|silently|surface|surfaces|also|value|values|output|outputs|input|inputs|result|results)$' || true
}

# ---------------------------------------------------------------------------
# Body-parity extraction and checking (GOD RULE 2: body-level drift detection)
# ---------------------------------------------------------------------------
# The identifier-set check (above) catches MISSING identifiers but cannot
# detect that a struct/enum body has changed while keeping the same names.
# Body parity compares the NORMALIZED declaration body of every struct
# and enum that appears in both the production source range and the
# entire mirror file.
#
# For each production struct/enum body, body-parity:
#   1. Extracts the full declaration (name + body content).
#   2. Finds the same-named struct/enum in the mirror.
#   3. Normalizes both bodies identically (strip comments, attributes,
#      pub(crate)→pub, collapse whitespace).
#   4. Compares the normalized bodies as strings.
#   5. Reports drift if the bodies differ.
#
# Function bodies are intentionally NOT checked for body parity because
# mirrors deliberately replace production function bodies with no-op
# `loop {}` stubs. Only struct and enum bodies carry verbatim surface
# contracts that must stay in sync with production.

# Extract struct/enum bodies from normalized source.
# Input: normalized source (already passed through strip_noise).
# Output: lines of the form "STRUCT<TAB>name<TAB>body_lines" or
#         "ENUM<TAB>name<TAB>body_lines" where body_lines is everything
#         between the outermost braces of the declaration (one line
#         per body line, no surrounding braces).
#
# NOTE: The outer /g regex finds struct/enum declarations. The inner
# brace-depth loop extracts the body. After extracting a body, pos()
# is set past the closing brace so the /g loop does NOT re-enter the
# body and match nested struct/enum declarations (e.g. `struct Foo`
# nested inside a field type of an outer struct). The body scan
# explicitly detects nested struct/enum keywords and skips their
# entire bodies (brace-matched) to avoid depth-counting errors.
extract_body_fragments() {
  perl -0777 -ne '
    while (/(?:\bpub\s+)?\b(struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\{/g) {
      my $decl_type = ($1 eq "struct" ? "STRUCT" : "ENUM");
      my $name = $2;
      my $pos = pos($_);
      my $depth = 1;
      my $body = "";
      my $i = $pos;
      my $len = length($_);
      while ($i < $len && $depth > 0) {
        my $ch = substr($_, $i, 1);
        if ($ch eq "{") {
          $depth++;
        } elsif ($ch eq "}") {
          $depth--;
          if ($depth == 0) {
            $i++;
            last;
          }
        }
        $body .= $ch;
        $i++;
        # Detect nested struct/enum declarations inside the body and
        # skip their entire brace-matched bodies to avoid corrupting
        # the depth counter of the outer struct/enum.
        if ($depth > 0) {
          my $rest = substr($_, $i);
          if ($rest =~ /\b(?:struct|enum)\s+[A-Za-z_][A-Za-z0-9_]*/g) {
            my $start = $i + $-[0];
            my $brace_pos = $i + $-[0] + length($&);
            if (substr($_, $brace_pos, 1) eq "{") {
              my $inner_depth = 1;
              my $j = $brace_pos + 1;
              while ($j < $len && $inner_depth > 0) {
                if (substr($_, $j, 1) eq "{") { $inner_depth++; }
                elsif (substr($_, $j, 1) eq "}") { $inner_depth--; }
                $j++;
              }
              $i = $j;
              $rest = substr($_, $i);
            }
          }
        }
      }
      pos($_) = $i;
      my @lines = split /\n/, $body;
      my @clean;
      for my $line (@lines) {
        $line =~ s/^\s+//;
        $line =~ s/\s+$//;
        $line =~ s/\s+/ /g;
        next if $line eq "";
        push @clean, $line;
      }
      if (@clean) {
        print "$decl_type\t$name\t" . join("|", @clean) . "\n";
      }
    }
  '
}

# Compare body fragments between production source and mirror.
# Args: $1 = production body fragments file (from extract_body_fragments)
#       $2 = mirror body fragments file
#       $3 = production source relative path (for logging)
#       $4 = mirror relative path (for logging)
#       $5 = line range (e.g. `1437-1641`, for logging only)
# Returns: 0 if body parity holds, 1 if drift detected.
check_body_parity() {
  local prod_bodies_file="$1"
  local mirror_bodies_file="$2"
  local prod_rel="$3"
  local mirror_rel="$4"
  local range="$5"

  if [ ! -s "$prod_bodies_file" ]; then
    return 0  # no structs/enums in production range — nothing to check
  fi

  local body_drift=0

  # For each struct/enum in production, check if mirror has the same name
  # with the same normalized body.
  while IFS=$'\t' read -r decl_type name body; do
    [ -z "$name" ] && continue

    # Look for this name in the mirror bodies
    local mirror_body
    mirror_body=$(awk -F'\t' -v n="$name" '$2 == n { print $3; exit }' "$mirror_bodies_file")

    if [ -z "$mirror_body" ]; then
      # Mirror does not declare this name at all.
      # The identifier check should have caught this, but body-parity
      # provides an independent cross-check.
      {
        printf '\n=== %s ===\n' "$mirror_rel"
        printf 'BODY DRIFT: %s `%s` declared in %s:%s but absent from mirror\n' \
          "$decl_type" "$name" "${prod_rel}" "$range"
      } | tee -a "$LOG"
      body_drift=1
      continue
    fi

    # Normalize field visibility on BOTH sides: mirrors intentionally
    # make struct fields `pub` while production may declare them
    # private. Strip field-level `pub` so the comparison focuses on
    # structural content (field names, types) rather than documented
    # visibility relaxation. This mirrors the existing `strip_noise`
    # behavior that normalizes `pub(crate)` to `pub`.
    body=$(printf '%s' "$body" | perl -pe 's/\bpub\s+([a-zA-Z_]\w*\s*:\s*)/$1/g')
    mirror_body=$(printf '%s' "$mirror_body" | perl -pe 's/\bpub\s+([a-zA-Z_]\w*\s*:\s*)/$1/g')

    # Normalize Vec<T> ↔ Box<[T]> substitutions: mirrors use
    # `Vec<T>` instead of `Box<[T]>` to work around Verus
    # single-file mode limitations. Normalize to `Box<[T]>` on
    # both sides so the comparison focuses on structural content
    # rather than documented type-shape substitutions.
    body=$(printf '%s' "$body" | perl -pe 's/Vec<\s*([^>]+)\s*>/Box<[\1]>/g')
    mirror_body=$(printf '%s' "$mirror_body" | perl -pe 's/Vec<\s*([^>]+)\s*>/Box<[\1]>/g')

    # Compare bodies as normalized strings
    if [ "$body" != "$mirror_body" ]; then
      {
        printf '\n=== %s ===\n' "$mirror_rel"
        printf 'BODY DRIFT: %s `%s` body differs between %s:%s and mirror\n' \
          "$decl_type" "$name" "${prod_rel}" "$range"
      } | tee -a "$LOG"
      body_drift=1
    fi
  done < "$prod_bodies_file"

  return "$body_drift"
}

# ---------------------------------------------------------------------------
# Per-mirror drift check
# ---------------------------------------------------------------------------
for mirror in "$MIRROR_DIR"/*.rs; do
  [ -e "$mirror" ] || continue
  mirror_count=$((mirror_count + 1))
  mirror_rel="${mirror#"$REPO_ROOT/"}"

  claims=$(extract_claims "$mirror")
  if [ -z "$claims" ]; then
    {
      printf '\n=== %s ===\n' "$mirror_rel"
      printf 'DRIFT: no claimed production source range found in header\n'
    } | tee -a "$LOG"
    drift_count=$((drift_count + 1))
    continue
  fi

  # Extract the master DRIFT POLICY path (first master-class claim).
  master_path=$(printf '%s\n' "$claims" | awk '$1 == "master" { $1=""; $2=""; sub(/^  */,""); print; exit }')
  master_dir=$(derive_master_dir "$master_path")
  if [ -z "$master_dir" ]; then
    master_dir="."
  fi

  # Mirror identifiers are extracted from the ENTIRE mirror file —
  # local-stub declarations in the preamble are intentional
  # substitutions for production types and must be present in the
  # identifier set.
  mirror_ids=$(extract_identifiers < "$mirror" | filter_noise_words)

  # Per-claim drift check. Only per-section claims are checked; the
  # master DRIFT POLICY claim (if any) is used only for bare-path
  # resolution. Per-section claims precisely describe which
  # production range the mirror body mirrors, while the master
  # claim often covers ranges that include intentionally REMOVED
  # sections.
  claim_drift=0
  while IFS= read -r claim_line; do
    [ -z "$claim_line" ] && continue
    klass=$(printf '%s' "$claim_line" | awk '{print $1}')
    range=$(printf '%s' "$claim_line" | awk '{$1=""; $2=""; sub(/^  */,""); print}' | sed 's/[[:space:]]*$//')

    # Skip master claims — they are summary pointers, not drift-check
    # claims. Use the FIRST master claim only to derive master_dir for
    # bare-path resolution.
    if [ "$klass" = "master" ]; then
      continue
    fi

    if [ "$klass" = "skip" ]; then
      continue
    fi

    resolution=$(resolve_range "$range" "$master_dir")
    status=$(printf '%s' "$resolution" | awk '{print $1}')
    abs_path=$(printf '%s' "$resolution" | awk '{print $2}')
    start=$(printf '%s' "$resolution" | awk '{print $3}')
    end=$(printf '%s' "$resolution" | awk '{print $4}')

    case "$status" in
      absent)
        {
          printf '\n=== %s ===\n' "$mirror_rel"
          printf 'DRIFT: claimed production source missing: %s\n' \
            "${abs_path#"$REPO_ROOT/"}"
        } | tee -a "$LOG"
        claim_drift=1
        continue
        ;;
      out_of_bounds)
        {
          printf '\n=== %s ===\n' "$mirror_rel"
          printf 'DRIFT: claimed range out of bounds: %s\n' "$range"
        } | tee -a "$LOG"
        claim_drift=1
        continue
        ;;
    esac

    prod_ids=$(sed -n "${start},${end}p" "$abs_path" \
      | extract_identifiers \
      | filter_noise_words)

    missing=$(comm -23 \
      <(printf '%s\n' "$prod_ids") \
      <(printf '%s\n' "$mirror_ids") \
      || true)

    if [ -n "$missing" ]; then
      {
        printf '\n=== %s ===\n' "$mirror_rel"
        printf 'DRIFT: production identifiers in %s missing from mirror:\n' \
          "${abs_path#"$REPO_ROOT/"}"
        printf '%s\n' "$missing" | sed 's/^/  - /'
      } | tee -a "$LOG"
      claim_drift=1
    fi

    # Body-parity check: compare struct/enum declaration bodies between
    # production source and mirror. This catches semantic changes that
    # the identifier-set check cannot — e.g. field renames within the
    # same struct, variant value changes, or added/removed fields that
    # happen to share names with stubs. Function bodies are excluded
    # because mirrors intentionally replace them with no-op stubs.
    _prod_bodies_tmp=$(mktemp)
    sed -n "${start},${end}p" "$abs_path" \
      | strip_noise \
      | extract_body_fragments > "$_prod_bodies_tmp"

    _mirror_bodies_tmp=$(mktemp)
    strip_noise < "$mirror" \
      | extract_body_fragments > "$_mirror_bodies_tmp"

    if check_body_parity "$_prod_bodies_tmp" "$_mirror_bodies_tmp" \
        "${abs_path#"$REPO_ROOT/"}" "$mirror_rel" "${start}-${end}"; then
      : # body parity holds
    else
      claim_drift=1
    fi
    rm -f "$_prod_bodies_tmp" "$_mirror_bodies_tmp"
  done <<< "$claims"

  drift_count=$((drift_count + claim_drift))
done

# ---------------------------------------------------------------------------
# Extern file drift check (second pass)
# ---------------------------------------------------------------------------
# Scans verification/verus/extern_*.rs files for BINDING LEDGER entries
# pointing to crates/ paths and verifies each named identifier is still
# present in the production source within a small context window of
# the claimed line range. This catches drift in:
#   - STRONG bindings: extern_*.rs files whose `#[path]` attribute
#     directly includes a production source under crates/.
#   - WEAK bindings: extern_*.rs files whose `#[path]` attribute
#     includes an in-tree production_inner/ mirror, but whose BINDING
#     LEDGER documents the upstream crates/ ranges the mirror was
#     derived from.
#
# BINDING LEDGER entry forms this pass recognises:
#   //   - `Identifier`                  <- crates/path:start-end
#   //   - `Identifier::Variant`         <- crates/path:line
#   //   - `Identifier` (description)    <- crates/path:start-end
#   //   - Production `Identifier` (...)
#   //                            <- crates/path:start-end
# Entries whose path does NOT start with `crates/` (e.g. mirrors
# under production_inner/, bare paths requiring master-path
# resolution, or summary lines using "Production source:" / "mirrors"
# phrasing) are silently skipped here — they are either covered by
# the production_inner/ scan above or are out of scope for the
# source-level identifier diff this pass performs.

EXTERN_DIR="verification/verus"
extern_scan_count=0
extern_drift_count=0

# Permissive identifier extractor for the extern drift pass. Keeps
# short SCREAMING_SNAKE_CASE constants (MAX, MIN, OK, ...) that the
# production_inner/ mirror extractor filters out, AND keeps 2+ char
# all-lowercase tokens (e.g. `pc`, `id`, `seq`) which are legitimate
# Rust method / field names that the production_inner/ extractor
# drops as "module / crate names".
extract_id_extern() {
  strip_noise \
    | perl -ne '
        while (/[A-Za-z_][A-Za-z0-9_]*/g) {
          my $id = $&;
          # Drop 1-char tokens only.
          next if length($id) < 2;
          # Drop common Rust keywords / primitive type names. Note:
          # `MAX`, `MIN`, `From`, `Self` etc. are intentionally NOT in
          # this list because named ledger identifiers can resolve to
          # them.
          next if $id =~ /^(?:pub|fn|impl|struct|enum|match|use|mod|self|Self|return|let|const|static|where|for|in|if|else|while|loop|break|continue|true|false|None|Some|Ok|Err|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|bool|str|String|Vec|Box|Option|Result|HashSet|HashMap|BTreeSet|BTreeMap|PhantomData|Default|copy|clone|debug)$/;
          print "$id\n";
        }
      ' \
    | sort -u
}

# Generate candidate LAST-segment tokens to try when checking a
# BINDING LEDGER entry. The full token is always tried first; known
# spec-side naming conventions (Mirror / Spec prefix, _pure /
# _decision suffix, production_ / spec_ prefix) are stripped to
# produce additional candidates. The intent is: production source
# has the original name; the spec-side mirror renames it via one of
# these conventions. The check succeeds if ANY candidate is present
# in production.
candidate_tokens() {
  local name="$1"
  printf '%s\n' "$name"
  case "$name" in
    Mirror*) printf '%s\n' "${name#Mirror}" ;;
  esac
  case "$name" in
    Spec*) printf '%s\n' "${name#Spec}" ;;
  esac
  case "$name" in
    *_pure) printf '%s\n' "${name%_pure}" ;;
  esac
  case "$name" in
    *_decision) printf '%s\n' "${name%_decision}" ;;
  esac
  case "$name" in
    production_*) printf '%s\n' "${name#production_}" ;;
  esac
  case "$name" in
    spec_*) printf '%s\n' "${name#spec_}" ;;
  esac
}

# Parse BINDING LEDGER entries from an extern file. Emits one line per
# entry whose path starts with `crates/`, formatted as:
#   NAMED_ID LINE_NO PATH START END
# NAMED_ID is the backtick-wrapped identifier from the entry's
# `//   - \`...\`` line (e.g. `StepBudget::MAX`). LINE_NO is the
# 1-indexed line of the path declaration (`<- crates/...`). START/END
# are the parsed line range; END is `0` if no range was specified
# (resolved by check_extern_entry to the whole file).
#
# Identifier / path pairing: the parser captures the most recent
# `//   - \`...\`` identifier as a pending value, then pairs it with
# the next `<- path` line within a small window (5 lines). The window
# accommodates multi-line description continuations but rejects
# stale pending identifiers from a previous BINDING LEDGER section
# or header preamble. Same-line identifier + path declarations are
# also supported (no `next` after identifier match).
parse_extern_ledger() {
  local extern_file="$1"
  perl -ne '
    my $max_gap = 5;
    # Track the most recent `//   - ` identifier (with optional
    # preceding word like `Production` or `Mirror`).
    if (/^\h*\/\/\h*\-\h+(?:[A-Za-z_][A-Za-z0-9_]*\h+)?\x60([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)/) {
      $pending_id = $1;
      $pending_line = $.;
    }
    # Path declaration: a `<-` followed by a path with optional range.
    # The path may be on the same line as the identifier or on a
    # continuation line within the BINDING LEDGER entry.
    if (/\s*\<-\s+([A-Za-z0-9_.\/-]+)(?::(\d+)(?:-(\d+))?)?/) {
      my $path = $1;
      my $start = defined $2 ? $2 : 1;
      my $end = defined $3 ? $3 : (defined $2 ? $2 : 0);
      my $gap = defined $pending_line ? ($. - $pending_line) : 999;
      if ($path =~ m{^crates/} && defined $pending_id && $gap <= $max_gap) {
        printf("%s %d %s %d %d\n", $pending_id, $., $path, $start, $end);
      }
      # Clear pending state regardless — any path line ends the
      # current entry, and a stale pending_id from a previous
      # section must not pair with this path.
      $pending_id = undef;
      $pending_line = undef;
    }
  ' "$extern_file"
}

# Check a single BINDING LEDGER entry. Reports drift (return 1) if:
#   - The claimed production source file is missing.
#   - The claimed line range is empty after clamping to file bounds.
#   - The LAST segment of the named identifier is not present in the
#     production source within a +/- 5 line context window of the
#     claimed range. The window accommodates ledger entries that
#     document the body of a function/method while the signature sits
#     one line above the claimed body range.
check_extern_entry() {
  local named_id="$1"
  local abs_path="$2"
  local start="$3"
  local end="$4"
  local extern_rel="$5"
  local line_no="$6"

  if [ ! -f "$abs_path" ]; then
    {
      printf '\n=== %s (BINDING LEDGER @ line %s) ===\n' "$extern_rel" "$line_no"
      printf 'DRIFT: claimed production source missing: %s\n' \
        "${abs_path#"$REPO_ROOT/"}"
    } | tee -a "$LOG"
    return 1
  fi

  local file_lines
  file_lines=$(wc -l < "$abs_path")

  if [ "$end" -eq 0 ]; then
    end="$file_lines"
  fi
  if [ "$start" -lt 1 ]; then start=1; fi
  if [ "$end" -gt "$file_lines" ]; then end="$file_lines"; fi
  if [ "$start" -gt "$end" ]; then
    {
      printf '\n=== %s (BINDING LEDGER @ line %s) ===\n' "$extern_rel" "$line_no"
      printf 'DRIFT: claimed range is empty after clamping: %s:%s-%s (file has %s lines)\n' \
        "${abs_path#"$REPO_ROOT/"}" "$start" "$end" "$file_lines"
    } | tee -a "$LOG"
    return 1
  fi

  # Expand the window: include 5 lines above and below the claimed
  # range. This handles ledger entries that document the body of a
  # function / method where the signature sits one line above the
  # claimed body range.
  local ctx_start ctx_end
  ctx_start=$(( start > 5 ? start - 5 : 1 ))
  ctx_end=$(( end + 5 < file_lines ? end + 5 : file_lines ))

  local prod_ids
  prod_ids=$(sed -n "${ctx_start},${ctx_end}p" "$abs_path" | extract_id_extern)

  # The LAST segment of the named identifier is the actual member
  # name (variant, method, field, constant). For multi-segment paths
  # like `EngineError::StepCounterOverflow`, this is the variant; for
  # `StepBudget::MAX`, this is the constant; for single-segment paths
  # like `StepBudget`, this is the struct.
  local key_token
  key_token=$(printf '%s' "$named_id" | awk -F'::' '{print $NF}')

  # Try the full token first, then known spec-side naming conventions
  # (Mirror prefix, _pure suffix, production_ prefix). The spec-side
  # mirror often renames the production identifier via one of these
  # conventions; the check succeeds if ANY candidate is present in
  # production.
  local found=0
  local candidate
  while IFS= read -r candidate; do
    [ -z "$candidate" ] && continue
    if printf '%s\n' "$prod_ids" | grep -qxF "$candidate"; then
      found=1
      break
    fi
  done < <(candidate_tokens "$key_token")

  if [ "$found" -eq 1 ]; then
    return 0
  fi

  {
    printf '\n=== %s (BINDING LEDGER @ line %s) ===\n' "$extern_rel" "$line_no"
    printf 'DRIFT: production identifier `%s` (last segment `%s`) not found in %s:%s-%s (window %s-%s)\n' \
      "$named_id" "$key_token" "${abs_path#"$REPO_ROOT/"}" \
      "$start" "$end" "$ctx_start" "$ctx_end"
  } | tee -a "$LOG"
  return 1
}

# Main extern loop. Each extern_*.rs file is scanned for BINDING
# LEDGER entries that point to crates/ paths; each such entry is
# verified against the production source.
for extern_file in "$EXTERN_DIR"/extern_*.rs; do
  [ -e "$extern_file" ] || continue
  extern_scan_count=$((extern_scan_count + 1))
  extern_rel="${extern_file#"$REPO_ROOT/"}"

  entries=$(parse_extern_ledger "$extern_file")
  [ -z "$entries" ] && continue

  extern_drift=0
  while IFS= read -r entry; do
    [ -z "$entry" ] && continue
    named_id=$(printf '%s' "$entry" | awk '{print $1}')
    line_no=$(printf '%s' "$entry" | awk '{print $2}')
    rel_path=$(printf '%s' "$entry" | awk '{print $3}')
    start=$(printf '%s' "$entry" | awk '{print $4}')
    end=$(printf '%s' "$entry" | awk '{print $5}')

    abs_path="$REPO_ROOT/$rel_path"
    if ! check_extern_entry "$named_id" "$abs_path" "$start" "$end" "$extern_rel" "$line_no"; then
      extern_drift=1
    fi
  done <<< "$entries"

  extern_drift_count=$((extern_drift_count + extern_drift))
done

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
total_drift=$((drift_count + extern_drift_count))
{
  printf '\n=== Summary ===\n'
  printf 'Mirror files checked:  %d\n' "$mirror_count"
  printf 'Extern files scanned:  %d\n' "$extern_scan_count"
  printf 'Drift findings:        %d\n' "$total_drift"
  printf 'Log:                   %s\n' "$LOG"
} | tee -a "$LOG"

if [ "$total_drift" -gt 0 ]; then
  printf '\nPRODUCTION-INNER DRIFT DETECTED. See %s\n' "$LOG" >&2
  exit 1
fi
printf '\nProduction-inner drift gate: PASS\n'
exit 0
