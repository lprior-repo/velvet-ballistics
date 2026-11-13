#!/usr/bin/env bash

kani_json_helper_build() {
  local script_dir root rustup_bin cargo_bin rustc_bin build_dir helper_bin
  script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
  root="$(cd -- "$script_dir/.." && pwd -P)"
  rustup_bin="${CARGO_HOME:-$HOME/.cargo}/bin/rustup"
  if [ ! -x "$rustup_bin" ]; then
    rustup_bin="$(command -v rustup || true)"
  fi
  if [ -z "$rustup_bin" ] || [ ! -x "$rustup_bin" ]; then
    printf 'rustup is required to resolve the pinned helper toolchain\n' >&2
    return 1
  fi
  cargo_bin="$("$rustup_bin" which --toolchain nightly-2026-04-28 cargo)" || return 1
  rustc_bin="$("$rustup_bin" which --toolchain nightly-2026-04-28 rustc)" || return 1
  build_dir="$root/target/native-tools/kani-json-helper/build"
  helper_bin="$build_dir/debug/kani-json-helper"
  mkdir -p -- "$root/target/native-tools/kani-json-helper/tmp" "$build_dir"
  TMPDIR="$root/target/native-tools/kani-json-helper/tmp" \
    TMP="$root/target/native-tools/kani-json-helper/tmp" \
    TEMP="$root/target/native-tools/kani-json-helper/tmp" CARGO_TARGET_DIR="$build_dir" \
    RUSTC_WRAPPER= \
    RUSTC="$rustc_bin" RUSTUP_TOOLCHAIN=nightly-2026-04-28 \
    "$cargo_bin" build --manifest-path "$root/Cargo.toml" --package velvet-ballistics-workspace-tests \
      --bin kani-json-helper --locked --offline --quiet >&2 || return 1
  printf '%s\n' "$helper_bin"
}
