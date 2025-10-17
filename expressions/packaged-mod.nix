{
  lib,
  pkgs,
  mod,
  version,
}:
let
in
pkgs.stdenv.mkDerivation rec {
  pname = "${mod.pname}-packaged";
  inherit version;

  src = mod;

  nativeBuildInputs = with pkgs; [
    zip
  ];

  noUnpack = true;
  phases = [ "installPhase" ];
  installPhase = ''
    mkdir -p $TMP/mod
    mkdir -p $out

    cp -r $src/* $TMP/mod
    cd $TMP/mod && zip -r $out/${pname}.zip *
  '';

  meta = mod.meta;
}
