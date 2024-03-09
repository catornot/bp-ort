use rrplug::prelude::*;
use std::mem::MaybeUninit;
use thiserror::Error;

use super::{
    bindings::{dtNavMeshQuery, dtPolyRef, dtQueryFilter},
    Hull, RecastDetour, RECAST_DETOUR,
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
}

impl Navigation {
    pub fn new(hull: Hull) -> Option<Self> {
        Some(Self {
            query: generate_nav_query(hull)?,
            extents: hull_to_extents(hull),
            hull,
            filter: dtQueryFilter {
                m_areaCost: Default::default(),
                m_includeFlags: u16::MAX,
                m_excludeFlags: 0,
            },
            path: Vec::with_capacity(PATH_CAPACITY),
            path_points: Vec::with_capacity(PATH_CAPACITY),
        })
    }

    pub fn switch_query(&mut self, hull: Hull) -> Option<Self> {
        self.path.clear();
        self.path_points.clear();

        Some(Self {
            query: generate_nav_query(hull)?,
            extents: hull_to_extents(hull),
            hull,
            filter: self.filter.clone(),
            path: self.path.clone(),
            path_points: self.path_points.clone(),
        })
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

        let status = unsafe {
            (dt_funcs.dtNavMeshQuery__findNearestPoly)(
                &mut self.query,
                &start_point,
                self.extents,
                &self.filter,
                &mut ref_start,
                &mut start,
            )
            .eq(&0x40000000)
            .then(|| {
                (dt_funcs.dtNavMeshQuery__findNearestPoly)(
                    &mut self.query,
                    &end_point,
                    &TARGET_EXTENTS,
                    &self.filter,
                    &mut ref_end,
                    &mut end,
                )
                .eq(&0x40000000)
            })
            .unwrap_or(false)
        };

        if !status || ref_end == 0 || ref_start == 0 {
            log::warn!(
                "failed to find nearest poly, with goal {} start {}",
                ref_end,
                ref_start
            );
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

        unsafe { self.path.set_len(path_size as usize) };
        self.path_points.clear();

        if path_size == 0 {
            return Err(NavigationError::PathNotFound(start, end));
        }

        if path_size == 1 {
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
                        )
                    };
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

    // pub(crate) fn reach_next_point(&mut self, local_data: &mut BotData, cmd: &mut CUserCmd) {}
}

impl Drop for Navigation {
    fn drop(&mut self) {
        unsafe { (RECAST_DETOUR.wait().dtFreeNavMeshQuery_Destroy)(&mut self.query) }
    }
}

fn generate_nav_query(hull: Hull) -> Option<dtNavMeshQuery> {
    let mut query = MaybeUninit::zeroed();

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
