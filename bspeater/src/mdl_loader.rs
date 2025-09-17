use std::{mem::size_of, path::PathBuf};

use bevy::prelude::*;
use bytemuck::offset_of;

use crate::{Compacttriangle, PhyHeader, PhySection, PhyVertex, SeekRead, StaticProp, Studiohdr};

pub fn extract_game_lump_models(
    game_lump: Vec<u8>,
    merged_dir: PathBuf,
) -> (Vec<StaticProp>, Vec<Option<(Vec<Vec3>, Vec<u32>)>>) {
    let mut game_lump = game_lump.into_iter().skip(20);

    let model_name_count =
        *bytemuck::try_from_bytes::<i32>(&std::array::from_fn::<_, 4, _>(|_| {
            game_lump
                .next()
                .expect("couldn't get expected game lump byte for model name")
        }))
        .expect("model_name_count couldn't get resolved");
    let models = (0..dbg!(model_name_count))
        .map(|_| {
            std::array::from_fn::<_, 128, _>(|_| {
                game_lump
                    .next()
                    .expect("couldn't get expected game lump byte for model name")
            })
        })
        .filter_map(|name| String::from_utf8(name.to_vec()).ok())
        .map(|name| {
            name.split_once('\0')
                .map(|(left, _)| left.to_owned())
                .unwrap_or(name)
                .to_lowercase()
        })
        .map(|name| (std::fs::File::open(merged_dir.join(&name)), name))
        .map(|(err, name)| {
            if err.is_err() {
                println!("failed to load: {name} because of {err:?}");
                panic!("must load all models");
            }
            err.expect("must load all models")
        })
        .map(|mut buf| extract_mdl_physics(&mut buf))
        .collect::<Vec<Option<(Vec<Vec3>, Vec<u32>)>>>();

    dbg!("what");

    // skip extra data
    let static_props = match extract_static_props(game_lump) {
        Ok(static_props) => static_props,
        Err(e) => {
            eprintln!("error: {e}");
            Vec::new()
        }
    };

    (static_props, models)
}

fn extract_mdl_physics(reader: &mut dyn SeekRead) -> Option<(Vec<Vec3>, Vec<u32>)> {
    // SAFETY: probably safe it's the same size yk
    let header = unsafe {
        let mut buf = [0; size_of::<Studiohdr>()];
        reader.read_exact(&mut buf).ok()?;
        std::mem::transmute::<[u8; size_of::<Studiohdr>()], Studiohdr>(buf)
    };

    // dbg!(String::from_utf8_lossy(&header.name));

    if header.phy_size == 0 {
        println!("mdl model is malformed with zero physics");
        return None;
    }

    // TODO: throw errors
    reader
        .seek(std::io::SeekFrom::Start(header.phy_offset as u64))
        .ok()?;
    // let mut phy = vec![0; header.phy_size as usize];
    let mut phy = vec![0; reader.stream_len().unwrap_or(header.phy_size as u64) as usize];
    reader.read_exact(&mut phy).ok()?;

    // SAFETY: probably not safe but it's almost ok
    let phy_header_offset = size_of::<PhyHeader>();
    let _phy_header = *bytemuck::try_from_bytes::<PhyHeader>(&phy[0..size_of::<PhyHeader>()])
        .expect("phy_header cound't get aquired");
    let section = *bytemuck::try_from_bytes::<PhySection>(
        &phy[phy_header_offset..phy_header_offset + size_of::<PhySection>()],
    )
    .expect("section cound't get aquired");

    let tri_offset = phy_header_offset + offset_of!(section, PhySection, tri);
    let indicies = Vec::from_iter(
        (0..(section.ledge.n_triangles as usize).min(phy.len()))
            .map(|i| {
                tri_offset + size_of::<Compacttriangle>() * i
                    ..tri_offset + size_of::<Compacttriangle>() * (i + 1)
            })
            .filter_map(|range| {
                Some(
                    *bytemuck::try_from_bytes::<Compacttriangle>(phy.get(range)?)
                        .expect("out of range maybe for compact tri"),
                )
            })
            .flat_map(|triangle| [triangle.edge1(), triangle.edge2(), triangle.edge3()])
            .map(|edge| edge.start_point_index() as u32)
            .collect::<Vec<u32>>(),
    );

    let phys_vertex_offset = phy_header_offset
        + offset_of!(section, PhySection, ledge)
        + section.ledge.c_point_offset as usize;
    Some((
        Vec::from_iter(
            (0..indicies
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
                .min(phy.len() as u32) as usize)
                .map(|i| {
                    phys_vertex_offset + size_of::<PhyVertex>() * i
                        ..phys_vertex_offset + size_of::<PhyVertex>() * (i + 1)
                })
                .filter_map(|range| {
                    Some(*bytemuck::try_from_bytes::<PhyVertex>(phy.get(range)?).ok()?)
                }),
        )
        .iter()
        .map(|vertex| vertex.pos * Vec3::splat(39.3701))
        .collect(),
        indicies,
    ))
}

fn extract_static_props(game_lump: impl Iterator<Item = u8>) -> Result<Vec<StaticProp>, String> {
    let mut game_lump = game_lump.skip(8);
    let static_prop_count = dbg!(i32::from_le_bytes(std::array::from_fn(|_| {
        game_lump
            .next()
            .expect("couldn't get expected game lump byte for static props count")
    })) as usize);
    let size = size_of::<StaticProp>();
    let buf = game_lump
        .skip(4) // skip some more stuff
        .collect::<Vec<u8>>()
        .get(0..static_prop_count * size_of::<StaticProp>())
        .expect("expected to have enough bytes for static props")
        .to_vec();
    assert!(buf.len() % size == 0);
    assert!(buf.capacity() % size == 0);

    Ok(buf
        .into_iter()
        .array_chunks::<{ size_of::<StaticProp>() }>()
        .map(|chunk| *bytemuck::from_bytes::<StaticProp>(&chunk))
        .collect())
}
