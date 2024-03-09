#![allow(non_snake_case)]

use rrplug::{offset_functions, prelude::Vector3};

pub mod bindings;
pub mod navigation;

use bindings::*;

#[repr(i32)]
#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Hull {
    #[default]
    Human = 0,
    Medium = 1,
    FlyingVehicle = 2,
    Small = 3,
    Titan = 4,
}
offset_functions! {
    RECAST_DETOUR + RecastDetour for WhichDll::Server => {
        nav_mesh = *mut *mut dtNavMesh where offset(0x105F5D0);
        ai_network = *mut *mut CAI_Network where offset(0x1061160);

        dtNavMesh__calcTileLoc = unsafe extern "C" fn() where offset(0x3e5f90);
        dtNavMesh__closestPointOnPoly = unsafe extern "C" fn() where offset(0x3e6250);
        dtNavMesh__getMaxTiles = unsafe extern "C" fn() where offset(0x3e7840);
        dtNavMesh__getParams = unsafe extern "C" fn() where offset(0x3e7b20);
        dtNavMesh__isValidPolyRef = unsafe extern "C" fn() where offset(0x3e7c00);
        dtNavMesh__getTileAndPolyByRef = unsafe extern "C" fn() where offset(0x3e7ea0);
        dtNavMesh__getTileAndPolyByRefUnsafe = unsafe extern "C" fn() where offset(0x3e7f90);
        dtNavMesh__getTileAt = unsafe extern "C" fn() where offset(0x3e83a0);
        dtNavMesh__isValidPolyRef2 = unsafe extern "C" fn() where offset(0x3e8b00);
        ZeroOutdtNavMesh = unsafe extern "C" fn(*mut dtNavMeshQuery) where offset(0x3e9560);
        navmesh_maybe_init_filter = unsafe extern "C" fn(*mut dtQueryFilter) -> *mut dtQueryFilter where offset(0x3e95a0);
        dtNavMeshQuery__closestPointOnPolyBoundary__variant = unsafe extern "C" fn(this: *const dtNavMeshQuery, _ref: dtPolyRef, pos: *const Vector3, closest: *mut Vector3, filter: *const dtQueryFilter) where offset(0x3ea750);
        dtNavMeshQuery__findPath = unsafe extern "C" fn(this: *mut dtNavMeshQuery,startRef: dtPolyRef,endRef: dtPolyRef, startPos: *const Vector3, endPos: *const Vector3, filter: *const dtQueryFilter, path: *mut dtPolyRef,unk: *const undefined, pathCount: *mut u32,maxPath: i32) -> dtStatus where offset(0x3ec310);
        dtNavMeshQuery__findStraightPath = unsafe extern "C" fn() where offset(0x3ee980);
        dtNavMeshQuery__SmthPathPortal = unsafe extern "C" fn() where offset(0x3ef820);
        dtNavMeshQuery__getPolyWallSegments = unsafe extern "C" fn() where offset(0x3efe30);
        dtNavMeshQuery__getEdgeMidPoint = unsafe extern "C" fn() where offset(0x3f0690);
        dtNavMeshQuery__init = unsafe extern "C" fn(this: *mut dtNavMeshQuery, nav: *const dtNavMesh ,maxNodes: i32) -> dtStatus64 where offset(0x3f0980);
        dtNavMesh__getTileMaybe = unsafe extern "C" fn() where offset(0x3f0da0);
        dtNavMeshQuery__findNearestPoly = unsafe extern "C" fn(this: *mut dtNavMeshQuery, center: *const Vector3,halfExtents: *const Vector3,filter: *const dtQueryFilter,nearestRef: *mut dtPolyRef ,nearestPt: *mut Vector3) -> dtStatus64 where offset(0x3ebe50);
        dtFreeNavMeshQuery_Destroy = unsafe extern "C" fn(*mut dtNavMeshQuery) where offset(0x3e95d0); // doesn't free the pointer only things inside

        GetNavMeshHullIndex = unsafe extern "C" fn(i32) -> i32 where offset(0x35e200);
    }
}
