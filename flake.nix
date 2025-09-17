{
  description = "A collection of plugins for northstar related to bots";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-win.url = "github:nixos/nixpkgs/24.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    catornot-flakes = {
      url = "github:catornot/catornot-flakes";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      nixpkgs-win,
      flake-utils,
      rust-overlay,
      catornot-flakes,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        native-pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        pkgs = import nixpkgs-win {
          inherit system;
          overlays = [ (import rust-overlay) ];
          crossSystem = {
            config = "x86_64-w64-mingw32";
            libc = "msvcrt";
          };
          config.microsoftVisualStudioLicenseAccepted = true;
        };
        toolchain-win = (pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
        toolchain-linux = (
          native-pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml
        );
        # toolchain-linux = native-pkgs.pkgsBuildBuild.rust-bin.stable.latest.default;
      in
      rec {
        formatter = native-pkgs.nixfmt-tree;
        packages = {
          bp-ort = pkgs.callPackage ./expressions/default.nix {
            rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
          };
          packaged-mod = pkgs.callPackage ./expressions/packaged-mod.nix {
            bp-ort = self.packages.${system}.bp-ort;
          };
          bspeater = native-pkgs.callPackage ./expressions/bspeater.nix {
            rust-bin = rust-overlay.lib.mkRustBin { } native-pkgs.buildPackages;
          };
          default = self.packages.${system}.bp-ort;

          tracy = native-pkgs.writeShellApplication {
            name = "tracy";

            runtimeInputs = [
              native-pkgs.tracy
            ];

            text = ''
              capture -o target/trace.tracy
            '';
          };

          tracy-open = native-pkgs.writeShellApplication {
            name = "tracy-open";

            runtimeInputs = [
              native-pkgs.tracy
            ];

            text = ''
              DISPLAY=:0 :w
              tracy target/trace.tracy
            '';
          };

          navmeshes =
            let
              bspeater = self.packages.${system}.bspeater;
              titanfall2 = catornot-flakes.packages.${system}.titanfall2;
              tf2vpk = catornot-flakes.packages.${system}.tf2vpk;
            in
            native-pkgs.callPackage ./expressions/navmeshes.nix { inherit bspeater titanfall2 tf2vpk; };

          win-shell = devShell.default;
          native-shell = devShell.native;
        };

        devShell.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            native-pkgs.bacon
            toolchain-win
            pkg-config
          ];

          buildInputs = with pkgs; [
            windows.mingw_w64_headers
            # windows.mcfgthreads
            windows.pthreads
          ];

          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath buildInputs;
          PATH = nixpkgs.lib.makeLibraryPath buildInputs;
          WINEPATH = nixpkgs.lib.makeLibraryPath buildInputs;
        };

        devShell.native = pkgs.mkShell rec {
          nativeBuildInputs = with native-pkgs; [
            bacon
            toolchain-linux
            clang
            cmake
            cmakeCurses
            pkg-config
          ];

          buildInputs = with native-pkgs; [
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
            pkg-config
          ];

          runtimeDependencies = with native-pkgs; [
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

          LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath runtimeDependencies;
          PATH = nixpkgs.lib.makeLibraryPath runtimeDependencies;

          # adding the export worked!
          shellHook = ''
            export CC=clang
            export CXX=clang++
            export CMAKE=${native-pkgs.cmake}/bin/cmake
            export WGPU_ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER=1
            export WGPU_BACKEND=vulkan
            export RUST_BACKTRACE=1
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
