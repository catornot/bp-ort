use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

fn main() {
    let mut default = DefaultHasher::new();

    let to_hash = rustc_version::version()
        .inspect_err(|err| eprintln!("rust version {err:?}"))
        .map_err(|err| err.to_string())
        .map(|version| version.to_string());
    to_hash.hash(&mut default);

    let dest_path = Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("rustc");
    fs::write(&dest_path, default.finish().to_string()).unwrap();
}
