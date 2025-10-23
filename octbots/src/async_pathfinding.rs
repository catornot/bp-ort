use oktree::prelude::*;
use parking_lot::RwLock;
use rrplug::prelude::*;
use std::{
    sync::Arc,
    thread::{self, available_parallelism, JoinHandle},
    time::Duration,
};

use crate::{
    loader::{map_to_i32, map_to_u32, Navmesh, NavmeshStatus},
    pathfinding::find_path,
};

pub type PathReceiver = flume::Receiver<Option<Vec<Vector3>>>;

pub struct Work {
    pub start: Vector3,
    pub end: Vector3,
    pub return_sender: flume::Sender<Option<Vec<Vector3>>>,
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
    pub fn new(navmesh: Arc<RwLock<Navmesh>>) -> JobMarket {
        let (sender, receiver) = flume::unbounded();
        let receiver = Arc::new(receiver);
        let cores = available_parallelism()
            .map(|cores| cores.get())
            .unwrap_or(8);
        log::info!("spawning {cores} worker threads");
        JobMarket {
            workers: (0..cores)
                .map(|_| thread::spawn(worker(Arc::clone(&receiver), Arc::clone(&navmesh))))
                .collect(),
            job_sender: sender,
        }
    }

    pub fn find_path(&self, start: Vector3, end: Vector3) -> Option<PathReceiver> {
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

    pub fn stop(&self) {
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
    job_receiver: Arc<flume::Receiver<JobMessage>>,
    navmesh: Arc<RwLock<Navmesh>>,
) -> impl Fn() {
    move || {
        while let Ok(JobMessage::Work(Work {
            start,
            end,
            return_sender,
        })) = job_receiver.recv()
        {
            let navmesh = navmesh.read();
            let NavmeshStatus::Loaded(navmesh_tree) = &navmesh.navmesh else {
                log::warn!("tried pathfinding without a navmesh");
                continue;
            };

            _ = return_sender.send(
                find_path(
                    navmesh_tree,
                    vector3_to_tuvec(navmesh.cell_size, start),
                    vector3_to_tuvec(navmesh.cell_size, end),
                )
                .map(|points| {
                    // string_pulling(
                    points
                        .into_iter()
                        .map(|point| tuvec_to_vector3(navmesh.cell_size, point))
                        .collect()
                    // )
                }),
            );
        }
    }
}

fn vector3_to_tuvec(cell_size: f32, origin: Vector3) -> TUVec3u32 {
    let scaled = origin / Vector3::new(cell_size, cell_size, cell_size);

    TUVec3u32::new(
        map_to_u32(scaled.x as i32),
        map_to_u32(scaled.y as i32),
        map_to_u32(scaled.z as i32),
    )
}

fn tuvec_to_vector3(cell_size: f32, point: TUVec3u32) -> Vector3 {
    Vector3::new(cell_size, cell_size, cell_size)
        * Vector3::new(
            map_to_i32(point.0.x) as f32,
            map_to_i32(point.0.y) as f32,
            map_to_i32(point.0.z) as f32,
        )
}
