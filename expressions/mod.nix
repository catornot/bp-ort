{
  lib,
  pkgs,
  plugins,
  version,
}:
let
in
pkgs.stdenv.mkDerivation rec {
  pname = "bp_ort";
  inherit version;

  manifest = pkgs.writeText "manifest.json" ''
    {
      "dependencies": [],
      "description": "player bots and other fun stuff",
      "name": "${pname}",
      "version_number": "${version}",
      "website_url": "${meta.homepage}"
    }
  '';

  src = ../.;

  noUnpack = true;
  phases = [ "installPhase" ];
  installPhase = ''
    mkdir -p $out/plugins

    cp ${plugins}/bin/* $out/plugins/
  '';

  meta = {
    description = pname;
    homepage = "https://github.com/catornot/bp-ort";
    license = lib.licenses.asl20;
  };
}
