use bevy::prelude::*;
use rkyv::{Archive, Deserialize, Serialize, rancor::Error as RancorError, to_bytes};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(
    // This will generate a PartialEq impl between our unarchived
    // and archived types
    compare(PartialEq),
    // Derives can be passed through to the generated type:
    derive(Debug),
)]
pub struct Navmesh {
    min: [i32; 3],
    max: [i32; 3],
    cell_size: f32,
    filled_pos: Vec<[i32; 3]>,
}

pub fn save_navmesh_to_disk(
    hit_pos: Vec<IVec3>,
    extends: (IVec3, IVec3),
    cell_size: f32,
    map_name: &str,
    output: &Path,
) {
    if let Err(err) = fs::create_dir_all(output) {
        bevy::log::error!(
            "coudln't create ouput dir at {} : {}",
            output.display(),
            err.to_string()
        );
    }

    let path = output.join(map_name).with_extension("navmesh");
    _ = fs::remove_file(&path);
    let mut file = match File::create_new(&path) {
        Err(err) => {
            bevy::log::error!("failed to save navmesh: {err:?}");
            return;
        }
        Ok(file) => file,
    };

    match to_bytes::<RancorError>(&Navmesh {
        min: extends.0.to_array(),
        max: extends.1.to_array(),
        cell_size,
        filled_pos: hit_pos.into_iter().map(|v| v.to_array()).collect(),
    })
    .map_err(|err| err.to_string())
    .and_then(|serialized| {
        file.write_all(serialized.as_slice())
            .map_err(|err| err.to_string())
    }) {
        Ok(_) => bevy::log::info!("saved to {}", path.display()),
        Err(err) => bevy::log::error!("failed to serialize navmesh: {err:?}"),
    }
}
