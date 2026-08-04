{
  description = "A collection of plugins for northstar related to bots";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
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
      flake-utils,
      rust-overlay,
      catornot-flakes,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
          config = {
            allowUnfreePredicate =
              pkg:
              builtins.elem (nixpkgs.lib.getName pkg) [
                "win-sdk"
                "xwin-fetch-msvc"
              ];
            microsoftVisualStudioLicenseAccepted = true;
          };
        };
        pkgs-cross = pkgs.pkgsCross.x86_64-windows;

        toolchain = (pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
      in
      {
        formatter = pkgs.nixfmt-tree;

        packages =
          let
            version = "0.1.7";
          in
          let
            mkPluginBuildType =
              plugin: buildType:
              pkgs-cross.callPackage ./nix/plugins.nix {
                inherit
                  plugin
                  version
                  buildType
                  toolchain
                  ;
                # some stuff needs to be native
                inherit (pkgs) pkg-config llvmPackages cargo-auditable;
              };
            mkPlugin = plugin: mkPluginBuildType plugin "release";
          in
          {
            bp-ort = mkPluginBuildType "bp_ort" "debug";
            ranim = mkPlugin "ranim";
            octbots = mkPlugin "octbots";
            serialized-io = mkPlugin "serialized_io";
            packaged-mod = pkgs.callPackage ./nix/packaged-mod.nix {
              inherit (self.packages.${system}) mod;
              inherit version;
            };
            mod = pkgs.callPackage ./nix/mod.nix {
              plugins = pkgs.symlinkJoin {
                name = "plugins";
                # must have at least one plugin
                paths = with self.packages.${system}; [
                  bp-ort
                  octbots
                  ranim
                  serialized-io
                ];
              };
              inherit version;
            };
            bspeater = pkgs.callPackage ./nix/bspeater.nix {
              inherit version toolchain;
              graphical = false;
            };
            bspeater-graphical = pkgs.callPackage ./nix/bspeater.nix {
              inherit version toolchain;
              graphical = true;
            };
            bspeater-win = pkgs-cross.callPackage ./nix/bspeater.nix {
              inherit version toolchain;
              graphical = false;
            };

            default = self.packages.${system}.mod;

            tracy = pkgs.writeShellApplication {
              name = "tracy";

              runtimeInputs = [
                pkgs.tracy
              ];

              text = ''
                capture -o target/trace.tracy
              '';
            };

            tracy-open = pkgs.writeShellApplication {
              name = "tracy-open";

              runtimeInputs = [
                pkgs.tracy
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
              pkgs.callPackage ./nix/navmeshes.nix { inherit bspeater titanfall2 tf2vpk; };

          };

        devShells = {
          win-shell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              toolchain
              pkg-config
            ];

            buildInputs = with pkgs-cross; [
              windows.sdk
            ];
            shellHook = ''
              SDK=${pkgs-cross.windows.sdk}

              # so some tools in rust expect normal clang => no clang-cl
              export CC_x86_64_pc_windows_msvc=${pkgs.llvmPackages.clang-unwrapped}/bin/clang
              export CXX_x86_64_pc_windows_msvc=${pkgs.llvmPackages.clang-unwrapped}/bin/clang++
              export AR_x86_64_pc_windows_msvc=${pkgs.llvmPackages.bintools-unwrapped}/bin/llvm-lib

              export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=${pkgs.llvmPackages.lld}/bin/lld-link

              export CFLAGS_x86_64_pc_windows_msvc="
                --target=x86_64-windows-msvc
                -fms-compatibility-version=19.11
                -isystem $SDK/crt/include
                -isystem $SDK/sdk/Include/ucrt
                -isystem $SDK/sdk/Include/shared
                -isystem $SDK/sdk/Include/um
                -isystem $SDK/sdk/Include/winrt
              "

              export RUSTFLAGS="
                -C link-arg=/libpath:$SDK/crt/lib/x64
                -C link-arg=/libpath:$SDK/sdk/Lib/ucrt/x64
                -C link-arg=/libpath:$SDK/sdk/Lib/um/x64
              "

              export VCINSTALLDIR=$SDK/crt
            '';
          };

          native-shell = pkgs.mkShell rec {
            nativeBuildInputs = with pkgs; [
              cargo-deny
              cargo-audit
              bacon
              toolchain
              clang
              cmake
              cmakeCurses
              pkg-config
            ];

            buildInputs = with pkgs; [
              stdenv.cc
              zstd
              libxkbcommon
              vulkan-loader
              libx11
              libxcursor
              libxi
              libxrandr
              alsa-lib-with-plugins
              wayland
              glfw
              udev
            ];

            LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath buildInputs;

            # adding the export worked!
            shellHook = ''
              export CC=clang
              export CXX=clang++
              export CMAKE=${pkgs.cmake}/bin/cmake
              export WGPU_ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER=1
              export WGPU_BACKEND=vulkan
              export RUST_BACKTRACE=1
            '';
          };

          wiki-shell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              mdbook
            ];
          };

          default = pkgs.mkShell {
            shellHook = ''
              echo "this flake provdies multiple shells choose one"
              echo "nix develop .#win-shell # provides tooling to build the plugins"
              echo "nix develop .#native-shell # provides tooling to build native tooling"
              echo "nix develop .#wiki-shell # provides to build the wiki"
            '';
          };
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
