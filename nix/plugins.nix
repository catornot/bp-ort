{
  plugin,
  version,
  buildType ? "release",
  toolchain,
  lib,
  makeRustPlatform,
  callPackage,
}:
let
  cargoLock = (import ./cargo_lock.nix { });
in
(makeRustPlatform {
  cargo = toolchain;
  rustc = toolchain;
}).buildRustPackage
  {
    name = plugin;
    inherit version;

    src = ../.;

    inherit buildType;
    rustToolchain = toolchain;
    buildInputs = [
    ];

    nativeBuildInputs = [
      toolchain
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

    # we need this since bspeater cannot be compiled for windows
    patches = [
      (callPackage ./crate_patch.nix { allowedCrate = plugin; })
    ];

    inherit cargoLock;
  }
