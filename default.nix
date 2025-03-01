{
  lib,
  rustPlatform,
  pkgs,
  rust-bin,
}:
let
in
# why does it need to be so complicated >:(
rustPlatform.buildRustPackage rec {
  name = "bp-ort";

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

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      # "rrplug-4.1.0" = lib.fakeHash;
      "rrplug-4.1.0" = "sha256-4tfmFZifz8fL+Y8qCMm9vZRZnBrpGROe/6/tNNyrycQ=";
    };
  };
}
