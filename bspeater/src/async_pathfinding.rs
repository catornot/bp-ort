use bevy::math::{IVec3, Vec3};
use oktree::prelude::*;
use parking_lot::Mutex;
use std::{
    sync::Arc,
    thread::{self, JoinHandle, available_parallelism},
    time::Duration,
};

use crate::{debug::Navmesh, map_to_i32, map_to_u32, pathfinding::find_path};

pub type PathReceiver = flume::Receiver<Option<Vec<Vec3>>>;

pub struct Work {
    pub start: Vec3,
    pub end: Vec3,
    pub return_sender: flume::Sender<Option<Vec<Vec3>>>,
}

enum JobMessage {
    Work(Work),
    Stop,
}

pub struct JobMarket {
    workers: Vec<JoinHandle<()>>,
    job_sender: flume::Sender<JobMessage>,
}

impl JobMarket {
    pub fn new(navmesh: Arc<Navmesh>) -> JobMarket {
        let (sender, receiver) = flume::unbounded();
        let receiver = Arc::new(Mutex::new(receiver));
        JobMarket {
            workers: (0..available_parallelism()
                .map(|cores| cores.get())
                .unwrap_or(8))
                .map(|_| thread::spawn(worker(Arc::clone(&receiver), Arc::clone(&navmesh))))
                .collect(),
            job_sender: sender,
        }
    }

    pub fn find_path(&self, start: Vec3, end: Vec3) -> Option<PathReceiver> {
        let (sender, receiver) = flume::unbounded();

        self.job_sender
            .send(JobMessage::Work(Work {
                start,
                end,
                return_sender: sender,
            }))
            .ok()?;
        const fn check_sync<T: Sync + Send>() {}

        check_sync::<PathReceiver>();

        Some(receiver)
    }

    pub fn _stop(&self) {
        for _ in 0..self.workers.len() {
            _ = self.job_sender.send(JobMessage::Stop);
        }

        // give threads time to end
        thread::sleep(Duration::from_secs(1));

        // self.workers.iter().for_each(|worker| {
        //     let start = SystemTime::now();
        //     while !worker.is_finished()
        //     && start.elapsed().unwrap_or(Duration::new(1, 0)) >= Duration::new(1, 0)
        //     {
        //         thread::sleep(Duration::from_millis(10));
        //     }
        // });
    }
}

fn worker(
    job_receiver: Arc<Mutex<flume::Receiver<JobMessage>>>,
    navmesh: Arc<Navmesh>,
) -> impl Fn() {
    move || {
        while let Ok(JobMessage::Work(Work {
            start,
            end,
            return_sender,
        })) = job_receiver.lock_arc().recv()
        {
            _ = return_sender.send(
                find_path(
                    &navmesh.navmesh_tree,
                    vector3_to_tuvec(navmesh.cell_size, start),
                    vector3_to_tuvec(navmesh.cell_size, end),
                )
                .map(|points| {
                    points
                        .into_iter()
                        .map(|point| tuvec_to_vector3(navmesh.cell_size, point))
                        .collect()
                }),
            );
        }
    }
}

fn vector3_to_tuvec(cell_size: f32, origin: Vec3) -> TUVec3u32 {
    let scaled = (origin / Vec3::splat(cell_size)).as_ivec3();

    TUVec3u32::new(
        map_to_u32(scaled.x),
        map_to_u32(scaled.y),
        map_to_u32(scaled.z),
    )
}

fn tuvec_to_vector3(cell_size: f32, point: TUVec3u32) -> Vec3 {
    Vec3::splat(cell_size)
        * IVec3::new(
            map_to_i32(point.0.x),
            map_to_i32(point.0.y),
            map_to_i32(point.0.z),
        )
        .as_vec3()
}
