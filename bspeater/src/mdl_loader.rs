use anyhow::anyhow;
use bevy::prelude::*;
use bytemuck::offset_of;
use itertools::Itertools;
use std::{mem::size_of, path::PathBuf};

use crate::{Compacttriangle, PhyHeader, PhySection, PhyVertex, StaticProp, Studiohdr};

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
        .map(|name| (std::fs::read(merged_dir.join(&name)), name))
        .map(|(err, name)| {
            if err.is_err() {
                eprintln!("failed to load: {name} because of {err:?}");
                panic!("must load all models");
            }
            err.expect("must load all models")
        })
        .map(|buf| extract_mdl_physics(&buf))
        .map(|model_data| model_data.inspect_err(|err| eprintln!("{err}")).ok())
        .collect::<Vec<Option<(Vec<Vec3>, Vec<u32>)>>>();

    // skip extra data
    let static_props = match extract_static_props(game_lump) {
        Ok(static_props) => static_props,
        Err(e) => {
            eprintln!("error: {e}");
            Vec::new()
        }
    };

    println!(
        "models: {}/{} phys, static_props: {}/{} solid",
        models.iter().flatten().count(),
        models.len(),
        static_props
            .iter()
            .filter_map(|prop| models.get(prop.model_index as usize))
            .flatten()
            .count(),
        static_props.len()
    );

    (static_props, models)
}

fn extract_mdl_physics(buf: &[u8]) -> anyhow::Result<(Vec<Vec3>, Vec<u32>)> {
    let header = {
        let buf: [u8; size_of::<Studiohdr>()] = std::array::from_fn(|i| buf[i]);
        *bytemuck::from_bytes::<Studiohdr>(&buf)
    };

    if header.phy_size == 0 {
        anyhow::bail!("mdl model is malformed with zero physics");
    }

    let phy = buf
        .get(
            header.phy_offset as u64 as usize
                ..(header.phy_offset + header.phy_size) as u64 as usize,
        )
        .ok_or_else(|| anyhow!("didn't fit or smth"))?
        .to_vec();

    let phy_header_offset = size_of::<PhyHeader>();
    let _phy_header = *bytemuck::try_from_bytes::<PhyHeader>(&phy[0..size_of::<PhyHeader>()])
        .expect("phy_header cound't get aquired");
    let section = *bytemuck::try_from_bytes::<PhySection>(
        &phy[phy_header_offset..phy_header_offset + size_of::<PhySection>()],
    )
    .expect("section cound't get aquired");

    let tri_offset = phy_header_offset + offset_of!(section, PhySection, tri);
    let indicies = Vec::from_iter(
        (0..(section.ledge.n_triangles as usize))
            .map(|i| {
                tri_offset + size_of::<Compacttriangle>() * i
                    ..tri_offset + size_of::<Compacttriangle>() * (i + 1)
            })
            .map(|range| {
                *bytemuck::try_from_bytes::<Compacttriangle>(
                    phy.get(range).expect("out of range maybe for compact tri"),
                )
                .expect("couldn't get tri")
            })
            .flat_map(|triangle| {
                [triangle.edge1(), triangle.edge2(), triangle.edge3()]
                    .into_iter()
                    .rev()
            })
            .map(|edge| edge.start_point_index() as u32)
            .collect::<Vec<u32>>(),
    );

    let phys_vertex_offset = phy_header_offset
        + offset_of!(section, PhySection, ledge)
        + section.ledge.c_point_offset as usize;

    let vertices = Vec::from_iter(
        (0..indicies.iter().copied().max().unwrap_or(0) as usize + 1)
            .map(|i| {
                phys_vertex_offset + size_of::<PhyVertex>() * i
                    ..phys_vertex_offset + size_of::<PhyVertex>() * (i + 1)
            })
            .map(|range| {
                *bytemuck::try_from_bytes::<PhyVertex>(
                    phy.get(range).expect("out of range maybe for phyvertex"),
                )
                .expect("couldn't get phy")
            }),
    )
    .iter()
    .map(|vertex| vertex.pos * Vec3::splat(39.3701).with_y(-39.3701))
    .collect_vec();

    // println!("indicies: {}, vertices: {}", indicies.len(), vertices.len());
    Ok((vertices, indicies))
}

fn extract_static_props(
    mut game_lump: impl Iterator<Item = u8>,
) -> Result<Vec<StaticProp>, String> {
    let static_prop_count = dbg!(i32::from_le_bytes(std::array::from_fn(|_| {
        game_lump
            .next()
            .expect("couldn't get expected game lump byte for static props count")
    })) as usize);
    let size = size_of::<StaticProp>();
    let buf = game_lump
        .skip(8) // skip some more stuff
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
        .collect_vec())
}
