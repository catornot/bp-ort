{
  allowedCrate ? null,
  allowedCrates ? if allowedCrate == null then [ ] else [ allowedCrate ],
  libCrates ? [ "shared" ],
  writeText,
}:
let
  crates =
    if allowedCrate == null && allowedCrates == [ ] then
      throw "must have at least one crate"
    else
      builtins.concatStringsSep ", " (builtins.map (crate: ''"${crate}"'') (allowedCrates ++ libCrates));
in
writeText "cratePatche.patch" ''
  diff --git a/Cargo.toml b/Cargo.toml
  index 32d1d30..89f1661 100644
  --- a/Cargo.toml
  +++ b/Cargo.toml
  @@ -2,7 +2,7 @@
   resolver = "2"
   
   members = [
  -    "bp_ort", "bspeater", "octbots", "r2mole", "ranim", "shared", "serialized_io",
  +    ${crates}
   ]
   
   [workspace.dependencies]
''
