#![allow(unused, dead_code)]

use once_cell::sync::Lazy;
use recastnavigation_sys::*;
use rrplug::prelude::*;

use crate::bindings::SERVER_FUNCTIONS;

static mut NAV_MESH_QUERY: Lazy<*mut dtNavMeshQuery> =
    Lazy::new(|| unsafe { dtAllocNavMeshQuery() });

pub unsafe fn get_path(start: Vector3, end: Vector3) {
    // sooooooo
    // this will not work :(
    // the types are diffrent
    let nav_mesh = SERVER_FUNCTIONS.wait().nav_mesh;

    for i in 0..4 {
        dbg!(&*(*nav_mesh).add(i));
    }

    // squirrel GetNodeCount?
    let result = dbg!(dtNavMeshQuery_init(*NAV_MESH_QUERY, *nav_mesh, 65500)); //65500

    if result & DT_FAILURE != 0 {
        log::error!("failed to init navmeshes");
        return;
    }

    let filter = dtQueryFilter::new();
    let mut closest_poly1 = 0;
    let mut nearset_position1 = Vector3::ZERO;
    dbg!(dtNavMeshQuery_findNearestPoly(
        *NAV_MESH_QUERY,
        (&Vector3::new(100., 100., 1.)).into(),
        (&Vector3::new(100., 100., 100.)).into(),
        &filter,
        &mut closest_poly1,
        &mut nearset_position1.x,
    ));

    dbg!((closest_poly1, nearset_position1));

    let mut closest_poly2 = 0;
    let mut nearset_position2 = Vector3::ZERO;
    dbg!(dtNavMeshQuery_findNearestPoly(
        *NAV_MESH_QUERY,
        (&Vector3::new(100., 1000., 1.)).into(),
        (&Vector3::new(100., 100., 100.)).into(),
        &filter,
        &mut closest_poly2,
        &mut nearset_position2.x,
    ));

    dbg!((closest_poly2, nearset_position2));

    let mut path = [0; 100];
    let mut path_count = 0;
    dbg!(dtNavMeshQuery_findPath(
        *NAV_MESH_QUERY,
        closest_poly1,
        closest_poly2,
        &nearset_position1.x,
        &nearset_position2.x,
        &filter,
        path.as_mut_ptr(),
        &mut path_count,
        100
    ));
}
