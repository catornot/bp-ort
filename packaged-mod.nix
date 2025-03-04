{
  lib,
  pkgs,
  bp-ort,
}:
let
in
  pkgs.stdenv.mkDerivation rec {
    pname = "bp_ort";
    version = "0.1.3";

    manifest = pkgs.writeText "manifest.json" ''
      {
        "dependencies": [],
        "description": "player bots and other fun stuff",
        "name": "${pname}",
        "version_number": "${version}",
        "website_url": "${meta.homepage}"
      }
    '';

    src = ./.;

    nativeBuildInputs = [
    ];
    buildInputs = with pkgs; [
      gmp
    ];
    
    noUnpack = true;
    phases = [ "installPhase" ];
    installPhase = ''
      mkdir -p $out/mods
      mkdir -p $out/plugins
      cp ${bp-ort}/bin/bp_ort.dll $out/plugins/bp_ort.dll
      cp ${bp-ort}/bin/octbots.dll $out/plugins/octbots.dll
      cp ${bp-ort}/bin/ranim.dll $out/plugins/ranim.dll
      cp -r ${./cat_or_not.BotExtras} $out/mods/cat_or_not.BotExtras
      cp ${./cat_or_not.BotExtras/icon.png} $out/icon.png
      cp ${./README.md} $out/README.md
      cp ${manifest} $out/manifest.json
    '';

    meta = {
      description = pname;
      homepage = "https://github.com/catornot/bp-ort";
      license = lib.licenses.unlicense;
    };
  }
