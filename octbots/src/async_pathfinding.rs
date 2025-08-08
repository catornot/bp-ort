use oktree::prelude::*;
use parking_lot::RwLock;
use std::{
    sync::{mpmc, Arc},
    thread::{self, available_parallelism, JoinHandle},
};

use crate::{
    loader::{Navmesh, NavmeshStatus},
    pathfinding::find_path,
};

pub type PathReceiver = mpmc::Receiver<Option<Vec<TUVec3u32>>>;

pub struct Work {
    pub start: TUVec3u32,
    pub end: TUVec3u32,
    pub return_sender: mpmc::Sender<Option<Vec<TUVec3u32>>>,
}

enum JobMessage {
    Work(Work),
    Stop,
}

pub struct JobMarket {
    workers: Vec<JoinHandle<()>>,
    job_sender: mpmc::Sender<JobMessage>,
}

impl JobMarket {
    pub fn new(navmesh: Arc<RwLock<Navmesh>>) -> JobMarket {
        let (sender, receiver) = mpmc::channel();
        JobMarket {
            workers: (0..available_parallelism()
                .map(|cores| cores.get())
                .unwrap_or(8))
                .map(|_| thread::spawn(worker(receiver.clone(), Arc::clone(&navmesh))))
                .collect(),
            job_sender: sender,
        }
    }

    pub fn find_path(&self, start: TUVec3u32, end: TUVec3u32) -> Option<PathReceiver> {
        let (sender, receiver) = mpmc::channel();

        self.job_sender
            .send(JobMessage::Work(Work {
                start,
                end,
                return_sender: sender,
            }))
            .ok()?;

        Some(receiver)
    }

    pub fn stop(&self) {
        for _ in 0..self.workers.len() {
            _ = self.job_sender.send(JobMessage::Stop);
        }
    }
}

fn worker(job_receiver: mpmc::Receiver<JobMessage>, navmesh: Arc<RwLock<Navmesh>>) -> impl Fn() {
    move || {
        while let Ok(JobMessage::Work(Work {
            start,
            end,
            return_sender,
        })) = job_receiver.recv()
        {
            let NavmeshStatus::Loaded(navmesh_tree) = &navmesh.read().navmesh else {
                log::warn!("tried pathfinding without a navmesh");
                continue;
            };

            _ = return_sender.send(find_path(navmesh_tree, start, end));
        }
    }
}
