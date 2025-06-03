use bevy::prelude::*;

use crate::{
    Compactledge, PhyHeader, PhySection, PhyVertex, SeekRead, StaticProp, Studiohdr, UNPACK_MERGED,
};

pub fn extract_game_lump_models(
    game_lump: Vec<u8>,
) -> (Vec<StaticProp>, Vec<Option<(Vec<Vec3>, Vec<u32>)>>) {
    let mut game_lump = game_lump.into_iter().skip(20);

    let model_name_count = i32::from_le_bytes(std::array::from_fn(|_| {
        game_lump
            .next()
            .expect("couldn't get expect game lump byte")
    }));
    let models = (0..dbg!(model_name_count))
        .map(|_| {
            std::array::from_fn::<_, 128, _>(|_| {
                game_lump
                    .next()
                    .expect("couldn't get expected game lump byte for model name")
            })
        })
        .map(|name| String::from_utf8(name.to_vec()).unwrap())
        .map(|name| {
            name.split_once('\0')
                .map(|(left, _)| left.to_owned())
                .unwrap_or(name)
                .to_lowercase()
        })
        .map(|name| (std::fs::File::open(format!("{UNPACK_MERGED}/{name}")), name))
        .map(|(err, name)| {
            if err.is_err() {
                println!("failed to load: {name} because of {err:?}");
                panic!("must load all models");
            }
            err.expect("must load all models")
        })
        .map(|mut buf| extract_mdl_physics(&mut buf))
        .collect::<Vec<Option<(Vec<Vec3>, Vec<u32>)>>>();
    // skip extra data
    let static_props = extract_static_props(game_lump);

    (static_props, models)
}

fn extract_mdl_physics(mut reader: &mut dyn SeekRead) -> Option<(Vec<Vec3>, Vec<u32>)> {
    // SAFETY: probably safe it's the same size yk
    let header = unsafe {
        let mut buf = [0; std::mem::size_of::<Studiohdr>()];
        let mut header_drain = reader.read_exact(&mut buf);
        std::mem::transmute::<[u8; 724], Studiohdr>(buf)
    };

    // dbg!(String::from_utf8_lossy(&header.name));

    if header.phy_size == 0 {
        println!("mdl model is malformed with zero physics");
        return None;
    }

    reader.seek(std::io::SeekFrom::Start(header.phy_offset as u64));
    let mut phy = vec![0; header.phy_size as usize];
    reader.read_to_end(&mut phy);

    // SAFETY: probably not safe but it's almost ok
    unsafe {
        let phy_header = (*phy.as_ptr().cast::<PhyHeader>().as_ref().expect("how"));
        let section = (phy
            .as_ptr()
            .byte_offset(std::mem::size_of::<PhyHeader>() as isize)
            .cast::<PhySection>()
            .as_ref()
            .expect("how"));

        let indicies = std::slice::from_raw_parts(&section.tri, section.ledge.n_triangles as usize)
            .iter()
            .flat_map(|triangle| [triangle.edge1(), triangle.edge2(), triangle.edge3()])
            .map(|edge| edge.start_point_index() as u32)
            .collect::<Vec<u32>>();

        Some((
            std::slice::from_raw_parts(
                (&section.ledge as *const Compactledge)
                    .byte_offset(section.ledge.c_point_offset as isize)
                    .cast::<PhyVertex>(),
                indicies.iter().copied().max().unwrap_or(0) as usize,
            )
            .iter()
            .map(|vertex| vertex.pos * Vec3::splat(39.3701))
            .collect(),
            indicies,
        ))
    }
}

fn extract_static_props(game_lump: impl Iterator<Item = u8>) -> Vec<StaticProp> {
    let mut game_lump = game_lump.skip(8);
    let static_prop_count = dbg!(i32::from_le_bytes(std::array::from_fn(|_| {
        game_lump
            .next()
            .expect("couldn't get expected game lump byte for static props count")
    })) as usize);
    let size = std::mem::size_of::<StaticProp>();
    let mut buf = game_lump
        .skip(4) // skip some more stuff
        .collect::<Vec<u8>>()
        .get(0..static_prop_count * std::mem::size_of::<StaticProp>())
        .expect("expected to have enough bytes for static props")
        .to_vec();
    assert!(buf.len() % size == 0);
    assert!(buf.capacity() % size == 0);
    let static_props = unsafe {
        Vec::from_raw_parts(
            buf.as_mut_ptr().cast::<StaticProp>(),
            buf.len() / size,
            buf.capacity() / size,
        )
    };
    std::mem::forget(buf);
    static_props
}
