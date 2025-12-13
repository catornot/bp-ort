#![deny(clippy::unwrap_used, clippy::expect_used)]
use rand::Rng;
use rrplug::prelude::*;
use std::mem::MaybeUninit;
use thiserror::Error;

use super::{
    bindings::{dtNavMeshQuery, dtPolyRef, dtQueryFilter},
    Hull, JumpTypes, RecastDetour, RECAST_DETOUR,
};

const HUMAN_EXTENTS: Vector3 = Vector3::new(100.0, 100.0, 136.0);
const TITAN_EXTENTS: Vector3 = Vector3::new(200.0, 200.0, 500.0);
const TARGET_EXTENTS: Vector3 = Vector3::new(200.0, 200.0, 500.0);
const PATH_CAPACITY: usize = 256;

#[derive(Error, Debug)]
pub enum NavigationError {
    #[error("failed to find nearest poly, with start {0:?}({2}) and end {1:?}({3})")]
    NoPoints(Vector3, Vector3, u32, u32),

    #[error("path not found between {0:?} and {1:?}")]
    PathNotFound(Vector3, Vector3),
}

#[derive(Debug)]
pub struct Navigation {
    query: dtNavMeshQuery,
    extents: &'static Vector3,
    pub hull: Hull,
    pub filter: dtQueryFilter,
    path: Vec<dtPolyRef>,
    pub path_points: Vec<Vector3>,
    pub end_point: Option<Vector3>,
    jump_types: Vec<u8>,
    straigth_path_points: Vec<Vector3>,
    straigth_path: Vec<dtPolyRef>,
    straigth_path_flags: Vec<u8>,
    straigth_path_jumps: Vec<u8>,
}

impl Navigation {
    pub fn new(hull: Hull) -> Option<Self> {
        Some(Self {
            query: generate_nav_query(hull, None)?,
            extents: hull_to_extents(hull),
            hull,
            filter: dtQueryFilter {
                m_areaCost: if hull == Hull::Human {
                    // [
                    //     1621.6901, 1274.1852, 1698.9136, 1158.3501, 1814.7485, 2123.6418, 0.0, 0.0,
                    //     3243.3801, 2123.6418, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                    //     2123.6418, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                    // ]
                    [0.0; 32]
                } else {
                    Default::default()
                },
                m_includeFlags: u16::MAX,
                m_excludeFlags: 0,
                m_traverseFlags: (JumpTypes::SmallObjectsCrossing.into_u32()
                    | JumpTypes::CratesTraversal.into_u32()
                    | JumpTypes::ShortWallTraversal.into_u32()
                    | JumpTypes::ShortJumpsAcrossSameLevel.into_u32())
                    * (hull == Hull::Human) as u32,
            },
            path: Vec::with_capacity(PATH_CAPACITY),
            path_points: Vec::with_capacity(PATH_CAPACITY),
            end_point: None,
            jump_types: vec![u8::MAX; PATH_CAPACITY],
            straigth_path: Vec::with_capacity(PATH_CAPACITY),
            straigth_path_points: Vec::with_capacity(PATH_CAPACITY),
            straigth_path_flags: Vec::with_capacity(PATH_CAPACITY),
            straigth_path_jumps: Vec::with_capacity(PATH_CAPACITY),
        })
    }

    pub fn switch_query(&mut self, hull: Hull) -> Option<()> {
        self.path.clear();
        self.path_points.clear();

        self.query = generate_nav_query(hull, Some(&self.query))?;
        self.extents = hull_to_extents(hull);
        self.hull = hull;

        Some(())
    }

    pub fn new_path(
        &mut self,
        start_point: Vector3,
        end_point: Vector3,
        dt_funcs: &RecastDetour,
    ) -> Result<(), NavigationError> {
        let mut ref_start = 0;
        let mut start = Vector3::ZERO;
        let mut ref_end = 0;
        let mut end = Vector3::ZERO;

        self.end_point = None;

        let status = unsafe {
            if (dt_funcs.dtNavMeshQuery__findNearestPoly)(
                &mut self.query,
                &start_point,
                self.extents,
                &self.filter,
                &mut ref_start,
                &mut start,
            ) == 0x40000000
            {
                (dt_funcs.dtNavMeshQuery__findNearestPoly)(
                    &mut self.query,
                    &end_point,
                    &TARGET_EXTENTS,
                    &self.filter,
                    &mut ref_end,
                    &mut end,
                ) == 0x40000000
            } else {
                false
            }
        };

        if !status || ref_end == 0 || ref_start == 0 {
            log::warn!("failed to find nearest poly, with goal {ref_end} start {ref_start}");
            return Err(NavigationError::NoPoints(start, end, ref_start, ref_end));
        }

        let mut path_size = 0;
        let unk: i64 = 0;

        self.path.clear();

        unsafe {
            (dt_funcs.dtNavMeshQuery__findPath)(
                &mut self.query,
                ref_start,
                ref_end,
                &start,
                &end,
                &self.filter,
                self.path.as_mut_ptr(),
                (&unk as *const i64).cast(),
                &mut path_size,
                self.path.capacity() as i32,
            )
        };

        unsafe {
            self.path
                .set_len(path_size.min(self.path.capacity() as u32) as usize)
        };
        self.path_points.clear();

        if path_size == 0 {
            return Err(NavigationError::PathNotFound(start, end));
        }

        self.end_point = Some(end_point);

        let mut straight_size = 0;
        unsafe {
            (dt_funcs.dtNavMeshQuery__findStraightPath)(
                &mut self.query,
                &start,
                &end,
                self.path.as_ptr(),
                self.jump_types.as_ptr(),
                path_size as i32,
                self.straigth_path_points.as_mut_ptr(),
                self.straigth_path_flags.as_mut_ptr(),
                self.straigth_path.as_mut_ptr(),
                self.straigth_path_jumps.as_mut_ptr(),
                &mut straight_size,
                PATH_CAPACITY as i32,
                0,
                0,
            );
        }

        if straight_size != 0 {
            let straight_size = straight_size.min(self.path.capacity() as i32);
            unsafe {
                self.jump_types.set_len(straight_size as usize);
                self.straigth_path_jumps.set_len(straight_size as usize);
                self.straigth_path_points.set_len(straight_size as usize);
                self.straigth_path.set_len(straight_size as usize);
                self.straigth_path_points.clone_into(&mut self.path_points);
                self.path_points.push(end_point);
                self.path_points.reverse();
                self.straigth_path.clone_into(&mut self.path);
            }
        } else if path_size == 1 {
            self.path_points.push(start_point);
            self.path_points.push(end_point);
        } else {
            let mut prev_point = start;
            self.path
                .iter()
                .cloned()
                .map(|polyref| (polyref, Vector3::ZERO))
                .map(|(next_ref, mut next_point)| {
                    unsafe {
                        (dt_funcs.dtNavMeshQuery__closestPointOnPolyBoundary__variant)(
                            &self.query,
                            next_ref,
                            &prev_point,
                            &mut next_point,
                            &self.filter,
                        );
                        // let mut tile = std::ptr::null_mut();
                        // let mut poly = std::ptr::null_mut();
                        // (dt_funcs.dtNavMesh__)(self.query.nav, next_ref, &mut tile, &mut poly);

                        // let poly = poly.as_mut().unwrap();
                    }
                    prev_point = next_point;
                    next_point
                })
                .rev()
                .collect_into(&mut self.path_points);
        }

        Ok(())
    }

    pub fn next_point(&mut self) -> Option<Vector3> {
        self.path_points.pop()
    }

    pub fn random_point_around(
        &mut self,
        point: Vector3,
        radius: f32,
        min_radius: Option<f32>,
    ) -> Option<Vector3> {
        const MAX_RANDOM_POINTS: usize = 65; // the game uses this amount
        let funcs = RECAST_DETOUR.wait();

        let len = MAX_RANDOM_POINTS.min(self.straigth_path_points.capacity());
        if unsafe {
            (funcs.dtFreeNavMeshQuery_findRandomPointsAroundCircle)(
                &mut self.query,
                &point,
                min_radius.unwrap_or(100f32).min(radius),
                radius,
                &self.filter,
                funcs.some_non_function_function,
                funcs.dtfrand,
                // straigth_path_points is only used as tmp storage so this won't break anything
                len as u32,
                self.straigth_path_points.as_mut_ptr(),
            )
        } == 0x40000000
        {
            // SAFETY: the function should have filled every point
            unsafe {
                self.straigth_path_points.set_len(len);
            }
            self.straigth_path_points
                .get(rand::thread_rng().gen_range(0..self.straigth_path_points.len()))
                .copied()
        } else {
            None
        }
    }
}

impl Drop for Navigation {
    fn drop(&mut self) {
        unsafe { (RECAST_DETOUR.wait().dtFreeNavMeshQuery_Destroy)(&mut self.query) }
    }
}

fn generate_nav_query(hull: Hull, prev_query: Option<&dtNavMeshQuery>) -> Option<dtNavMeshQuery> {
    let mut query = prev_query
        .map(|query| MaybeUninit::new(query.clone()))
        .unwrap_or_else(MaybeUninit::zeroed);

    let dt_funcs = RECAST_DETOUR.wait();

    unsafe {
        (dt_funcs.ZeroOutdtNavMesh)(query.as_mut_ptr());
        let hull_index = (dt_funcs.GetNavMeshHullIndex)(hull as i32);

        if (dt_funcs.dtNavMeshQuery__init)(
            query.as_mut_ptr(),
            *dt_funcs.nav_mesh.add(hull_index as usize).as_ref()?,
            2048,
        ) != 0x40000000
        {
            return None;
        }
    }

    Some(unsafe { query.assume_init() })
}

fn hull_to_extents(hull: Hull) -> &'static Vector3 {
    match hull {
        Hull::Human => &HUMAN_EXTENTS,
        Hull::Medium => &HUMAN_EXTENTS,
        Hull::FlyingVehicle => &TITAN_EXTENTS,
        Hull::Small => &HUMAN_EXTENTS,
        Hull::Titan => &TITAN_EXTENTS,
    }
}
