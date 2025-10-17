{
  lib,
  stdenv,
  xvfb-run,
  bspeater,
  titanfall2,
  tf2vpk,
  maps ? [
    "mp_angel_city"
    "mp_black_water_canal"
    "mp_grave"
    "mp_colony02"
    "mp_complex3"
    "mp_crashsite3"
    "mp_drydock"
    "mp_eden"
    "mp_thaw"
    "mp_forwardbase_kodai"
    "mp_glitch"
    "mp_homestead"
    "mp_relic02"
    "mp_rise"
    "mp_wargames"
    # "mp_lobby" # not in mp_lobby vpk lol it's in commmon
    "mp_lf_deck"
    "mp_lf_meadow"
    "mp_lf_stacks"
    "mp_lf_township"
    "mp_lf_traffic"
    "mp_lf_uma"
    "mp_coliseum"
    "mp_coliseum_column"
    # "mp_box"
    # sp maps are absent from my zip rip
    # "sp_training"
    # "sp_crashsite"
    # "sp_sewers1"
    # "sp_boomtown_start"
    # "sp_boomtown"
    # "sp_boomtown_end"
    # "sp_hub_timeshift"
    # "sp_timeshift_spoke02"
    # "sp_beacon"
    # "sp_beacon_spoke0"
    # "sp_tday"
    # "sp_s2s"
    # "sp_skyway_v1"
  ],
}:
let
in
# could split this derivation into a top level one and one that compiles one navmesh to allow for distributed builds and parallel building
stdenv.mkDerivation {
  name = "navmeshes";
  version = "0.0.0";

  src = ../.;

  nativeBuildInputs = [
    bspeater
    tf2vpk
    xvfb-run
  ];

  buildInputs = [
    titanfall2
  ];

  noUnpack = true;
  phases = [ "buildPhase" ];
  buildPhase = (
    lib.concatLines (
      builtins.map (
        name: "xvfb-run bspeater -d ${titanfall2}/vpk -n ${name} -o $out --display -v $TMPDIR"
      ) maps
    )
  );
}
