{
  plugin,
  version,
  buildType ? "release",
  lib,
  rustPlatform,
  pkgs,
  rust-bin,
}:
let
  cargoLock = (import ./cargo_lock.nix { });
in
rustPlatform.buildRustPackage {
  name = plugin;
  inherit version;

  inherit buildType;
  rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
  buildInputs = [
  ];

  nativeBuildInputs = [
    (rust-bin.fromRustupToolchainFile ../rust-toolchain.toml)
    pkgs.pkg-config
  ];

  src = ../.;

  meta = {
    description = "A collection of plugins for northstar related to bots";
    homepage = "https://github.com/catornot/bp-ort";
    license = lib.licenses.asl20;
    maintainers = [ "cat_or_not" ];
  };

  # we need this since bspeater cannot be compiled for windows
  patches = [
    (pkgs.callPackage ./crate_patch.nix { allowedCrate = plugin; })
  ];

  inherit cargoLock;
}
