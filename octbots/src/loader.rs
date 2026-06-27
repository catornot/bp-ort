use bspeater_lib::saving::{NAVMESH_VERSION, Navmesh as NavmeshBin};
use oktree::prelude::*;
use rkyv::api::low::from_bytes;
use rrplug::high::filesystem;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicI32, Ordering},
    thread::{Builder, JoinHandle},
};

pub type Octree32 = Octree<u32, TUVec3u32>;

#[derive(Default, Debug)]
pub struct Navmesh {
    pub navmesh: NavmeshStatus,
    loading_thread: Option<JoinHandle<Option<(Octree32, f32)>>>,
    building_thread: Option<JoinHandle<Option<()>>>,
    id: String,
    did_build: bool,
    can_build: bool,
    pub cell_size: f32,
}

#[derive(Default, Debug)]
pub enum NavmeshStatus {
    #[default]
    Unloaded,
    Loading,
    Loaded(Octree32),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum LoadingStatus<'a> {
    Loading(&'a str),
    Loaded(&'a str),
}

struct VPKFileReader;

impl bspeater_lib::VPKReader for VPKFileReader {
    fn read_vpk_file(&self, path: &Path) -> Result<Vec<u8>, io::Error> {
        log::info!("reading {path:?}");
        let file = filesystem::open(path).map_err(|err| io::Error::other(format!("{err:?}")))?;
        let mut buf = Vec::new();

        file.reader().read_to_end(&mut buf)?;

        Ok(buf)
    }
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
    pub fn load_navmesh(&mut self, id: &str) -> LoadingStatus<'_> {
        match &mut self.navmesh {
            NavmeshStatus::Unloaded => {
                self.did_build = false;
                self.id = id.to_owned();
                self.spawn_load_worker();
                LoadingStatus::Loading(&self.id)
            }
            NavmeshStatus::Loading => LoadingStatus::Loading(&self.id),
            NavmeshStatus::Loaded(_) if id != self.id => {
                self.drop_navmesh();
                self.load_navmesh(id)
            }
            NavmeshStatus::Loaded(_) => LoadingStatus::Loaded(&self.id),
        }
    }

    /// try to mount the a navmesh that can be potentially loaded
    pub fn try_loaded(&mut self) -> Option<bool> {
        if self.navmesh.get().is_some() {
            return Some(true);
        }

        if self
            .loading_thread
            .as_ref()
            .map(|thread| !thread.is_finished())
            .unwrap_or(true)
        {
            return Some(false);
        }

        if let Some(thread) = self.loading_thread.take() {
            if let Some((octree, cell_size)) = thread.join().ok().flatten() {
                self.navmesh = NavmeshStatus::Loaded(octree);
                self.cell_size = cell_size;

                log::info!("loaded {}.navmesh", self.id);
                Some(true)
            } else {
                self.navmesh = NavmeshStatus::Unloaded;
                log::warn!("no octtree found when trying to load from async worker");

                self.spawn_builder_worker();
                None
            }
        } else {
            Some(false)
        }
    }

    pub fn try_build_status(&mut self) {
        if let Some(result) = self
            .building_thread
            .as_ref()
            .filter(|thread| thread.is_finished())
            .map(|_| {})
            .map(|_| self.building_thread.take().unwrap().join())
        {
            match result {
                Ok(Some(_)) => _ = self.spawn_load_worker(),
                Ok(None) | Err(_) => {
                    _ = self.building_thread.take();
                    log::warn!("failed building navmesh : {}.navmesh", self.id);
                }
            }
        }
    }

    pub fn drop_navmesh(&mut self) {
        log::info!("dropping {}", self.id);
        _ = self.loading_thread.take();
        _ = self.building_thread.take();
        self.navmesh = NavmeshStatus::Unloaded;
    }

    pub fn allow_building(&mut self) {
        self.can_build = true;
    }

    pub fn is_building(&self) -> bool {
        self.building_thread.is_some()
    }

    fn spawn_load_worker(&mut self) -> Option<()> {
        self.loading_thread.replace(
            Builder::new()
                .name(format!("loading {}", self.id))
                .spawn(async_load_worker_builder(self.id.clone()))
                .ok()?,
        );
        self.navmesh = NavmeshStatus::Loading;
        None
    }

    fn spawn_builder_worker(&mut self) -> Option<()> {
        if self.did_build || !self.can_build {
            return None;
        }

        self.building_thread.replace(
            Builder::new()
                .name(format!("building {}", self.id))
                .spawn(async_build_worker_builder(self.id.clone()))
                .ok()?,
        );
        self.did_build = true;
        None
    }
}

fn async_load_worker_builder(id: String) -> impl FnOnce() -> Option<(Octree32, f32)> {
    move || {
        let profile = std::env::args()
            .find(|arg| arg.starts_with("-profile"))
            .and_then(|profile| {
                profile
                    .split_once('=')
                    .map(|(_, profile)| profile.to_string())
            })
            .unwrap_or_else(|| "R2Northstar".to_string());

        let mut file = std::fs::File::open(format!("output/{id}.navmesh"))
            .or_else(|_| std::fs::File::open(format!("{profile}/octnavs/{id}.navmesh")))
            .inspect_err(|err| {
                log::warn!("failed loading {id}: {err}; bots possibly would be unable to move")
            })
            .ok()?;
        let mut buf = Vec::with_capacity(10000);
        file.read_to_end(&mut buf)
            .inspect_err(|err| log::warn!("failed reading during loading {id}: {err}"))
            .ok()?;

        let navmesh = from_bytes::<NavmeshBin, rkyv::rancor::Error>(&buf)
            .inspect_err(|err| log::warn!("failed parsing {id}: {err}"))
            .ok()?;

        if navmesh.version != NAVMESH_VERSION {
            log::warn!("tried loading wrong version of the navmesh");
            return None;
        }

        OFFSET.store(
            navmesh
                .min
                .iter()
                .min()
                .copied()
                .unwrap_or(OFFSET.load(Ordering::Relaxed))
                .abs(),
            Ordering::Relaxed,
        );

        let (min, max) = (
            navmesh
                .min
                .map(map_to_u32)
                .iter()
                .min()
                .copied()
                .unwrap_or(0),
            navmesh
                .max
                .map(map_to_u32)
                .iter()
                .max()
                .copied()
                .unwrap_or_else(|| unreachable!()),
        );

        log::info!("pre oct init");
        // swizzle here
        let mut octree: Octree32 = Octree::from_aabb_with_capacity(
            dbg!(Aabb::from_min_max(
                TUVec3 {
                    x: round_down_to_power_of_2(min),
                    y: round_down_to_power_of_2(min),
                    z: round_down_to_power_of_2(min),
                },
                TUVec3 {
                    x: round_up_to_power_of_2(max),
                    y: round_up_to_power_of_2(max),
                    z: round_up_to_power_of_2(max),
                },
            )),
            navmesh.filled_pos.len(),
        );
        log::info!("post oct init {}", octree.len());

        let mut err = String::new();
        navmesh
            .filled_pos
            .iter()
            .map(|pos| pos.map(map_to_u32))
            // swizzle here too
            .for_each(|pos| {
                _ = octree
                    .insert(TUVec3u32::new(pos[0], pos[2], pos[1]))
                    .inspect_err(|thiserr| err = thiserr.to_string());
            });

        log::info!("post oct fill {} {err}", octree.iter_elements().count());

        Some((octree, navmesh.cell_size))
    }
}

fn async_build_worker_builder(id: String) -> impl FnOnce() -> Option<()> {
    move || {
        let profile = std::env::args()
            .find(|arg| arg.starts_with("-profile"))
            .and_then(|profile| {
                profile
                    .split_once('=')
                    .map(|(_, profile)| profile.to_string())
            })
            .unwrap_or_else(|| "R2Northstar".to_string());

        let navmesh_output_path = PathBuf::from(format!("{profile}/octnavs"));
        if let Err(err) = fs::create_dir_all(&navmesh_output_path) {
            log::warn!("failed to create octnavs dir; can't build navmeshes; {err}");
            return None;
        }

        let file = fs::File::create_new(navmesh_output_path.join(&id).with_extension("navmesh"))
            .inspect_err(|err| {
                log::warn!("failed to open navmesh file while building {id}: {err}; bots possibly would be unable to move")
            })
            .ok()?;

        _ = file;

        let Ok(bsp) = filesystem::open(&PathBuf::from("maps").join(&id).with_extension("bsp"))
            .inspect_err(|err| err.log())
        else {
            return None;
        };

        let mut buf = Vec::new();
        if let Err(err) = bsp.reader().read_to_end(&mut buf) {
            log::warn!("{err:?}");
            return None;
        }

        if let Err(err) = bspeater_lib::create_navmesh(
            &id,
            io::Cursor::new(buf),
            VPKFileReader,
            navmesh_output_path,
        ) {
            log::warn!("failed to build navmesh {err:?}");
            return None;
        }

        Some(())
    }
}

fn round_up_to_power_of_2(mut num: u32) -> u32 {
    num = num.wrapping_sub(1);
    num |= num >> 1;
    num |= num >> 2;
    num |= num >> 4;
    num |= num >> 8;
    num |= num >> 16;
    num.wrapping_add(1)
}

fn round_down_to_power_of_2(num: u32) -> u32 {
    round_up_to_power_of_2(num) >> 1
}

static OFFSET: AtomicI32 = AtomicI32::new(i32::MAX / 2);
pub fn map_to_u32(value: i32) -> u32 {
    (value + OFFSET.load(Ordering::Relaxed)) as u32
}

pub fn map_to_i32(value: u32) -> i32 {
    value as i32 - OFFSET.load(Ordering::Relaxed)
}
