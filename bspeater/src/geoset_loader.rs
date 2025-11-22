use crate::*;
use bevy::math::{Affine3A, DVec3};
use rayon::prelude::*;

pub fn geoset_to_meshes(
    BSPData {
        vertices,
        tricoll_headers,
        tricoll_triangles,
        geo_sets,
        col_primatives,
        unique_contents,
        brushes,
        brush_side_plane_offsets,
        brush_planes,
        grid,
        props,
        model_data,
    }: BSPData,
) -> Vec<Mesh> {
    geo_sets
        .into_iter()
        .flat_map(|geoset| {
            col_primatives
                .get(((geoset.prim_start >> 8) & 0x1FFFFF) as usize..)
                .unwrap_or(&[])
                .iter()
                .take((geoset.prim_count.eq(&1).not() as usize) * geoset.prim_count as usize)
                .map(|col_primative| col_primative.val)
                .chain(geoset.prim_count.eq(&1).then_some(geoset.prim_start))
        })
        .filter_map(|primative| {
            let flag = Contents::SOLID as i32 | Contents::PLAYER_CLIP as i32;
            let no_flag = Contents::WINDOW_NO_COLLIDE as i32;
            // if it doesn't contain any
            if unique_contents[primative as usize & 0xFF] & flag == 0
                || unique_contents[primative as usize & 0xFF] & no_flag != 0
            {
                None
            } else {
                Some((
                    PrimitiveType::try_from((primative >> 29) & 0x7)
                        .expect("invalid primative type"),
                    ((primative >> 8) & 0x1FFFFF) as usize,
                    unique_contents[primative as usize & 0xFF],
                ))
            }
        })
        .collect::<std::collections::HashSet<(PrimitiveType, usize, i32)>>()
        // maybe this doesn't improve anything but it's cool
        .into_par_iter()
        .filter_map(|(ty, index, contents)| {
            let mut pushing_vertices: Vec<Vec3> = Vec::new();
            let mut indices = Vec::new();

            match ty {
                PrimitiveType::Tricoll => tricoll_to_mesh(
                    &tricoll_headers[index],
                    &vertices,
                    &tricoll_triangles,
                    &mut pushing_vertices,
                    &mut indices,
                ),
                PrimitiveType::Brush => brush_to_mesh(
                    &brushes[index],
                    &brush_side_plane_offsets,
                    &brush_planes,
                    grid,
                    &mut pushing_vertices,
                    &mut indices,
                )?,
                PrimitiveType::Prop => prop_to_mesh(
                    &props,
                    &model_data,
                    index,
                    &mut pushing_vertices,
                    &mut indices,
                )?,
            }

            Some(
                Mesh::new(
                    bevy::render::mesh::PrimitiveTopology::TriangleList,
                    RenderAssetUsages::all(),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    pushing_vertices.iter().map(|_| Vec3::ZERO).collect_vec(),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    pushing_vertices.iter().map(|_| Vec2::ONE).collect_vec(),
                )
                .with_inserted_attribute(
                    ATTRIBUTE_UNIQUE_CONTENTS,
                    vec![contents; pushing_vertices.len()],
                )
                .with_inserted_attribute(
                    ATTRIBUTE_PRIMATIVE_TYPE,
                    vec![ty as u32; pushing_vertices.len()],
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pushing_vertices)
                .with_inserted_indices(bevy::render::mesh::Indices::U32(indices)),
            )
        })
        .collect::<Vec<Mesh>>()
}

fn brush_to_mesh(
    brush: &Brush,
    brush_side_plane_offsets: &[u16],
    brush_planes: &[Vec4],
    grid: CMGrid,
    pushing_vertices: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
) -> Option<()> {
    let planes = (0..brush.num_plane_offsets as usize)
        .map(|i| {
            grid.base_plane_offset as usize + i + brush.brush_side_offset as usize
                - brush_side_plane_offsets[brush.brush_side_offset as usize + i] as usize
        })
        .map(|index| brush_planes[index])
        .collect::<Vec<_>>();

    #[rustfmt::skip] let extend_planes =[
        Vec4::new(1., 0., 0., brush.extends.x.abs()),
        Vec4::new(-1., 0., 0., brush.extends.x.abs()),
        Vec4::new(0., 1., 0., brush.extends.y.abs()),
        Vec4::new(0., -1., 0., brush.extends.y.abs()),
        Vec4::new(0., 0., 1., brush.extends.z.abs()),
        Vec4::new(0., 0., -1., brush.extends.z.abs()),
    ];

    let transform = Transform::from_translation(brush.origin)
        .compute_matrix()
        .inverse()
        .transpose();
    let planes = planes
        .into_iter()
        .map(|vec4| transform.mul_vec4(vec4))
        .chain(extend_planes)
        .collect_vec();

    let points = &planes
        .iter()
        // .filter(|plane| plane.w != 0.) // hmm idk
        .tuple_combinations()
        .flat_map(|(p1, p2, p3)| {
            let intersection = calculate_intersection_point([p1, p2, p3])?;
            // If the intersection does not exist within the bounds the hull, discard it
            if !contains_point(&planes, intersection) {
                return None;
            }

            Some(intersection)
        })
        .map(|v| (v.as_vec3() + brush.origin).xzy())
        .map(|v| v.into())
        .collect::<Vec<_>>();

    let (vertices, pindices) = avian3d::parry::transformation::try_convex_hull(points).ok()?;

    pushing_vertices.extend(vertices.iter().map(|v| Vec3::new(v.x, v.y, v.z)));
    indices.extend(pindices.iter().flatten());

    Some(())
}

fn prop_to_mesh(
    props: &[StaticProp],
    model_data: &[Option<(Vec<Vec3>, Vec<u32>)>],
    index: usize,
    pushing_vertices: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
) -> Option<()> {
    if props.len() <= index {
        return None;
    }

    let static_prop = props[index];
    let transform = Transform::from_translation(static_prop.origin.xzy())
        .with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            static_prop.angles.x.to_radians(),
            static_prop.angles.y.to_radians(),
            static_prop.angles.z.to_radians(),
        ))
        .with_scale(Vec3::splat(static_prop.scale))
        // .looking_to(Vec3::new(0.5, 0.0, 1.0), Vec3::new(0., 1., 0.))
        .compute_affine();

    let transform =
        new_source_transform_matrix(static_prop.origin, static_prop.angles, static_prop.scale);

    if let Some(model_data) = model_data
        .get(static_prop.model_index as usize)
        .and_then(|o| o.as_ref())
        .filter(|_| static_prop.solid == 6)
    // figure what this actually is ^ rigth vphysics stuff I rember
    {
        indices.extend(&model_data.1);
        pushing_vertices.extend(
            model_data
                .0
                .iter()
                .copied()
                .map(|vert| transform.mul_vec4(vert.extend(1.)).truncate().xzy()),
        );
    } else {
        // println!("no phys model");
    }

    Some(())
}

fn new_source_transform_matrix(origin: Vec3, angles: Vec3, scale: f32) -> Mat4 {
    let sy = angles.y.to_radians().sin();
    let sp = angles.x.to_radians().sin();
    let sr = angles.z.to_radians().sin();
    let cy = angles.y.to_radians().cos();
    let cp = angles.x.to_radians().cos();
    let cr = angles.z.to_radians().cos();
    Mat4::from_cols(
        Vec4::new(cp * cy * scale, cp * sy * scale, -sp * scale, 0.),
        Vec4::new(
            (sp * sr * cy - cr * sy) * scale,
            (sp * sr * sy + cr * cy) * scale,
            sr * cp * scale,
            0.,
        ),
        Vec4::new(
            (sp * cr * cy + sr * sy) * scale,
            (sp * cr * sy - sr * cy) * scale,
            cr * cp * scale,
            0.,
        ),
        Vec4::new(origin.x, origin.y, origin.z, 0.),
    )
}

fn model_to_mesh(
    model_data: &(Vec<Vec3>, Vec<u32>),
    pushing_vertices: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
    transform: Affine3A,
) {
    indices.extend(&model_data.1);
    pushing_vertices.extend(
        model_data
            .0
            .iter()
            .copied()
            .map(|vert| transform.transform_point3(vert)),
    );
}

fn tricoll_to_mesh(
    tricoll_header: &TricollHeader,
    vertices: &[Vec3],
    tricoll_triangles: &[TricollTri],
    pushing_vertices: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
) {
    let verts = &vertices[tricoll_header.first_vertex as usize..];
    let triangles_base = &tricoll_triangles[tricoll_header.first_triangle as usize..];
    for triangle in triangles_base
        .iter()
        .take(tricoll_header.num_triangles as usize)
        .map(|triangle| triangle.data)
    {
        let vert0 = triangle & 0x3FF;
        let vert1 = vert0 + ((triangle >> 10) & 0x7F);
        let vert2 = vert0 + ((triangle >> 17) & 0x7F);

        for vert_pos in [vert0, vert1, vert2].map(|vert| verts[vert as usize].xzy()) {
            pushing_vertices.push(vert_pos);

            indices.push(
                pushing_vertices
                    .iter()
                    .zip([vert_pos].iter().cycle())
                    .position(|(other, cmp)| other == cmp)
                    .unwrap_or(pushing_vertices.len() - 1) as u32,
            )
        }
    }
}

fn contains_point(planes: &[Vec4], point: DVec3) -> bool {
    planes
        .iter()
        .map(|v| v.as_dvec4())
        // .all(|plane| plane.dot(point.extend(1.)) < 0.001f64)
        .all(|plane| plane.xyz().dot(point) - plane.w < 0.001f64)
}

fn calculate_intersection_point(planes: [&Vec4; 3]) -> Option<DVec3> {
    let [p1, p2, p3] = planes.map(|p| p.as_dvec4());
    let m1 = DVec3::new(p1.x, p2.x, p3.x);
    let m2 = DVec3::new(p1.y, p2.y, p3.y);
    let m3 = DVec3::new(p1.z, p2.z, p3.z);
    let d = -DVec3::new(p1.w, p2.w, p3.w);

    let u = m2.cross(m3);
    let v = m1.cross(d);

    let denom = m1.dot(u);

    // Check for parallel planes or if planes do not intersect
    if denom.abs() < f64::EPSILON {
        return None;
    }

    Some(DVec3::new(d.dot(u), m3.dot(v), -m2.dot(v)) / denom)
}
