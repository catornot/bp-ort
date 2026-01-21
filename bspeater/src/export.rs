// sligtly modified version of export.rs from https://github.com/mrclputra/bevy_WeaverGen_V3
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

// saves the model as an obj file
// by iterating through all the meshes

use bevy::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::{EnabledFeatures, ProcessingStep, WorldName};

// export all meshes in scene
pub fn export_obj<'a>(
    mesh_entities: impl Iterator<Item = &'a Mesh>,
    filename: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);

    // OBJ header
    writeln!(writer, "# Exported from BSPEater")?;

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
    features: Res<EnabledFeatures>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    bevy::log::info!("trying to save meshes");

    if !features.no_export_obj {
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
    }

    next_state.set(ProcessingStep::Exit);
}
