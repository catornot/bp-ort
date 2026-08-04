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
    mkdir -p $out/mods
    mkdir -p $out/plugins

    cp ${plugins}/bin/* $out/plugins/

    cp -r ${../cat_or_not.BotExtras} $out/mods/cat_or_not.BotExtras
    cp ${../cat_or_not.BotExtras/icon.png} $out/icon.png
    cp ${../README.md} $out/README.md
    cp ${manifest} $out/manifest.json
  '';

  meta = {
    description = pname;
    homepage = "https://github.com/catornot/bp-ort";
    license = lib.licenses.asl20;
  };
}
