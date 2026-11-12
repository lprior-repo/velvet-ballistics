   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.228
   Compiling cfg-if v1.0.4
   Compiling autocfg v1.5.0
   Compiling serde v1.0.228
   Compiling semver v1.0.28
   Compiling thiserror v2.0.18
   Compiling crc32fast v1.5.0
   Compiling libc v0.2.186
   Compiling simd-adler32 v0.3.9
   Compiling find-msvc-tools v0.1.9
   Compiling adler2 v2.0.1
   Compiling shlex v1.3.0
   Compiling arraydeque v0.5.1
   Compiling zmij v1.0.21
   Compiling scopeguard v1.2.0
   Compiling foldhash v0.1.5
   Compiling byteorder v1.5.0
   Compiling anstyle v1.0.14
   Compiling bitflags v2.11.1
   Compiling hashbrown v0.17.1
   Compiling equivalent v1.0.2
   Compiling version_check v0.9.5
   Compiling utf8parse v0.2.2
   Compiling getrandom v0.3.4
   Compiling itoa v1.0.18
   Compiling stable_deref_trait v1.2.1
   Compiling zerocopy v0.8.48
   Compiling serde_json v1.0.149
   Compiling iana-time-zone v0.1.65
   Compiling encoding_rs v0.8.35
   Compiling lock_api v0.4.14
   Compiling pxfm v0.1.29
   Compiling colorchoice v1.0.5
   Compiling constant_time_eq v0.4.2
   Compiling miniz_oxide v0.8.9
   Compiling cc v1.2.62
   Compiling hashbrown v0.15.5
   Compiling hash32 v0.2.1
   Compiling fdeflate v0.3.7
   Compiling anstyle-parse v1.0.0
   Compiling rustix v1.1.4
   Compiling anyhow v1.0.102
   Compiling rustc_version v0.4.1
   Compiling cpufeatures v0.3.0
   Compiling getrandom v0.4.2
   Compiling arrayvec v0.7.6
   Compiling is_terminal_polyfill v1.70.2
   Compiling anstyle-query v1.1.5
   Compiling spin v0.9.8
   Compiling winnow v1.0.2
   Compiling arrayref v0.3.9
   Compiling once_cell v1.21.4
   Compiling memchr v2.8.0
   Compiling bytemuck v1.25.0
   Compiling smallvec v1.15.1
   Compiling clap_lex v1.1.0
   Compiling ahash v0.8.12
   Compiling anstream v1.0.0
   Compiling winnow v0.7.15
   Compiling linux-raw-sys v0.12.1
   Compiling unicode-width v0.2.2
   Compiling toml_writer v1.1.1+spec-1.1.0
   Compiling bitflags v1.3.2
   Compiling num-traits v0.2.19
   Compiling strsim v0.11.1
   Compiling heck v0.5.0
   Compiling byteorder-lite v0.1.0
   Compiling base64 v0.22.1
   Compiling hashlink v0.10.0
   Compiling indexmap v2.14.0
   Compiling unsafe-libyaml v0.2.11
   Compiling heapless v0.7.17
   Compiling crc32c v0.6.8
   Compiling toml_parser v1.1.2+spec-1.1.0
   Compiling flate2 v1.1.9
   Compiling clap_builder v4.6.0
   Compiling annotate-snippets v0.12.16
   Compiling ryu v1.0.23
   Compiling fastrand v2.4.1
   Compiling nohash-hasher v0.2.0
   Compiling syn v2.0.117
   Compiling saphyr-parser v0.0.6
   Compiling encoding_rs_io v0.1.7
   Compiling blake3 v1.8.5
   Compiling png v0.18.1
   Compiling png v0.17.16
   Compiling ordered-float v5.3.0
   Compiling saphyr v0.0.6
warning: blake3@1.8.5: The C compiler "cc" does not support -mavx512f and -mavx512vl.
warning: blake3@1.8.5: c/blake3_sse2_x86-64_unix.S:2292: fatal error: when writing output to /tmp/ccz1zFxr.s: Disk quota exceeded
warning: blake3@1.8.5: compilation terminated.
error: failed to run custom build command for `blake3 v1.8.5`

Caused by:
  process didn't exit successfully: `/home/lewis/src/tmp_build/vb-qi37.4.2-static-target/debug/build/blake3-bbadf17c04aa2097/build-script-build` (exit status: 1)
  --- stdout
  cargo:rustc-check-cfg=cfg(blake3_sse2_ffi, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_sse2_rust, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_sse41_ffi, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_sse41_rust, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_avx2_ffi, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_avx2_rust, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_avx512_ffi, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_neon, values(none()))
  cargo:rustc-check-cfg=cfg(blake3_wasm32_simd, values(none()))
  cargo:rerun-if-env-changed=CARGO_FEATURE_PURE
  cargo:rerun-if-env-changed=CARGO_FEATURE_NO_NEON
  CC_x86_64-unknown-linux-gnu = None
  CC_x86_64_unknown_linux_gnu = None
  HOST_CC = None
  CC = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  CRATE_CC_NO_DEFAULTS = None
  CFLAGS = None
  HOST_CFLAGS = None
  CFLAGS_x86_64_unknown_linux_gnu = None
  CFLAGS_x86_64-unknown-linux-gnu = None
  CC_x86_64-unknown-linux-gnu = None
  CC_x86_64_unknown_linux_gnu = None
  HOST_CC = None
  CC = None
  CRATE_CC_NO_DEFAULTS = None
  CFLAGS = None
  HOST_CFLAGS = None
  CFLAGS_x86_64_unknown_linux_gnu = None
  CFLAGS_x86_64-unknown-linux-gnu = None
  cargo:warning=The C compiler "cc" does not support -mavx512f and -mavx512vl.
  cargo:rerun-if-env-changed=BLAKE3_CI
  cargo:rerun-if-env-changed=CARGO_FEATURE_PREFER_INTRINSICS
  cargo:rerun-if-env-changed=CARGO_FEATURE_PURE
  cargo:rustc-cfg=blake3_sse2_ffi
  cargo:rustc-cfg=blake3_sse41_ffi
  cargo:rustc-cfg=blake3_avx2_ffi
  CC_FORCE_DISABLE = None
  CC_x86_64-unknown-linux-gnu = None
  CC_x86_64_unknown_linux_gnu = None
  HOST_CC = None
  CC = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  CRATE_CC_NO_DEFAULTS = None
  CFLAGS = None
  HOST_CFLAGS = None
  CFLAGS_x86_64_unknown_linux_gnu = None
  CFLAGS_x86_64-unknown-linux-gnu = None
  cargo:warning=c/blake3_sse2_x86-64_unix.S:2292: fatal error: when writing output to /tmp/ccz1zFxr.s: Disk quota exceeded
  cargo:warning=compilation terminated.

  --- stderr


  error occurred in cc-rs: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-fPIC" "-g" "-gdwarf-4" "-fno-omit-frame-pointer" "-m64" "-Wall" "-Wextra" "-std=c11" "-o" "/home/lewis/src/tmp_build/vb-qi37.4.2-static-target/debug/build/blake3-93882288afbcb006/out/b8423798394d5395-blake3_sse2_x86-64_unix.o" "-c" "c/blake3_sse2_x86-64_unix.S"


warning: build failed, waiting for other jobs to finish...

EXIT_STATUS=101
