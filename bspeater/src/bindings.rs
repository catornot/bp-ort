use bevy::prelude::*;
use modular_bitfield::prelude::*;

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug)]
pub enum LumpIds {
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
pub enum Contents {
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
pub enum MeshFlags {
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
pub enum MeshMasks {
    MASK_VERTEX = 0x600,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BSPHeader {
    pub filemagic: [u8; 4],
    pub version: i32,
    pub map_revisions: i32,
    pub _127: i32,
    pub lumps: [LumpHeader; 128],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LumpHeader {
    pub fileofs: i32, // offset into file (bytes)
    pub filelen: i32, // length of lump (bytes)
    pub version: i32, // lump format version
    pub four_cc: i32, // lump ident code}
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CMGrid {
    pub cell_size: f32,
    pub cell_org: [i32; 2],
    pub cell_count: [i32; 2],
    pub straddle_group_count: i32,
    pub base_plane_offset: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BspMesh {
    pub first_mesh_index: u32,
    pub num_triangles: u16,
    pub first_vertex: u16,
    pub num_vertices: u16,
    pub vertex_type: u16,
    pub styles: [u8; 4],
    pub luxel_origin: [u16; 2],
    pub luxel_offset_max: [u8; 2],
    pub material_sort: u16,
    pub mesh_flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TricollHeader {
    pub flags: i16,         // always 0?
    pub texture_flags: i16, // copy of texture_data.flags
    pub texture_data: i16,  // probably for surfaceproperties & decals
    pub num_vertices: i16,  // Vertices indexed by TricollTriangles
    pub num_triangles: u16, // number of TricollTriangles in this TricollHeader
    // num_nodes is derived from the following formula
    // 2 * (num_triangles - (num_triangles + 3) % 6 + 3) // 3
    pub num_bevel_indices: u16,
    pub first_vertex: u32, // index into Vertices, added as an offset to TricollTriangles
    pub first_triangle: u32, // index into TricollTriangles;
    pub first_node: i32,   // index into TricollNodes
    pub first_bevel_index: u32, // index into TricollBevelIndices?
    pub origin: Vec3,      // true origin is -(origin / scale)
    pub scale: f32,        // 0.0 for patches
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GridCell {
    pub geo_set_start: i16,
    pub geo_set_count: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GeoSet {
    pub straddle_group: i16,
    pub prim_count: i16,
    pub prim_start: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GeoSetBounds {
    pub origin: [i16; 3],
    pub cos: i16,
    pub extends: [i16; 3],
    pub sin: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Hash, Eq)]
pub enum PrimitiveType {
    Brush = 0,
    Tricoll = 2,
    Prop = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VertexUnlit {
    pub vertex_index: i32,
    pub normal_index: i32,
    pub albedo_uv: Vec2,
    pub color: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VertexLitFlat {
    pub vertex_index: u32,
    pub normal_index: u32,
    pub albedo_uv: Vec2,
    pub color: u32,
    pub light_map_uv: [f32; 2],
    pub light_map_xy: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VertexLitBump {
    pub vertex_index: i32,
    pub normal_index: i32,
    pub albedo_uv: Vec2,
    pub color: u32,
    pub light_map_uv: [f32; 2],
    pub light_map_xy: [f32; 2],
    pub tangent: [i32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VertexUnlitTS {
    pub vertex_index: i32,
    pub normal_index: i32,
    pub albedo_uv: Vec2,
    pub color: u32,
    pub unk: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VertexBlinnPhong {
    pub vertex_index: i32,
    pub normal_index: i32,
    pub color: u32,
    pub uv: [f32; 4],
    pub tangent: [f32; 16],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub origin: Vec3,
    pub num_non_axial_do_discard: u8,
    pub num_plane_offsets: u8,
    pub index: i16,
    pub extends: Vec3,
    pub brush_side_offset: i32,
}

static ASSERT: () = assert!(std::mem::size_of::<Brush>() == 0x20);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MaterialSort {
    pub texture_data: i16,
    pub light_map_header: i16,
    pub cubemap: i16,
    pub last_vertex: i16,
    pub vertex_offset: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TricollNode {
    pub vals: [i16; 8], //just a guess because 16bit intrinics are used on this at engine.dll + 0x1D1B10
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TricollTri {
    pub data: u32, //bitpacked
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TricollBevelStart {
    pub val: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TricollBevelIndex {
    pub gap_0: [u8; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColBrush {
    pub gap_0: [u8; 32],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CollPrimitive {
    pub val: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Dtexdata {
    pub reflectivity: Vec3,
    pub name_string_table_id: i32,
    pub width: i32,
    pub height: i32,
    pub view_width: i32,
    pub view_height: i32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DCollbrush {
    pub origin: Vec3,               // size: 12
    pub non_axial_count: [u8; 2],   // size: 2
    pub prior_brush_count: i16,     // size: 2
    pub extent: Vec3,               // size: 12
    pub prior_non_axial_count: i32, // size: 4
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StaticProp {
    pub origin: Vec3,
    pub angles: Vec3,
    pub scale: f32,
    pub model_index: u16,
    pub solid: u8,
    pub flags: u8,
    pub skin: u16,
    pub word_22: u16,
    pub forced_fade_scale: f32,
    pub lighting_origin: Vec3,
    pub diffuse_modulation_r: u8,
    pub diffuse_modulation_g: u8,
    pub diffuse_modulation_b: u8,
    pub diffuse_modulation_a: u8,
    pub unk: i32,
    pub collision_flags_remove: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
    // file version as defined by VHV_VERSION
    pub version: i32,

    // hardware params that affect how the model is to be optimized.
    pub vert_cache_size: i32,
    pub max_bones_per_strip: u16,
    pub max_bones_per_face: u16,
    pub max_bones_per_vert: i32,

    // must match checkSum in the .mdl
    pub check_sum: i32,

    pub num_lods: i32, // garymcthack - this is also specified in ModelHeader_t and should match

    // one of these for each LOD
    pub material_replacement_list_offset: i32,

    pub num_body_parts: i32,
    pub body_part_offset: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BodyPartHeader {
    pub num_models: i32,
    pub model_offset: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModelHeader {
    pub num_lods: i32,
    pub lod_offset: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModelLODHeader {
    pub num_meshes: i32,
    pub mesh_offset: i32,
    pub switch_point: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct MeshHeader {
    pub num_strip_groups: i32,
    pub strip_group_header_offset: i32,
    pub flags: u8,
}
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StripGroupHeader {
    pub num_verts: i32,
    pub vert_offset: i32,
    pub num_indices: i32,
    pub index_offset: i32,
    pub num_strips: i32,
    pub strip_offset: i32,
    pub flags: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StripHeader {
    // indexOffset offsets into the mesh's index array.
    pub num_indices: i32,
    pub index_offset: i32,

    // vertexOffset offsets into the mesh's vert array.
    pub num_verts: i32,
    pub vert_offset: i32,

    // use this to enable/disable skinning.
    // May decide (in optimize.cpp) to put all with 1 bone in a different strip
    // than those that need skinning.
    pub num_bones: u16,

    pub flags: u8,

    pub num_bone_state_changes: i32,
    pub bone_state_change_offset: i32,
}

const MAX_NUM_BONES_PER_VERT: usize = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    // these index into the mesh's vert[origMeshVertID]'s bones
    pub bone_weight_index: [u8; MAX_NUM_BONES_PER_VERT],
    pub num_bones: u8,

    pub orig_mesh_vert_id: u16,

    // for sw skinned verts, these are indices into the global list of bones
    // for hw skinned verts, these are hardware bone indices
    pub bone_id: [u8; MAX_NUM_BONES_PER_VERT],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Studiohdr {
    pub id: i32,          // Model format ID, such as "IDST" (0x49 0x44 0x53 0x54)
    pub version: i32,     // Format version number, such as 53 (0x35,0x00,0x00,0x00)
    pub checksum: i32,    // This has to be the same in the phy and vtx files to load!
    pub sznameindex: i32, // This has been moved from studiohdr2_t to the front of the main header.
    pub name: [u8; 64],   // The internal name of the model, padding with null chars.
    // Typically "my_model.mdl" will have an internal name of "my_model"
    pub length: i32, // Data size of MDL file in chars.

    pub eyeposition: Vec3, // ideal eye position

    pub illumposition: Vec3, // illumination center

    pub hull_min: Vec3, // ideal movement hull size
    pub hull_max: Vec3,

    pub view_bbmin: Vec3, // clipping bounding box
    pub view_bbmax: Vec3,

    pub flags: i32,

    // highest observed: 250
    // max is definitely 256 because 8bit uint limit
    pub numbones: i32, // bones
    pub boneindex: i32,

    pub numbonecontrollers: i32, // bone controllers
    pub bonecontrollerindex: i32,

    pub numhitboxsets: i32,
    pub hitboxsetindex: i32,

    pub numlocalanim: i32,   // animations/poses
    pub localanimindex: i32, // animation descriptions

    pub numlocalseq: i32, // sequences
    pub localseqindex: i32,

    pub activitylistversion: i32, // initialization flag - have the sequences been indexed? set on load
    pub eventsindexed: i32,

    // mstudiotexture_t
    // short rpak path
    // raw textures
    pub numtextures: i32, // the material limit exceeds 128, probably 256.
    pub textureindex: i32,

    // this should always only be one, unless using vmts.
    // raw textures search paths
    pub numcdtextures: i32,
    pub cdtextureindex: i32,

    // replaceable textures tables
    pub numskinref: i32,
    pub numskinfamilies: i32,
    pub skinindex: i32,

    pub numbodyparts: i32,
    pub bodypartindex: i32,

    pub numlocalattachments: i32,
    pub localattachmentindex: i32,

    pub numlocalnodes: i32,
    pub localnodeindex: i32,
    pub localnodenameindex: i32,

    pub deprecated_numflexdesc: i32,
    pub deprecated_flexdescindex: i32,

    pub deprecated_numflexcontrollers: i32,
    pub deprecated_flexcontrollerindex: i32,

    pub deprecated_numflexrules: i32,
    pub deprecated_flexruleindex: i32,

    pub numikchains: i32,
    pub ikchainindex: i32,

    pub ui_panel_count: i32,
    pub ui_panel_offset: i32,

    pub numlocalposeparameters: i32,
    pub localposeparamindex: i32,

    pub surfacepropindex: i32,

    pub keyvalueindex: i32,
    pub keyvaluesize: f32,

    pub numlocalikautoplaylocks: i32,
    pub localikautoplaylockindex: i32,

    pub mass: f32,
    pub contents: i32,

    // external animations, models, etc.
    pub numincludemodels: i32,
    pub includemodelindex: u8,

    // implementation specific back pointer to virtual data
    pub virtual_model: i32,

    pub bonetablebynameindex: i32,

    // if STUDIOHDR_FLAGS_CONSTANT_DIRECTIONAL_LIGHT_DOT is set,
    // this value is used to calculate directional components of lighting
    // on static props
    pub constdirectionallightdot: u8,

    // set during load of mdl data to track *desired* lod configuration (not actual)
    // the *actual* clamped root lod is found in studiohwdata
    // this is stored here as a global store to ensure the staged loading matches the rendering
    pub root_lod: f32,

    // set in the mdl data to specify that lod configuration should only allow first numAllowRootLODs
    // to be set as root LOD:
    //	numAllowedRootLODs = 0	means no restriction, any lod can be set as root lod.
    //	numAllowedRootLODs = N	means that lod0 - lod(N-1) can be set as root lod, but not lodN or lower.
    pub num_allowed_root_lods: u8,

    pub unused: u8,

    pub default_fade_dist: f32, // set to -1 to never fade. set above 0 if you want it to fade out, distance is in feet.
    // player/titan models seem to inherit this value from the first model loaded in menus.
    // works oddly on entities, probably only meant for static props
    pub deprecated_numflexcontrollerui: i32,
    pub deprecated_flexcontrolleruiindex: i32,

    pub fl_vert_anim_fixed_point_scale: f32,
    pub surfaceprop_lookup: i32, // this index must be cached by the loader, not saved in the file

    // stored maya files from used dmx files, animation files are not added. for internal tools likely
    // in r1 this is a mixed bag, some are null with no data, some have a four byte section, and some actually have the files stored.
    pub source_filename_offset: i32,

    pub numsrcbonetransform: i32,
    pub srcbonetransformindex: i32,

    pub illumpositionattachmentindex: i32,

    pub linearboneindex: i32,

    pub m_n_bone_flex_driver_count: i32,
    pub m_n_bone_flex_driver_index: i32,

    // for static props (and maybe others)
    // Precomputed Per-Triangle AABB data
    pub m_n_per_tri_aabbindex: i32,
    pub m_n_per_tri_aabbnode_count: i32,
    pub m_n_per_tri_aabbleaf_count: i32,
    pub m_n_per_tri_aabbvert_count: i32,

    // always "" or "Titan", this is probably for internal tools
    pub unk_string_offset: i32,

    // ANIs are no longer used and this is reflected in many structs
    // start of interal file data
    pub vtx_offset: i32, // VTX
    pub vvd_offset: i32, // VVD / IDSV
    pub vvc_offset: i32, // VVC / IDCV
    pub phy_offset: i32, // VPHY / IVPS

    pub vtx_size: i32, // VTX
    pub vvd_size: i32, // VVD / IDSV
    pub vvc_size: i32, // VVC / IDCV
    pub phy_size: i32, // VPHY / IVPS

    // this data block is related to the vphy, if it's not present the data will not be written
    // definitely related to phy, apex phy has this merged into it
    pub unk_offset: i32, // section between vphy and vtx.?
    pub unk_count: i32,  // only seems to be used when phy has one solid

    // mostly seen on '_animated' suffixed models
    // manually declared bone followers are no longer stored in kvs under 'bone_followers', they are now stored in an array of ints with the bone index.
    pub bone_follower_count: i32,
    pub bone_follower_offset: i32, // index only written when numbones > 1, means whatever func writes this likely checks this (would make sense because bonefollowers need more than one bone to even be useful). maybe only written if phy exists

    pub unused1: [i32; 60],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhyHeader {
    pub size: i32, // Size of this header section (generally 16), this is also version.
    pub id: i32,   // Often zero, unknown purpose.
    pub solid_count: i32, // Number of solids in file
    pub check_sum: i32, // checksum of source .mdl file (4-bytes)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhySection {
    pub surfaceheader: SwapCompactSurfaceheader,
    pub surfaceheader2: LegacySurfaceHeader,
    pub ledge: Compactledge,
    pub tri: Compacttriangle,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SwapCompactSurfaceheader {
    pub size: i32, // size of the content after this byte
    pub vphysics_id: i32,
    pub version: i16,
    pub model_type: i16,
    pub surface_size: i32,
    pub drag_axis_areas: Vec3,
    pub axis_map_size: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LegacySurfaceHeader {
    pub mass_center: Vec3,
    pub rotation_inertia: Vec3,

    pub upper_limit_radius: f32,

    // big if true
    pub packed: BitPackedPart,
    pub offset_ledgetree_root: i32,

    pub dummy: [i32; 3], // dummy[2] is id
}

#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy, Specifier)]
pub struct BitPackedPart {
    pub max_deviation: B8, // 8
    pub byte_size: B24,    // 24
}

#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy, Specifier)]
pub struct Compactedge {
    pub start_point_index: B16, // point index
    pub opposite_index: B15, // rel to this // maybe extra array, 3 bits more than tri_index/pierce_index
    pub is_virtual: bool,
}
// static_assert(sizeof(compactedge_t) == 4);

#[bitfield(bits = 128)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Compacttriangle {
    pub tri_index: B12, // used for upward navigation
    pub pierce_index: B12,
    pub material_index: B7,
    pub is_virtual: bool,

    // three edges
    pub edge1: Compactedge,
    pub edge2: Compactedge,
    pub edge3: Compactedge,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Compactledge {
    pub c_point_offset: i32, // byte offset from 'this' to (ledge) point array
    pub offsets: i32,
    pub packed: i32,
    pub n_triangles: i16,
    pub for_future_use: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhyVertex {
    pub pos: Vec3,    // relative to bone
    pub pad: [u8; 4], // align to 16 bytes
}

pub struct BSPData {
    pub vertices: Vec<Vec3>,
    pub tricoll_headers: Vec<TricollHeader>,
    pub tricoll_triangles: Vec<TricollTri>,
    pub texture_data: Vec<Dtexdata>,
    pub geo_sets: Vec<GeoSet>,
    pub col_primatives: Vec<CollPrimitive>,
    pub unique_contents: Vec<i32>,
    pub brushes: Vec<Brush>,
    pub brush_side_plane_offsets: Vec<u16>,
    pub brush_planes: Vec<Vec4>,
    pub grid: CMGrid,
    pub props: Vec<StaticProp>,
    pub model_data: Vec<Option<(Vec<Vec3>, Vec<u32>)>>,
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

impl TryFrom<u32> for PrimitiveType {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Brush,
            2 => Self::Tricoll,
            3 => Self::Prop,
            value => return Err(value),
        })
    }
}
