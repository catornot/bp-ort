{
  lib,
  rustPlatform,
  pkgs,
  rust-bin,
}:
let
  cargoLock = (import ./cargo_lock.nix { });
in
rustPlatform.buildRustPackage (final: {
  name = "bspeater";
  version = "0.1.0";

  rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
  buildInputs = with pkgs; [
    libgcc
    stdenv.cc
    zstd
    libxkbcommon
    vulkan-loader
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    alsa-lib-with-plugins
    wayland
    glfw-wayland
    udev
  ];

  nativeBuildInputs = [
    (rust-bin.fromRustupToolchainFile ../rust-toolchain.toml)
    pkgs.autoPatchelfHook
    pkgs.pkg-config
  ];

  runtimeDependencies = with pkgs; [
    libgcc
    stdenv.cc
    zstd
    libxkbcommon
    vulkan-loader
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    alsa-lib-with-plugins
    wayland
    glfw-wayland
    udev
  ];

  LD_LIBRARY_PATH = lib.makeLibraryPath final.runtimeDependencies;
  PATH = lib.makeLibraryPath final.runtimeDependencies;

  src = ../.;

  meta = {
    description = "A collection of plugins for northstar related to bots";
    homepage = "https://github.com/catornot/bp-ort";
    license = lib.licenses.unlicense;
    mainProgram = "bspeater";
    platforms = [ "x86_64-linux" ];
    maintainers = [ "cat_or_not" ];
  };

  # we need this since bspeater cannot be compiled for windows
  patches = [
    ./only_bspeater.patch
  ];

  inherit cargoLock;
})
