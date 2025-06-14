{
  lib,
  rustPlatform,
  pkgs,
  rust-bin,
}:
let
in
rustPlatform.buildRustPackage {
  name = "bp-ort";

  buildType = "debug";
  rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  buildInputs = [
  ];

  nativeBuildInputs = [
    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
    pkgs.pkg-config
  ];

  src = ./.;

  meta = {
    description = "A collection of plugins for northstar related to bots";
    homepage = "https://github.com/catornot/bp-ort";
    license = lib.licenses.unlicense;
    mainProgram = "bp-ort";
    # platforms = [ "x86_64-linux" ];
    maintainers = [ "cat_or_not" ];
  };

  # we need this since bspeater cannot be compiled for windows
  patches = [
    ./Cargo.toml.patch
  ];

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "rrplug-4.1.0" = "sha256-4ufvWl0VaHDAoXjNGJ7lBfPLgGLYyRz7E6e4TWED3Ko=";
      "bevy_gltf_export-0.1.0" = "sha256-eIp8TY96HMpmRpRsryaQzZII5/liCLaScF331GQINfY=";
    };
  };
}
