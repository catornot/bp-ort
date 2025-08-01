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
        overrides = (builtins.fromTOML (builtins.readFile (self + "/rust-toolchain.toml")));
        toolchain-win = (pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
        toolchain-linux = (native-pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
      in
      rec {
        formatter = native-pkgs.nixfmt-rfc-style;
        packages = {
          bp-ort = pkgs.callPackage ./default.nix {
            rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
          };
          packaged-mod = pkgs.callPackage ./packaged-mod.nix { bp-ort = self.packages.${system}.bp-ort; };
          default = self.packages.${system}.bp-ort;

          win-shell = devShell.default;
          native-shell = devShell.native;
        };

        devShell.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            toolchain-win
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

        devShell.native = pkgs.mkShell rec {
          nativeBuildInputs = with native-pkgs; [
            toolchain-linux
            clang
            cmake
            cmakeCurses
            pkg-config
          ];

          buildInputs = with native-pkgs; [
            zstd
            libxkbcommon
            vulkan-loader
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            alsa-lib
            wayland
            glfw-wayland
            udev
          ];

          runtimeDependencies = with native-pkgs; [
            libxkbcommon
            libgcc
            glibc.out
            vulkan-loader
            alsa-lib
            udev
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            wayland
            glfw-wayland
          ];

          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath runtimeDependencies;
          PATH = nixpkgs.lib.makeLibraryPath runtimeDependencies;

          # adding the export worked!
          shellHook = ''
            export CC=clang
            export CXX=clang++
            export CMAKE=${native-pkgs.cmake}/bin/cmake
            export WGPU_ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER=1
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
