    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling libc v0.2.186
   Compiling crc32fast v1.5.0
   Compiling simd-adler32 v0.3.9
   Compiling adler2 v2.0.1
   Compiling getrandom v0.3.4
   Compiling zerocopy v0.8.48
   Compiling utf8parse v0.2.2
   Compiling version_check v0.9.5
   Compiling serde_json v1.0.149
   Compiling is_terminal_polyfill v1.70.2
   Compiling pxfm v0.1.29
   Compiling anstyle-query v1.1.5
   Compiling winnow v1.0.2
   Compiling blake3 v1.8.5
   Compiling colorchoice v1.0.5
   Compiling anyhow v1.0.102
   Compiling heck v0.5.0
   Compiling toml_writer v1.1.1+spec-1.1.0
   Compiling byteorder-lite v0.1.0
   Compiling serde_spanned v1.1.1
   Compiling toml_datetime v0.7.5+spec-1.1.0
   Compiling bytemuck v1.25.0
   Compiling bitflags v1.3.2
   Compiling smallvec v1.15.1
   Compiling strsim v0.11.1
   Compiling unicode-width v0.2.2
   Compiling ordered-float v5.3.0
   Compiling winnow v0.7.15
   Compiling encoding_rs_io v0.1.7
   Compiling vb_core v0.1.0 (/home/lewis/src/vb-femdation/vb-qi37-4-2/crates/vb_core)
   Compiling base64 v0.22.1
   Compiling nohash-hasher v0.2.0
   Compiling ryu v1.0.23
warning: blake3@1.8.5: The C compiler "cc" does not support -mavx512f and -mavx512vl.
warning: blake3@1.8.5: sccache: error: failed to execute compile
warning: blake3@1.8.5: sccache: caused by: Compiler not supported: "failed to write temporary file"
error: failed to run custom build command for `blake3 v1.8.5`

Caused by:
  process didn't exit successfully: `/home/lewis/src/vb-femdation/vb-qi37-4-2/target/debug/build/blake3-bbadf17c04aa2097/build-script-build` (exit status: 1)
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
  cargo:warning=sccache: error: failed to execute compile
  cargo:warning=sccache: caused by: Compiler not supported: "failed to write temporary file"

  --- stderr


  error occurred in cc-rs: command did not execute successfully (status code exit status: 2): LC_ALL="C" "sccache" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-fPIC" "-g" "-gdwarf-4" "-fno-omit-frame-pointer" "-m64" "-Wall" "-Wextra" "-std=c11" "-o" "/home/lewis/src/vb-femdation/vb-qi37-4-2/target/debug/build/blake3-93882288afbcb006/out/b8423798394d5395-blake3_sse2_x86-64_unix.o" "-c" "c/blake3_sse2_x86-64_unix.S"


warning: build failed, waiting for other jobs to finish...
error: error writing dependencies to `/tmp/sccachegWVCaG/deps.d`: Disk quota exceeded (os error 122)

error: could not compile `pxfm` (lib) due to 1 previous error
error: error writing dependencies to `/tmp/sccacheurIJrp/deps.d`: Disk quota exceeded (os error 122)

error: could not compile `libc` (lib) due to 1 previous error
error: error writing dependencies to `/tmp/sccacheLeGqfJ/deps.d`: Disk quota exceeded (os error 122)

error: could not compile `zerocopy` (lib) due to 1 previous error

EXIT_STATUS=101
