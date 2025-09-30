use bevy::prelude::*;
use bonsai_bt::{Action, BT, Event, RUNNING, Sequence, Status, UpdateArgs};
use itertools::Itertools;
use std::sync::Arc;

use crate::{
    async_pathfinding::{JobMarket, PathReceiver},
    debug::Navmesh,
};

pub struct BotBrain {
    _navmesh: Arc<Navmesh>,
    path_receiver: Option<PathReceiver>,
    path: Vec<Vec3>,
}

#[derive(Debug, Clone)]
pub enum BotAction {
    RenderPath,
    FindPath,
    CheckNavmesh,
}

pub type Behavior = BT<BotAction, BotBrain>;

pub fn init_pathfinding(navmesh: Arc<Navmesh>) -> Behavior {
    let routine = Sequence(vec![
        Action(BotAction::CheckNavmesh),
        Action(BotAction::FindPath),
        Action(BotAction::RenderPath),
    ]);

    BT::new(
        routine,
        BotBrain {
            _navmesh: navmesh,
            path_receiver: None,
            path: Vec::new(),
        },
    )
}

pub fn run_behavior(
    bt: &mut Behavior,
    delta: f64,
    endpoints: [Vec3; 2],
    job_market: &JobMarket,
    mut gizmos: Gizmos,
) {
    let e = Event::from(UpdateArgs { dt: delta });

    bt.tick(&e, &mut |args, brain| match args.action {
        BotAction::CheckNavmesh => (Status::Success, 0.),
        BotAction::FindPath => {
            let [start, end] = endpoints;

            if brain.path_receiver.as_ref().is_none() {
                // log::info!("pathfinding from {start:?} to {end:?}");
                brain.path_receiver = job_market.find_path(start, end);
            }

            let status = if let Some(path_receiver) = brain.path_receiver.as_ref() {
                match path_receiver.try_recv() {
                    Ok(Some(path)) => {
                        brain.path = path;

                        bevy::log::info!("found path");
                        (Status::Success, 0.)
                    }
                    Ok(None) => {
                        bevy::log::info!("path failed");
                        (Status::Failure, 0.)
                    }
                    Err(flume::TryRecvError::Disconnected) => (Status::Failure, 0.),
                    Err(flume::TryRecvError::Empty) => RUNNING,
                }
            } else {
                (Status::Failure, 0.)
            };

            if status != RUNNING {
                brain.path_receiver.take();
            }
            status
        }
        BotAction::RenderPath => 'render: {
            if brain.path.is_empty() {
                break 'render (Status::Failure, 0.);
            }

            bevy::log::info!("path render");
            brain
                .path
                .iter()
                .cloned()
                .tuple_windows()
                .for_each(|(p1, p2)| gizmos.line(p1, p2, Color::linear_rgba(0., 0.9, 0.1, 1.0)));

            (Status::Success, 0.)
        }
    });

    if bt.is_finished() {
        bt.reset_bt();
    }
}
