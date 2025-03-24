{
  description = "A collection of plugins for northstar related to bots";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        native-pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
          crossSystem = {
            config = "x86_64-w64-mingw32";
            libc = "msvcrt";
          };
        };
      in
      rec {
        formatter = native-pkgs.nixfmt-rfc-style;
        packages = {
          bp-ort = pkgs.callPackage ./default.nix {
            rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
          };
          packaged-mod = pkgs.callPackage ./packaged-mod.nix { bp-ort = self.packages.${system}.bp-ort; };
          default = self.packages.${system}.bp-ort;

          default-shell = devShell.default;
          run-shell = devShell.run;
        };

        devShell.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            windows.mingw_w64_headers
            windows.mcfgthreads
            windows.mingw_w64_pthreads
          ];

          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath buildInputs;
          PATH = nixpkgs.lib.makeLibraryPath buildInputs;
          WINEPATH = nixpkgs.lib.makeLibraryPath buildInputs;
        };

        devShell.run = pkgs.mkShell rec {
          nativeBuildInputs = with native-pkgs; [
            clang
            cmake
            cmakeCurses
            pkg-config
          ];

          buildInputs = with native-pkgs; [
            libgcc
            glibc.out
          ];
          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath buildInputs;
          PATH = nixpkgs.lib.makeLibraryPath buildInputs;
          WINEPATH = nixpkgs.lib.makeLibraryPath buildInputs;

          # adding the export worked!
          shellHook = ''
            echo "hi"
            export CC=clang
            export CXX=clang++
            export CMAKE=${native-pkgs.cmake}/bin/cmake
          '';
        };

        nix.settings = {
          substituters = [
            "https://cache.nixos.org/"
            "https://nix-community.cachix.org"
          ];
          trusted-public-keys = [
            "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
          ];
        };
      }
    );
}
