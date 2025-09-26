use bevy_math::{NormedVectorSpace, Vec3};
use itertools::Itertools;
use oktree::prelude::*;
use parking_lot::{Mutex, RwLock};
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
    job_receiver: Arc<Mutex<flume::Receiver<JobMessage>>>,
    navmesh: Arc<RwLock<Navmesh>>,
) -> impl Fn() {
    move || {
        while let Ok(JobMessage::Work(Work {
            start,
            end,
            return_sender,
        })) = job_receiver.lock_arc().recv()
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

fn string_pulling(mut positions: Vec<Vector3>) -> Vec<Vector3> {
    let positions_vec3 = positions
        .iter()
        .map(|point| Vec3::from_array([point.x, point.y, point.z]))
        .collect_vec();
    douglas_peucker(&positions_vec3, &mut positions)
        .map(|_| positions)
        .unwrap_or_else(|| {
            positions_vec3
                .into_iter()
                .map(|point| Vector3::from(point.to_array()))
                .collect_vec()
        })
    // positions
    //     .into_iter()
    //     .tuple_windows()
    //     .filter_map(|(p1, p2, p3)| ((p1 - p2) != (p2 - p3)).then_some(p2))
    //     .collect()
}

const EPSILON: f32 = 0.01;
fn douglas_peucker(points: &[Vec3], output: &mut Vec<Vector3>) -> Option<()> {
    if points.len() < 3 {
        output.extend(points.iter().map(|points| Vector3::from(points.to_array())));
        return Some(());
    }

    let (index, max_distance) = points
        .iter()
        .skip(1)
        .take(points.len().saturating_sub(1))
        .copied()
        .enumerate()
        .filter_map(|(i, point)| {
            Some((
                i + 1,
                perpendicular_distance([*points.first()?, *points.last()?], point),
            ))
        })
        .fold(
            (0, f32::MIN),
            |(index_max, max_distance), (index, distance)| {
                if max_distance < distance {
                    (index, distance)
                } else {
                    (index_max, max_distance)
                }
            },
        );

    if max_distance > EPSILON {
        douglas_peucker(points.get(..=index).expect("index oob"), output)?;
        douglas_peucker(points.get(index..).expect("first index oob"), output)?;
    } else if !points.is_empty() {
        output.push(Vector3::from(points.first()?.to_array()));
        output.push(Vector3::from(points.last()?.to_array()));
    }

    Some(())
}

fn perpendicular_distance(line: [Vec3; 2], off_point: Vec3) -> f32 {
    if line[1] == line[0] {
        return (off_point - line[0]).norm();
    }

    (line[1] - line[0]).cross(line[0] - off_point).norm().abs() / (line[1] - line[0]).norm()
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
