#![allow(unused, dead_code)]

use once_cell::sync::Lazy;
use recastnavigation_sys::*;
use rrplug::prelude::*;

use crate::bindings::SERVER_FUNCTIONS;

static mut NAV_MESH_QUERY: Lazy<*mut dtNavMeshQuery> =
    Lazy::new(|| unsafe { dtAllocNavMeshQuery() });

pub unsafe fn get_path(start: Vector3, end: Vector3) {
    let nav_mesh = SERVER_FUNCTIONS.wait().nav_mesh;

    dbg!(dtNavMeshQuery_init(*NAV_MESH_QUERY, nav_mesh, 65500));

    // dtNavMeshQuery_findNearestPoly(, , , , , )
}
