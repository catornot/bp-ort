use bevy::prelude::*;
use bincode::Encode;
use std::fs;

#[derive(Encode)]
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
) {
    match bincode::encode_to_vec(
        Navmesh {
            min: extends.0.to_array(),
            max: extends.1.to_array(),
            cell_size,
            filled_pos: hit_pos.into_iter().map(|v| v.to_array()).collect(),
        },
        bincode::config::standard(),
    ) {
        Ok(serialized) => {
            fs::create_dir_all("output");
            let path = format!("output/{map_name}.navmesh");
            if let Err(err) = fs::write(&path, serialized) {
                bevy::log::error!("failed to save navmesh: {err:?}");
            } else {
                bevy::log::info!("saved to {path}");
            }
        }
        Err(err) => bevy::log::error!("failed to serialize navmesh: {err:?}"),
    }
}
