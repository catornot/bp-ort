use oktree::prelude::*;
use parking_lot::Mutex;
use std::thread::{Builder, JoinHandle, ThreadId};

use crate::NavmeshBin;

type Octree32 = Octree<u32, TUVec3u32>;

#[derive(Default, Debug)]
pub struct Navmesh {
    pub navmesh: NavmeshStatus,
    loading_thread: Option<JoinHandle<Option<Octree32>>>,
    id: String,
}

#[derive(Default, Debug)]
pub enum NavmeshStatus {
    #[default]
    Unloaded,
    Loading,
    Loaded(Octree32),
}

#[derive(Debug)]
pub enum LoadingStatus<'a> {
    Loading(&'a str),
    Loaded(&'a str),
}

impl NavmeshStatus {
    pub fn get(&self) -> Option<&Octree32> {
        match self {
            NavmeshStatus::Loaded(octree) => Some(octree),
            _ => None,
        }
    }
}

impl Navmesh {
    pub fn load_navmesh(&mut self, id: &str) -> LoadingStatus {
        match &mut self.navmesh {
            NavmeshStatus::Unloaded => {
                self.id = id.to_owned();
                self.spawn_load_worker();
                LoadingStatus::Loading(&self.id)
            }
            NavmeshStatus::Loading => LoadingStatus::Loading(&self.id),
            NavmeshStatus::Loaded(_) => LoadingStatus::Loaded(&self.id),
        }
    }

    /// try to mount the a navmesh that can be potentially loaded
    pub fn try_loaded(&mut self) -> Option<bool> {
        if self
            .loading_thread
            .as_ref()
            .map(|thread| thread.is_finished())
            .unwrap_or(true)
        {
            return Some(false);
        }

        if let Some(thread) = self.loading_thread.take() {
            if let Some(octree) = thread.join().ok().flatten() {
                self.navmesh = NavmeshStatus::Loaded(octree);
                Some(true)
            } else {
                log::warn!("no octtree found when trying to load from async worker");
                None
            }
        } else {
            Some(false)
        }
    }

    pub fn drop_navmesh(&mut self) {
        _ = self.loading_thread.take();
        self.navmesh = NavmeshStatus::Unloaded;
    }

    fn spawn_load_worker(&mut self) -> Option<()> {
        self.loading_thread.replace(
            Builder::new()
                .name(format!("loading {}", self.id))
                .spawn(async_load_worker_builder(self.id.clone()))
                .ok()?,
        );
        None
    }
}

fn async_load_worker_builder(id: String) -> impl FnOnce() -> Option<Octree32> {
    move || {
        let offset = (u32::MAX / 2) as i32;
        let navmesh = bincode::decode_from_std_read::<NavmeshBin, _, _>(
            &mut std::fs::File::open(format!("output/{id}.navmesh"))
                .inspect_err(|err| log::error!("failed loading navmesh file: {err}"))
                .ok()?,
            bincode::config::standard(),
        )
        .inspect_err(|err| log::error!("failed parsing navmesh file: {err}"))
        .ok()?;

        let map_to_u32 = |value| (value + offset) as u32;
        let (min, max) = (navmesh.min.map(map_to_u32), navmesh.max.map(map_to_u32));

        // swizzle here
        let mut octree: Octree32 = Octree::from_aabb(Aabb::from_min_max(
            TUVec3 {
                x: min[0],
                y: min[2],
                z: min[1],
            },
            TUVec3 {
                x: max[0],
                y: max[2],
                z: max[1],
            },
        ));

        navmesh
            .filled_pos
            .iter()
            .map(|pos| pos.map(map_to_u32))
            // swizzle here too
            .for_each(|pos| _ = octree.insert(TUVec3u32::new(pos[0], pos[2], pos[2])));

        Some(octree)
    }
}
