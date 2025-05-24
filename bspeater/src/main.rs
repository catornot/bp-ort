#![allow(dead_code, unused, clippy::type_complexity)]
use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    ops::Div,
    process::Command,
};

trait SeekRead: Seek + Read {}
impl<T: Seek + Read> SeekRead for T {}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug)]
enum LumpIds {
    ENTITIES = 0x0000,
    PLANES = 0x0001,
    TEXTURE_DATA = 0x0002,
    VERTICES = 0x0003,
    LIGHTPROBE_PARENT_INFOS = 0x0004,
    SHADOW_ENVIRONMENTS = 0x0005,
    LIGHTPROBE_BSP_NODES = 0x0006,
    LIGHTPROBE_BSP_REF_IDS = 0x0007,
    UNUSED_8 = 0x0008,
    UNUSED_9 = 0x0009,
    UNUSED_10 = 0x000A,
    UNUSED_11 = 0x000B,
    UNUSED_12 = 0x000C,
    UNUSED_13 = 0x000D,
    MODELS = 0x000E,
    UNUSED_15 = 0x000F,
    UNUSED_16 = 0x0010,
    UNUSED_17 = 0x0011,
    UNUSED_18 = 0x0012,
    UNUSED_19 = 0x0013,
    UNUSED_20 = 0x0014,
    UNUSED_21 = 0x0015,
    UNUSED_22 = 0x0016,
    UNUSED_23 = 0x0017,
    ENTITY_PARTITIONS = 0x0018,
    UNUSED_25 = 0x0019,
    UNUSED_26 = 0x001A,
    UNUSED_27 = 0x001B,
    UNUSED_28 = 0x001C,
    PHYSICS_COLLIDE = 0x001D,
    VERTEX_NORMALS = 0x001E,
    UNUSED_31 = 0x001F,
    UNUSED_32 = 0x0020,
    UNUSED_33 = 0x0021,
    UNUSED_34 = 0x0022,
    GAME_LUMP = 0x0023,
    LEAF_WATER_DATA = 0x0024,
    UNUSED_37 = 0x0025,
    UNUSED_38 = 0x0026,
    UNUSED_39 = 0x0027,
    PAKFILE = 0x0028,
    UNUSED_41 = 0x0029,
    CUBEMAPS = 0x002A,
    TEXTURE_DATA_STRING_DATA = 0x002B,
    TEXTURE_DATA_STRING_TABLE = 0x002C,
    UNUSED_45 = 0x002D,
    UNUSED_46 = 0x002E,
    UNUSED_47 = 0x002F,
    UNUSED_48 = 0x0030,
    UNUSED_49 = 0x0031,
    UNUSED_50 = 0x0032,
    UNUSED_51 = 0x0033,
    UNUSED_52 = 0x0034,
    UNUSED_53 = 0x0035,
    WORLD_LIGHTS = 0x0036,
    WORLD_LIGHT_PARENT_INFOS = 0x0037,
    UNUSED_56 = 0x0038,
    UNUSED_57 = 0x0039,
    UNUSED_58 = 0x003A,
    UNUSED_59 = 0x003B,
    UNUSED_60 = 0x003C,
    UNUSED_61 = 0x003D,
    UNUSED_62 = 0x003E,
    UNUSED_63 = 0x003F,
    UNUSED_64 = 0x0040,
    UNUSED_65 = 0x0041,
    TRICOLL_TRIANGLES = 0x0042,
    UNUSED_67 = 0x0043,
    TRICOLL_NODES = 0x0044,
    TRICOLL_HEADERS = 0x0045,
    UNUSED_70 = 0x0046,
    VERTEX_UNLIT = 0x0047,
    VERTEX_LIT_FLAT = 0x0048,
    VERTEX_LIT_BUMP = 0x0049,
    VERTEX_UNLIT_TS = 0x004A,
    VERTEX_BLINN_PHONG = 0x004B,
    VERTEX_RESERVED_5 = 0x004C,
    VERTEX_RESERVED_6 = 0x004D,
    VERTEX_RESERVED_7 = 0x004E,
    MESH_INDICES = 0x004F,
    MESHES = 0x0050,
    MESH_BOUNDS = 0x0051,
    MATERIAL_SORTS = 0x0052,
    LIGHTMAP_HEADERS = 0x0053,
    UNUSED_84 = 0x0054,
    CM_GRID = 0x0055,
    CM_GRID_CELLS = 0x0056,
    CM_GEO_SETS = 0x0057,
    CM_GEO_SET_BOUNDS = 0x0058,
    CM_PRIMITIVES = 0x0059,
    CM_PRIMITIVE_BOUNDS = 0x005A,
    CM_UNIQUE_CONTENTS = 0x005B,
    CM_BRUSHES = 0x005C,
    CM_BRUSH_SIDE_PLANE_OFFSETS = 0x005D,
    CM_BRUSH_SIDE_PROPERTIES = 0x005E,
    CM_BRUSH_SIDE_TEXTURE_VECTORS = 0x005F,
    TRICOLL_BEVEL_STARTS = 0x0060,
    TRICOLL_BEVEL_INDICES = 0x0061,
    LIGHTMAP_DATA_SKY = 0x0062,
    CSM_AABB_NODES = 0x0063,
    CSM_OBJ_REFERENCES = 0x0064,
    LIGHTPROBES = 0x0065,
    STATIC_PROP_LIGHTPROBE_INDICES = 0x0066,
    LIGHTPROBE_TREE = 0x0067,
    LIGHTPROBE_REFERENCES = 0x0068,
    LIGHTMAP_DATA_REAL_TIME_LIGHTS = 0x0069,
    CELL_BSP_NODES = 0x006A,
    CELLS = 0x006B,
    PORTALS = 0x006C,
    PORTAL_VERTICES = 0x006D,
    PORTAL_EDGES = 0x006E,
    PORTAL_VERTEX_EDGES = 0x006F,
    PORTAL_VERTEX_REFERENCES = 0x0070,
    PORTAL_EDGE_REFERENCES = 0x0071,
    PORTAL_EDGE_INTERSECT_AT_EDGE = 0x0072,
    PORTAL_EDGE_INTERSECT_AT_VERTEX = 0x0073,
    PORTAL_EDGE_INTERSECT_HEADER = 0x0074,
    OCCLUSION_MESH_VERTICES = 0x0075,
    OCCLUSION_MESH_INDICES = 0x0076,
    CELL_AABB_NODES = 0x0077,
    OBJ_REFERENCES = 0x0078,
    OBJ_REFERENCE_BOUNDS = 0x0079,
    LIGHTMAP_DATA_RTL_PAGE = 0x007A,
    LEVEL_INFO = 0x007B,
    SHADOW_MESH_OPAQUE_VERTICES = 0x007C,
    SHADOW_MESH_ALPHA_VERTICES = 0x007D,
    SHADOW_MESH_INDICES = 0x007E,
    SHADOW_MESHES = 0x007F,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
enum Contents {
    // r1/scripts/vscripts/_consts.nut:1159
    EMPTY = 0x00,
    SOLID = 0x01,
    WINDOW = 0x02, // bulletproof glass etc. (transparent but solid)
    AUX = 0x04,    // unused ?
    GRATE = 0x08,  // allows bullets & vis
    SLIME = 0x10,
    WATER = 0x20,
    WINDOW_NO_COLLIDE = 0x40,
    ISOPAQUE = 0x80,         // blocks AI Line Of Sight, may be non - solid
    TEST_FOG_VOLUME = 0x100, // cannot be seen through, but may be non - solid
    UNUSED_1 = 0x200,
    BLOCK_LIGHT = 0x400,
    TEAM_1 = 0x800,
    TEAM_2 = 0x1000,
    IGNORE_NODRAW_OPAQUE = 0x2000, // ignore opaque if Surface.NO_DRAW
    MOVEABLE = 0x4000,
    PLAYER_CLIP = 0x10000, // blocks human players
    MONSTER_CLIP = 0x20000,
    BRUSH_PAINT = 0x40000,
    BLOCK_LOS = 0x80000, // block AI line of sight
    NO_CLIMB = 0x100000,
    TITAN_CLIP = 0x200000, // blocks titan players
    BULLET_CLIP = 0x400000,
    UNUSED_5 = 0x800000,
    ORIGIN = 0x1000000,  // removed before bsping an entity
    MONSTER = 0x2000000, // should never be on a brush, only in game
    DEBRIS = 0x4000000,
    DETAIL = 0x8000000,       // brushes to be added after vis leafs
    TRANSLUCENT = 0x10000000, // auto set if any surface has trans
    LADDER = 0x20000000,
    HITBOX = 0x40000000, // use accurate hitboxes on trace
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
enum MeshFlags {
    // source.Surface (source.TextureInfo rolled into titanfall.TextureData?)
    SKY_2D = 0x0002, // TODO: test overriding sky with this in-game
    SKY = 0x0004,
    WARP = 0x0008,        // Quake water surface ?
    TRANSLUCENT = 0x0010, // decals & atmo ?
    // titanfall.Mesh.flags
    VERTEX_LIT_FLAT = 0x000, // VERTEX_RESERVED_1
    VERTEX_LIT_BUMP = 0x200, // VERTEX_RESERVED_2
    VERTEX_UNLIT = 0x400,    // VERTEX_RESERVED_0
    VERTEX_UNLIT_TS = 0x600, // VERTEX_RESERVED_3
    // VERTEX_BLINN_PHONG = 0x ? ? ? # VERTEX_RESERVED_4
    SKIP = 0x20000, // 0x200 in valve.source.Surface(<< 8 ? )
    TRIGGER = 0x40000, // guessing
                    // masks
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
enum MeshMasks {
    MASK_VERTEX = 0x600,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BSPHeader {
    pub filemagic: [u8; 4],
    pub version: i32,
    pub map_revisions: i32,
    pub _127: i32,
    pub lumps: [LumpHeader; 128],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct LumpHeader {
    pub fileofs: i32, // offset into file (bytes)
    pub filelen: i32, // length of lump (bytes)
    pub version: i32, // lump format version
    pub four_cc: i32, // lump ident code}
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CMGrid {
    cell_size: f32,
    cell_org: [i32; 2],
    cell_count: [i32; 2],
    straddle_group_count: i32,
    base_plane_offset: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BspMesh {
    first_mesh_index: u32,
    num_triangles: u16,
    first_vertex: u16,
    num_vertices: u16,
    vertex_type: u16,
    styles: [u8; 4],
    luxel_origin: [u16; 2],
    luxel_offset_max: [u8; 2],
    material_sort: u16,
    mesh_flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct TricollHeader {
    flags: i16,         // always 0?
    texture_flags: i16, // copy of texture_data.flags
    texture_data: i16,  // probably for surfaceproperties & decals
    num_vertices: i16,  // Vertices indexed by TricollTriangles
    num_triangles: u16, // number of TricollTriangles in this TricollHeader
    // num_nodes is derived from the following formula
    // 2 * (num_triangles - (num_triangles + 3) % 6 + 3) // 3
    num_bevel_indices: u16,
    first_vertex: u32, // index into Vertices, added as an offset to TricollTriangles
    first_triangle: u32, // index into TricollTriangles;
    first_node: i32,   // index into TricollNodes
    first_bevel_index: u32, // index into TricollBevelIndices?
    origin: Vec3,      // true origin is -(origin / scale)
    scale: f32,        // 0.0 for patches
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct GridCell {
    geo_set_start: i16,
    geo_set_count: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct GeoSet {
    straddle_group: i16,
    prim_count: i16,
    prim_start: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct GeoSetBounds {
    origin: [i16; 3],
    cos: i16,
    extends: [i16; 3],
    sin: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
enum PrimitiveType {
    Brush = 0,
    Ticoll = 2,
    Prop = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VertexUnlit {
    vertex_index: i32,
    normal_index: i32,
    albedo_uv: Vec2,
    color: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VertexLitFlat {
    vertex_index: u32,
    normal_index: u32,
    albedo_uv: Vec2,
    color: u32,
    light_map_uv: [f32; 2],
    light_map_xy: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VertexLitBump {
    vertex_index: i32,
    normal_index: i32,
    albedo_uv: Vec2,
    color: u32,
    light_map_uv: [f32; 2],
    light_map_xy: [f32; 2],
    tangent: [i32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VertexUnlitTS {
    vertex_index: i32,
    normal_index: i32,
    albedo_uv: Vec2,
    color: u32,
    unk: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VertexBlinnPhong {
    vertex_index: i32,
    normal_index: i32,
    color: u32,
    uv: [f32; 4],
    tangent: [f32; 16],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Brush {
    origin: Vec3,
    num_non_axial_do_discard: u8,
    num_plane_offsets: u8,
    index: i16,
    extends: Vec3,
    brush_side_offset: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MaterialSort {
    texture_data: i16,
    light_map_header: i16,
    cubemap: i16,
    last_vertex: i16,
    vertex_offset: i32,
}

fn read_i32(reader: &mut dyn SeekRead) -> Result<i32, io::Error> {
    let mut int = [0; size_of::<i32>()];
    reader.read_exact(&mut int)?;
    Ok(i32::from_le_bytes(int))
}

fn read_f32(reader: &mut dyn SeekRead) -> Result<f32, io::Error> {
    let mut float = [0; size_of::<f32>()];
    reader.read_exact(&mut float)?;
    Ok(f32::from_le_bytes(float))
}

fn read_vec3(reader: &mut dyn SeekRead) -> Result<Vec3, io::Error> {
    Ok(Vec3::new(
        read_f32(reader)?,
        read_f32(reader)?,
        read_f32(reader)?,
    ))
}

fn read_lump(reader: &mut dyn SeekRead) -> Result<LumpHeader, io::Error> {
    Ok(LumpHeader {
        fileofs: read_i32(reader)?,
        filelen: read_i32(reader)?,
        version: read_i32(reader)?,
        four_cc: read_i32(reader)?,
    })
}

fn read_bspheader(reader: &mut dyn SeekRead) -> Result<BSPHeader, io::Error> {
    reader.seek(SeekFrom::Start(0))?;

    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    let version = read_i32(reader)?;

    assert_eq!(&magic, b"rBSP");
    assert_eq!(version, 37);

    Ok(BSPHeader {
        filemagic: magic,
        version,
        map_revisions: read_i32(reader)?,
        _127: read_i32(reader)?,
        lumps: (0..128)
            .map(|_| read_lump(reader))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| io::Error::other("incorrect size for lumps how!"))?,
    })
}

fn read_brush(reader: &mut dyn SeekRead) -> Result<Brush, io::Error> {
    Ok(Brush {
        origin: todo!(),
        num_non_axial_do_discard: todo!(),
        num_plane_offsets: todo!(),
        index: todo!(),
        extends: todo!(),
        brush_side_offset: todo!(),
    })
}

fn read_lump_data<T>(
    reader: &mut dyn SeekRead,
    header: &BSPHeader,
    id: LumpIds,
) -> Result<Vec<T>, io::Error> {
    let lump = get_lump(header, id);
    let size = std::mem::size_of::<T>();

    reader.seek(SeekFrom::Start(lump.fileofs as u64));

    let mut buf = vec![0; lump.filelen as usize];

    reader.read_exact(&mut buf)?;

    assert!(buf.len() % size == 0, "lump {id:?}");
    assert!(buf.capacity() % size == 0, "lump {id:?}");

    let tricoll = unsafe {
        Vec::<T>::from_raw_parts(
            buf.as_ptr().cast_mut().cast(),
            buf.len() / size,
            buf.capacity() / size,
        )
    };

    std::mem::forget(buf);

    Ok(tricoll)
}

fn get_lump(header: &BSPHeader, lump: LumpIds) -> &LumpHeader {
    &header.lumps[lump as usize]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let map_name = "mp_lf_uma";
    const PATH: &str = "/home/catornot/.local/share/Steam/steamapps/common/Titanfall2/vpk/";
    let name = format!("englishclient_{map_name}.bsp.pak000_dir.vpk");
    const UNPACK: &str = "target/vpk";

    Command::new("tf2-vpkunpack")
        .arg("--exclude")
        .arg("*")
        .arg("--include")
        .arg("maps")
        .arg(UNPACK)
        .arg(format!("{PATH}{name}"))
        .spawn()?
        .wait_with_output()?;

    let mut bsp = File::open(format!("{UNPACK}/maps/{map_name}.bsp"))?;

    assert!(std::mem::size_of::<Vec3>() == std::mem::size_of::<f32>() * 3);

    let header = read_bspheader(&mut bsp)?;
    let vertices = read_lump_data::<Vec3>(&mut bsp, &header, LumpIds::VERTICES)?;
    let normals = read_lump_data::<Vec3>(&mut bsp, &header, LumpIds::VERTEX_NORMALS)?;
    let mesh_indices = read_lump_data::<u16>(&mut bsp, &header, LumpIds::MESH_INDICES)?;
    let bspmeshes = read_lump_data::<BspMesh>(&mut bsp, &header, LumpIds::MESHES)?;
    let materialsorts = read_lump_data::<MaterialSort>(&mut bsp, &header, LumpIds::MATERIAL_SORTS)?;
    let vertex_unlit = read_lump_data::<VertexUnlit>(&mut bsp, &header, LumpIds::VERTEX_UNLIT)?;
    let vertex_lit_flat =
        read_lump_data::<VertexLitFlat>(&mut bsp, &header, LumpIds::VERTEX_LIT_FLAT)?;
    let vertex_lit_bump =
        read_lump_data::<VertexLitBump>(&mut bsp, &header, LumpIds::VERTEX_LIT_BUMP)?;
    let vertex_unlit_ts =
        read_lump_data::<VertexUnlitTS>(&mut bsp, &header, LumpIds::VERTEX_UNLIT_TS)?;

    println!("vertices {:#?}", vertices.len());
    println!("normals {:#?}", normals.len());

    let meshes = bspmeshes
        .into_par_iter()
        .filter_map(|bspmesh| {
            #[allow(clippy::eq_op)]
            let flag = MeshFlags::TRIGGER as u32 | MeshFlags::TRIGGER as u32;
            if (bspmesh.mesh_flags & flag) != 0
                || bspmesh.mesh_flags & MeshFlags::TRANSLUCENT as u32 != 0
            {
                return None;
            };

            let vertex_offset = materialsorts[bspmesh.material_sort as usize].vertex_offset;
            let vertex_offset2 = bspmesh.first_vertex;
            let mut vertexes: Vec<Vec3> = Vec::new();
            let mut indices = Vec::new();
            let mut uvs: Vec<Vec2> = Vec::new();

            for i in 0..bspmesh.num_triangles as usize * 3 {
                let vertex_index = (mesh_indices[i + bspmesh.first_mesh_index as usize] as usize
                    + vertex_offset as usize);
                let (vert_pos, vert_uv) =
                    match MeshFlags::try_from(bspmesh.mesh_flags & MeshMasks::MASK_VERTEX as u32)
                        .expect("not a mesh flag uh")
                    {
                        MeshFlags::VERTEX_LIT_FLAT => {
                            let vert = vertex_lit_flat[vertex_index];
                            (
                                vertices[vert.vertex_index as usize & 0x7FFFFFFF],
                                vert.albedo_uv,
                            )
                        }
                        MeshFlags::VERTEX_LIT_BUMP => {
                            let vert = vertex_lit_bump[vertex_index];
                            (
                                vertices[vert.vertex_index as usize & 0x7FFFFFFF],
                                vert.albedo_uv,
                            )
                        }
                        MeshFlags::VERTEX_UNLIT => {
                            let vert = vertex_unlit[vertex_index];
                            (
                                vertices[vert.vertex_index as usize & 0x7FFFFFFF],
                                vert.albedo_uv,
                            )
                        }
                        MeshFlags::VERTEX_UNLIT_TS => {
                            let vert = vertex_unlit_ts[vertex_index];
                            (
                                vertices[vert.vertex_index as usize & 0x7FFFFFFF],
                                vert.albedo_uv,
                            )
                        }
                        MeshFlags::SKY_2D
                        | MeshFlags::SKY
                        | MeshFlags::WARP
                        | MeshFlags::TRANSLUCENT
                        | MeshFlags::SKIP
                        | MeshFlags::TRIGGER => panic!("uh hu mesh flags"),
                    };
                let vert_pos = vert_pos.xzy();

                vertexes.push(vert_pos);
                uvs.push(vert_uv);

                indices.push(
                    vertexes
                        .iter()
                        .zip([vert_pos].iter().cycle())
                        .position(|(other, cmp)| other == cmp)
                        .unwrap_or(vertexes.len() - 1) as u32,
                )
            }

            Some(
                Mesh::new(
                    bevy::render::mesh::PrimitiveTopology::TriangleList,
                    RenderAssetUsages::all(),
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertexes)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
                .with_inserted_indices(bevy::render::mesh::Indices::U32(
                    indices, // indices.into_iter().rev().collect(),
                )),
            )
        })
        .collect::<Vec<Mesh>>();

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        FlyCameraPlugin,
        PhysicsPlugins::default(),
        // Physicsugin::default(),
        // WireframePlugin::default(),
    ))
    .init_resource::<WireframeConfig>()
    .add_systems(Startup, setup)
    .insert_resource(WorldName(map_name.to_owned()))
    .init_state::<ProcessingStep>();

    let materials = {
        let mut mat = app
            .world_mut()
            .get_resource_mut::<Assets<StandardMaterial>>()
            .expect("this should exist probably");
        [
            mat.add(StandardMaterial::from_color(Color::srgba_u8(
                100, 0, 0, 255,
            ))),
            mat.add(StandardMaterial::from_color(Color::srgba_u8(
                0, 100, 0, 255,
            ))),
            mat.add(StandardMaterial::from_color(Color::srgba_u8(
                0, 0, 100, 255,
            ))),
        ]
    };

    for mesh in meshes
        .into_iter()
        .enumerate()
        .map(|(i, mesh)| {
            (
                Collider::trimesh_from_mesh(&mesh).expect("huh"),
                RigidBody::Static,
                Mesh3d(
                    app.world_mut()
                        .get_resource_mut::<Assets<Mesh>>()
                        .expect("this should exist probably")
                        .add(mesh),
                ),
                MeshMaterial3d(materials[i % 3].clone()),
            )
        })
        .collect::<Vec<_>>()
    {
        app.world_mut().spawn(mesh);
    }

    app.add_systems(Startup, calc_extents)
        .add_systems(
            Update,
            (
                spawn_world_raycast.run_if(in_state(ProcessingStep::RayCasting)),
                raycast_world.run_if(in_state(ProcessingStep::Cleanup)),
                debug_world,
            ),
        )
        .run();

    Ok(())
}

#[derive(Resource, Clone, Copy, PartialEq)]
struct WorlExtents(Vec3, Vec3);

#[derive(Resource, Clone, PartialEq)]
struct WorldName(String);

#[derive(Component, Clone, Copy, PartialEq)]
struct WireMe;

#[derive(Component, Clone, Copy, PartialEq)]
struct GridPos(IVec3);

#[derive(Component, Clone, Copy, PartialEq)]
struct HitStuff;

#[derive(Debug, States, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
enum ProcessingStep {
    #[default]
    Startup,
    RayCasting,
    Cleanup,
    Saving,
    Done,
}

fn setup(mut commands: Commands, mut wireframe_config: ResMut<WireframeConfig>) {
    commands.spawn((
        Camera3d::default(),
        FlyCamera {
            max_speed: 50.,
            accel: 49.,
            friction: 40.,
            ..default()
        },
    ));
    wireframe_config.global = true;
}

fn calc_extents(
    mut commands: Commands,
    meshes: Query<&Mesh3d>,
    assets: Res<Assets<Mesh>>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    let (min, max) = meshes
        .iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|mesh| assets.get(&mesh.0))
        .filter_map(|mesh| {
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .map(|pos| match pos {
                    bevy::render::mesh::VertexAttributeValues::Float32x3(vertexes) => vertexes
                        .iter()
                        .map(|pos| Vec3::from_array(*pos))
                        .fold((Vec3::ZERO, Vec3::ZERO), |current, cmp| {
                            (current.0.min(cmp), current.1.max(cmp))
                        }),
                    _ => panic!("vertex is not vertex"),
                })
        })
        .reduce(
            || (Vec3::ZERO, Vec3::ZERO),
            |current, cmp| (current.0.min(cmp.0), current.1.max(cmp.1)),
        );

    commands.insert_resource(WorlExtents(min, max));
    next_state.set(ProcessingStep::RayCasting);
}

const CELL_SIZE: f32 = 50.;
fn spawn_world_raycast(
    mut commands: Commands,
    extends: Res<WorlExtents>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    dbg!("spawn_world_raycast");
    let extends = *extends;
    let commands = &mut commands;

    let mut spawns = (extends.0.x.div(CELL_SIZE) as i32..extends.1.x.div(CELL_SIZE) as i32)
        .flat_map(|x| {
            (extends.0.y.div(CELL_SIZE) as i32..extends.1.y.div(CELL_SIZE) as i32).flat_map(
                move |y| {
                    (extends.0.z.div(CELL_SIZE) as i32..extends.1.z.div(CELL_SIZE) as i32).map(
                        move |z| {
                            let origin = Vec3::new(
                                x as f32 * CELL_SIZE,
                                y as f32 * CELL_SIZE,
                                z as f32 * CELL_SIZE,
                            );
                            (
                                GridPos(IVec3::new(x, y, z)),
                                Transform::from_translation(origin),
                                avian3d::spatial_query::ShapeCaster::new(
                                    Collider::cuboid(CELL_SIZE, CELL_SIZE, CELL_SIZE),
                                    origin.with_y(y as f32 * CELL_SIZE + CELL_SIZE),
                                    Quat::default(),
                                    Dir3::NEG_Y,
                                )
                                .with_compute_contact_on_penetration(false)
                                .with_max_distance(0.),
                            )
                        },
                    )
                },
            )
        })
        .collect::<Vec<_>>();

    // {
    //     let origin = Vec3::new(0., 0., 0.);
    //     spawns.push((
    //         GridPos(IVec3::new(0, 0, 0)),
    //         Transform::from_translation(origin),
    //         avian3d::spatial_query::ShapeCaster::new(
    //             Collider::cuboid(CELL_SIZE, CELL_SIZE, CELL_SIZE),
    //             origin,
    //             Quat::default(),
    //             Dir3::NEG_Y,
    //         )
    //         .with_compute_contact_on_penetration(false)
    //         .with_max_distance(CELL_SIZE),
    //     ));
    // }

    commands.spawn_batch(spawns);
    next_state.set(ProcessingStep::Cleanup);
}

fn raycast_world(
    mut commands: Commands,
    ray_casts: Query<(Entity, &ShapeCaster, Option<&ShapeHits>, &GridPos)>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    if !ray_casts.iter().any(|(_, _, hits, _)| hits.is_some()) {
        return;
    }

    dbg!("raycast_world");
    dbg!(ray_casts.iter().count());
    ray_casts.iter().for_each(|(ent, _, hits, _)| {
        commands
            .entity(ent)
            .remove::<ShapeCaster>()
            .remove::<ShapeHits>()
            .insert_if(HitStuff, || {
                hits.into_iter()
                    .flat_map(|hits| hits.iter())
                    .next()
                    .is_some()
            })
            .insert(WireMe);
    });

    if ray_casts.is_empty() {
        next_state.set(ProcessingStep::Saving);
    }
}

fn save_navmesh(meshes: Query<(&GridPos, Option<&HitStuff>)>, mut gizmos: Gizmos) {
    // dbg!(ray_casts.iter().count());
}

fn debug_world(
    meshes: Query<&GridPos, (With<HitStuff>, With<WireMe>, Without<FlyCamera>)>,
    camera: Query<&Transform, (With<FlyCamera>, Without<WireMe>)>,
    mut gizmos: Gizmos,
) -> Result<(), BevyError> {
    dbg!(meshes.iter().count());
    let origin = camera.single()?.translation;

    for origin in meshes
        .iter()
        .map(|pos| pos.0.as_vec3() * Vec3::splat(CELL_SIZE))
        .filter(|translation| translation.distance(origin) < 1000.)
    {
        gizmos.cuboid(
            Transform::from_translation(origin).with_scale(Vec3::splat(50.)),
            Color::srgba_u8(255, 0, 0, 255),
        );
    }

    Ok(())
}

impl TryFrom<u32> for MeshFlags {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x0002 => MeshFlags::SKY_2D,
            0x0004 => MeshFlags::SKY,
            0x0008 => MeshFlags::WARP,
            0x0010 => MeshFlags::TRANSLUCENT,
            0x000 => MeshFlags::VERTEX_LIT_FLAT,
            0x200 => MeshFlags::VERTEX_LIT_BUMP,
            0x400 => MeshFlags::VERTEX_UNLIT,
            0x600 => MeshFlags::VERTEX_UNLIT_TS,
            0x20000 => MeshFlags::SKIP,
            0x40000 => MeshFlags::TRIGGER,
            value => return Err(value),
        })
    }
}
