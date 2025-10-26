use oktree::prelude::*;
use rrplug::prelude::*;

use crate::loader::{map_to_i32, map_to_u32};

#[derive(Debug, Clone)]
pub struct NavPoint {
    world: Vector3,
    nav: TUVec3u32,
    ground_distance: u32,
}

impl AsRef<Vector3> for NavPoint {
    fn as_ref(&self) -> &Vector3 {
        &self.world
    }
}

impl AsRef<TUVec3u32> for NavPoint {
    fn as_ref(&self) -> &TUVec3u32 {
        &self.nav
    }
}

impl AsRef<TUVec3<u32>> for NavPoint {
    fn as_ref(&self) -> &TUVec3<u32> {
        &self.nav.0
    }
}

impl AsRef<u32> for NavPoint {
    fn as_ref(&self) -> &u32 {
        &self.ground_distance
    }
}

impl std::ops::Deref for NavPoint {
    type Target = Vector3;

    fn deref(&self) -> &Self::Target {
        &self.world
    }
}

impl std::ops::DerefMut for NavPoint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.world
    }
}

impl NavPoint {
    pub fn new(nav: TUVec3u32, ground_distance: u32, cell_size: f32) -> Self {
        Self {
            world: tuvec_to_vector3(cell_size, nav),
            nav,
            ground_distance,
        }
    }

    pub fn as_vec(&self) -> Vector3 {
        self.world
    }

    pub fn as_point(&self) -> TUVec3u32 {
        self.nav
    }

    pub fn as_distance(&self) -> u32 {
        self.ground_distance
    }
}

pub fn vector3_to_tuvec(cell_size: f32, origin: Vector3) -> TUVec3u32 {
    let scaled = origin / Vector3::new(cell_size, cell_size, cell_size);

    TUVec3u32::new(
        map_to_u32(scaled.x as i32),
        map_to_u32(scaled.y as i32),
        map_to_u32(scaled.z as i32),
    )
}

pub fn tuvec_to_vector3(cell_size: f32, point: TUVec3u32) -> Vector3 {
    Vector3::new(cell_size, cell_size, cell_size)
        * Vector3::new(
            map_to_i32(point.0.x) as f32,
            map_to_i32(point.0.y) as f32,
            map_to_i32(point.0.z) as f32,
        )
}
