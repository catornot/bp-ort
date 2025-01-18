{
  description = "a collection of plugins for northstar related to bots";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    flake-utils = { url = "github:numtide/flake-utils"; };
  };

  outputs = { self, nixpkgs, flake-utils }: 
     flake-utils.lib.eachDefaultSystem (system:
      let
          pkgs = import nixpkgs {
            inherit system;
            crossSystem = {
              config = "x86_64-w64-mingw32";
              libc = "msvcrt";
            };
          };

          # pkgs = import nixpkgs { localSystem = system; crossSystem = "x86_64-w64-mingw32"; pkgsCross.mingwW64.windows.mcfgthreads.overrideAttrs = { dontDisableStatic = true; }; };
          # pkgs = import nixpkgs { inherit system; };
          # useWin32ThreadModel =
          #   stdenv:
          #   overrideCC stdenv (
          #     stdenv.cc.override (old: {
          #       cc = old.cc.override {
          #         threadsCross = {
          #           model = "win32";
          #           package = null;
          #         };
          #       };
          #     })
          #   );
      in
      {
        devShell = pkgs.mkShell rec {
          # buildInputs = with pkgs; [
          #   # pkgsCross.mingw32.buildPackages.bintools
          #   # pkgsCross.mingw32.buildPackages.libgcc
          #   # pkgsCross.mingw32.buildPackages.libgcrypt
          #   # pkgsCross.mingw32.stdenv.cc
          #   # pkgsCross.mingw32.windows.pthreads
          #   # pkgsCross.mingw32.windows.mingw_w64_headers
          #   # pkgsCross.mingw32.windows.mingw_w64_pthreads
          #   # pkgsCross.mingwW64.buildPackages.bintools
          #   # pkgsCross.mingwW64.buildPackages.libgcc
          #   pkgsCross.mingwW64.buildPackages.libgcrypt
          #   pkgsCross.mingwW64.stdenv.cc
          #   # pkgsCross.mingwW64.windows.pthreads
          #   # pkgsCross.mingwW64.windows.mingw_w64_headers
          #   pkgsCross.mingwW64.windows.mingw_w64_pthreads
          #   # pkgsCross.mingwW64.windows.mingw_runtime
          #   # pkgsCross.mingwW64.windows.mingw_w64
          # ];
          # packages = with pkgs; [
          #   # pkgsCross.mingwW64.buildPackages.pkg-config
          #   pkgsCross.mingwW64.buildPackages.gcc
          #   pkgsCross.mingwW64.buildPackages.lld
          #   # pkgsCross.mingw32.buildPackages.gcc
          #   pkgsCross.mingwW64.buildPackages.clang
          #   # pkgsCross.mingwW64.buildPackages.lldb
            # pkgsCross.mingwW64.buildPackages.mold
          # ];
          nativeBuildInputs = with pkgs; [ 
            pkg-config
            # lld
            # lld_12
            # lld
            # clang
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
      });
}
