// sligtly modified version of export.rs from https://github.com/mrclputra/bevy_WeaverGen_V3

// saves the model as an obj file
// by iterating through all the meshes

use bevy::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::{ProcessingStep, WorldName};

// export all meshes in scene
pub fn export_obj<'a>(
    mesh_entities: impl Iterator<Item = &'a Mesh>,
    filename: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);

    // OBJ header
    writeln!(writer, "# Exported from Slum Generator")?;
    writeln!(writer, "Written by Marcel Putra 2025")?;

    // OBJ format indices start at 1, dont ask why :)
    let mut vertex_offset = 1;
    let mut mesh_count = 0;

    // export all mesh entities
    for mesh in mesh_entities {
        writeln!(writer, "# Mesh {}", mesh_count)?;
        writeln!(writer, "o Mesh_{}", mesh_count)?;

        // extract vertices from the mesh
        if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            && let bevy::mesh::VertexAttributeValues::Float32x3(vertices) = positions
        {
            // write vertices
            for vertex in vertices {
                writeln!(writer, "v {} {} {}", vertex[0], vertex[1], vertex[2])?;
            }

            // write faces using the mesh indices
            if let Some(indices) = mesh.indices() {
                match indices {
                    bevy::mesh::Indices::U16(indices) => {
                        for chunk in indices.chunks(3) {
                            if chunk.len() == 3 {
                                writeln!(
                                    writer,
                                    "f {} {} {}",
                                    vertex_offset + chunk[0] as u32,
                                    vertex_offset + chunk[1] as u32,
                                    vertex_offset + chunk[2] as u32
                                )?;
                            }
                        }
                    }
                    bevy::mesh::Indices::U32(indices) => {
                        for chunk in indices.chunks(3) {
                            if chunk.len() == 3 {
                                writeln!(
                                    writer,
                                    "f {} {} {}",
                                    vertex_offset + chunk[0],
                                    vertex_offset + chunk[1],
                                    vertex_offset + chunk[2]
                                )?;
                            }
                        }
                    }
                }
            }

            vertex_offset += vertices.len() as u32;
            writeln!(writer)?;
            mesh_count += 1;
        }
    }

    writer.flush()?;
    bevy::log::info!("Exported {} meshes to {}", mesh_count, filename.display());

    Ok(())
}

pub fn save_meshes(
    meshes_assets: Res<Assets<Mesh>>,
    map_name: Res<WorldName>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    bevy::log::info!("trying to save meshes");

    match export_obj(
        meshes_assets.iter().map(|(_, mesh)| mesh),
        &map_name
            .output
            .join(&map_name.map_name)
            .with_extension("obj"),
    ) {
        Err(err) => {
            bevy::log::error!("coudln't save meshes {err:?}");
        }
        Ok(_) => {
            bevy::log::info!("saved meshes");
        }
    }

    next_state.set(ProcessingStep::Exit);
}
