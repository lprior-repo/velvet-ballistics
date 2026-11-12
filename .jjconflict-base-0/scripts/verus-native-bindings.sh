#!/usr/bin/env bash
# Shared native linkage for the existing weak constructor/replay proof bridges.
# This builds real source; it supplies no verifier contracts or trusted bodies.

verus_native_manifest() {
  local root="$1"
  sha256sum "$root/Cargo.toml" "$root/Cargo.lock" "$root/rust-toolchain.toml"
  if [ -f "$root/.cargo/config.toml" ]; then sha256sum "$root/.cargo/config.toml"; fi
  rg --files --null "$root/crates" --glob '*.rs' --glob Cargo.toml \
    | sort -z | xargs -0 sha256sum
}

verus_native_build() {
  local root="$1" evidence="$2" native_toolchain
  mkdir -p -- "$evidence"
  evidence="$(cd -- "$evidence" && pwd -P)"
  VERUS_NATIVE_EVIDENCE="$evidence"
  native_toolchain=$(verus --version | sed -n 's/^  Toolchain: //p')
  if [ -z "$native_toolchain" ] || [[ "$native_toolchain" == *$'\n'* ]]; then
    printf 'Cannot determine the exact Verus Rust toolchain.\n' >&2
    return 1
  fi
  VERUS_NATIVE_TOOLCHAIN="$native_toolchain"
  local native_rustc
  native_rustc=$(rustup which --toolchain "$native_toolchain" rustc) || return 1
  VERUS_NATIVE_RUSTC="$native_rustc"
  "$native_rustc" --version --verbose > "$evidence/compiler.txt" || return 1
  {
    flock --exclusive 9
    verus_native_manifest "$root" > "$evidence/source-before.sha256" || return 1
    # Moon can supply RUSTC independently of the selected Cargo toolchain.
    if ! RUSTC="$native_rustc" RUSTC_WRAPPER= CARGO_TARGET_DIR="$root/target/verus-native-build" \
      rustup run "$native_toolchain" cargo build --manifest-path "$root/Cargo.toml" \
        --locked --offline --lib -p vb_core -p vb_storage --message-format=json \
        > "$evidence/build.jsonl" 2> "$evidence/build.stderr"; then
      cat "$evidence/build.stderr" >&2
      return 1
    fi
    verus_native_manifest "$root" > "$evidence/source-after.sha256" || return 1
    if ! cmp -s "$evidence/source-before.sha256" "$evidence/source-after.sha256"; then
      printf 'Native source changed during compilation; refusing stale binding evidence.\n' >&2
      return 1
    fi
    if ! perl -MJSON::PP -e '
      my ($root, $receipt) = @ARGV;
      open my $file, "<", $receipt or die "$receipt: $!";
      my %found;
      while (<$file>) {
        my $item = decode_json($_);
        next unless $item->{reason} eq "compiler-artifact";
        my $name = $item->{target}{name};
        next unless $name eq "vb_core" || $name eq "vb_storage";
        die "wrong native manifest" unless $item->{manifest_path} eq "$root/crates/$name/Cargo.toml";
        my @libs = grep { /\.rlib$/ } @{$item->{filenames}};
        die "ambiguous native artifact" unless @libs == 1;
        $found{$name} = $libs[0];
      }
      for my $name (qw(vb_core vb_storage)) {
        die "missing artifact $name" unless defined $found{$name};
        print "$name\t$found{$name}\n";
      }
    ' "$root" "$evidence/build.jsonl" > "$evidence/artifacts.tsv"; then
      return 1
    fi
    local name artifact core_artifact='' storage_artifact=''
    while IFS=$'\t' read -r name artifact; do
      if [[ "$artifact" != "$root/target/verus-native-build/"* ]] || [ ! -s "$artifact" ]; then
        printf 'Native artifact is absent or outside the controlled target directory.\n' >&2
        return 1
      fi
      case "$name" in
        vb_core) core_artifact="$artifact" ;;
        vb_storage) storage_artifact="$artifact" ;;
      esac
    done < "$evidence/artifacts.tsv"
    if [ -z "$core_artifact" ] || [ -z "$storage_artifact" ]; then
      printf 'Incomplete native artifact receipt.\n' >&2
      return 1
    fi
    sha256sum "$core_artifact" "$storage_artifact" > "$evidence/artifacts.sha256" || return 1
    local native_dep_dir
    native_dep_dir="$(dirname -- "$core_artifact")"
    if [ -d "$native_dep_dir/deps" ]; then native_dep_dir="$native_dep_dir/deps"; fi
    VERUS_NATIVE_ARGS=(--edition=2024 --extern "vb_core=$core_artifact"
      --extern "vb_storage=$storage_artifact" -L "dependency=$native_dep_dir")
    printf 'NATIVE_BINDING_BUILD_OK toolchain=%s receipt=%s\n' "$native_toolchain" "$evidence/build.jsonl"
  } 9> "$evidence/build.lock"
}

verus_native_check_facade() {
  local root="$1" mirror="$2" code
  if [ "$mirror" != "$root/verification/verus/production_inner/try_from_parts_production.rs" ]; then
    printf 'Unexpected native facade path.\n' >&2
    return 1
  fi
  # A native facade cannot declare a local type, wrapper, cfg fork, or validator.
  code=$(sed '/^[[:space:]]*\/\//d; /^[[:space:]]*$/d' "$mirror" | tr -d '[:space:]')
  if [ "$code" != 'pubusevb_core::*;pubusevb_core::workflow::*;' ]; then
    printf 'Native constructor facade contains code other than the two exact production reexports.\n' >&2
    return 1
  fi
  if [[ "$mirror" == *'"'* ]] || [[ "$mirror" == *'\'* ]]; then
    printf 'Unsupported native facade path quoting.\n' >&2
    return 1
  fi
  local probe="$VERUS_NATIVE_EVIDENCE/constructor-identity.rs"
  cat > "$probe" <<RUST
#[path = "$mirror"]
pub mod facade;
pub fn same_parts(value: vb_core::WorkflowParts) -> facade::WorkflowParts { value }
pub fn same_workflow(value: vb_core::CompiledWorkflow) -> facade::CompiledWorkflow { value }
pub fn same_error(value: vb_core::WorkflowError) -> facade::WorkflowError { value }
fn main() {
    let bound: fn(facade::WorkflowParts) -> Result<facade::CompiledWorkflow, facade::WorkflowError> = facade::CompiledWorkflow::try_from_parts;
    let actual: fn(vb_core::WorkflowParts) -> Result<vb_core::CompiledWorkflow, vb_core::WorkflowError> = vb_core::CompiledWorkflow::try_from_parts;
    if !std::ptr::fn_addr_eq(bound, actual) { std::process::exit(1); }
}
RUST
  if ! "$VERUS_NATIVE_RUSTC" --deny warnings \
    "${VERUS_NATIVE_ARGS[@]}" "$probe" -o "$VERUS_NATIVE_EVIDENCE/constructor-identity" \
    > "$VERUS_NATIVE_EVIDENCE/constructor-identity.build.log" 2>&1; then
    cat "$VERUS_NATIVE_EVIDENCE/constructor-identity.build.log" >&2
    return 1
  fi
  "$VERUS_NATIVE_EVIDENCE/constructor-identity" || return 1
  printf 'NATIVE_CONSTRUCTOR_IDENTITY_OK actual types and actual function\n'
}

verus_native_claim() {
  local source="$1" start="$2" end="$3"
  perl -0777 -e '
    my ($path, $start, $end) = @ARGV;
    open my $file, "<", $path or die "$path: $!";
    local $/; my $code = <$file>;
    my $mask = $code;
    $mask =~ s{"(?:\\.|[^"\\])*"}{my $s=$&; $s =~ s/[^\n]/ /g; $s}gse;
    $mask =~ s{//[^\n]*|/\*.*?\*/}{my $s=$&; $s =~ s/[^\n]/ /g; $s}gse;
    for my $item (["fn", "try_from_parts"], ["struct", "WorkflowParts"], ["enum", "WorkflowError"]) {
      my ($kind, $name) = @$item;
      next unless $mask =~ /\b\Q$kind\E\s+\Q$name\E\b/g;
      my $begin = $-[0];
      my $brace = index($mask, "{", $+[0]);
      die "missing body" if $brace < 0;
      my $depth = 0; my $finish;
      for (my $i=$brace; $i<length($mask); $i++) {
        my $c = substr($mask,$i,1);
        $depth++ if $c eq "{";
        if ($c eq "}") { $depth--; if ($depth == 0) { $finish=$i; last; } }
      }
      die "unbalanced body" unless defined $finish;
      my $a = 1 + (substr($code,0,$begin) =~ tr/\n//);
      my $b = 1 + (substr($code,0,$finish) =~ tr/\n//);
      if ($a == $start && $b == $end) { print "$name\n"; exit 0; }
      pos($mask) = 0;
    }
    die "Native claim must cover one complete current constructor, parts struct, or error enum\n";
  ' "$source" "$start" "$end"
}
