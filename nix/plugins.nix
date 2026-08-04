{
  plugin,
  version,
  buildType ? "release",
  toolchain,
  lib,
  makeRustPlatform,
  pkg-config,
  cargo-auditable,
  llvmPackages,
  callPackage,
  windows,
}:
let
  cargoLock = (import ./cargo_lock.nix { });
in
(makeRustPlatform {
  cargo = toolchain;
  rustc = toolchain;
  inherit cargo-auditable;
  stdenv = llvmPackages.stdenv;
}).buildRustPackage
  rec {
    name = plugin;
    inherit version;

    src = ../.;

    inherit buildType;
    rustToolchain = toolchain;
    buildInputs = [
      windows.sdk
    ];

    nativeBuildInputs = [
      toolchain
      pkg-config
    ];

    cargoBuildFlags = [
      "--package"
      plugin
    ];

    meta = {
      description = "A collection of plugins for northstar related to bots";
      homepage = "https://github.com/catornot/bp-ort";
      license = lib.licenses.asl20;
      maintainers = [ "cat_or_not" ];
    };

    SDK=windows.sdk;

    # so some tools in rust expect normal clang => no clang-cl
    CC_x86_64_pc_windows_msvc="${llvmPackages.clang-unwrapped}/bin/clang";
    CXX_x86_64_pc_windows_msvc="${llvmPackages.clang-unwrapped}/bin/clang++";
    AR_x86_64_pc_windows_msvc="${llvmPackages.bintools-unwrapped}/bin/llvm-lib";

    CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="${llvmPackages.lld}/bin/lld-link";

    CFLAGS_x86_64_pc_windows_msvc="
      --target=x86_64-windows-msvc
      -fms-compatibility-version=19.11
      -isystem ${SDK}/crt/include
      -isystem ${SDK}/sdk/Include/ucrt
      -isystem ${SDK}/sdk/Include/shared
      -isystem ${SDK}/sdk/Include/um
      -isystem ${SDK}/sdk/Include/winrt
    ";

     RUSTFLAGS="
      -C link-arg=/libpath:$SDK/crt/lib/x64
      -C link-arg=/libpath:$SDK/sdk/Lib/ucrt/x64
      -C link-arg=/libpath:$SDK/sdk/Lib/um/x64
    ";

    VCINSTALLDIR="${SDK}/crt";

    # we need this since bspeater cannot be compiled for windows
    patches = [
      (callPackage ./crate_patch.nix { allowedCrate = plugin; })
    ];

    inherit cargoLock;
  }
