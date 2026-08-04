{
  lib,
  rustPlatform,
  pkgs,
  rust-bin,
  version,
  graphical,
  true-graphical ? graphical,
}:
let
  cargoLock = (import ./cargo_lock.nix { });
  graphical = true; # sadly oktree and other stuff still pulls in the renderer (time) to make a custom octtree lib
in
rustPlatform.buildRustPackage rec {
  name = "bspeater";
  inherit version;

  rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
  buildInputs =

    if graphical then
      with pkgs;
      [
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
        glfw
        udev
      ]
    else
      [ ];

  nativeBuildInputs = [
    (rust-bin.fromRustupToolchainFile ../rust-toolchain.toml)
  ]
  ++ lib.optional graphical [
    pkgs.pkg-config
    pkgs.autoPatchelfHook
  ];

  runtimeDependencies =
    if graphical then
      with pkgs;
      [
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
        glfw
        udev
      ]
    else
      [ ];

  cargoFeatures = lib.optional true-graphical [
    "graphics"
  ];

  LD_LIBRARY_PATH = if graphical then lib.makeLibraryPath runtimeDependencies else "";
  PATH = if graphical then lib.makeLibraryPath runtimeDependencies else "";

  src = ../.;

  meta = {
    description = "A collection of plugins for northstar related to bots";
    homepage = "https://github.com/catornot/bp-ort";
    license = lib.licenses.asl20;
    mainProgram = "bspeater";
    # platforms = [ "x86_64-linux" "x86_64-w64-mingw32" ];
    maintainers = [ "cat_or_not" ];
  };

  patches = [
    (pkgs.callPackage ./crate_patch.nix {
      allowedCrate = "bspeater";
      libCrates = [ ];
    })
  ];

  inherit cargoLock;
}
