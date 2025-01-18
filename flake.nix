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
      in
      {
        devShell = pkgs.mkShell rec {
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
      });
}
