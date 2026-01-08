use indexmap::{map::Entry, IndexMap};
use oktree::prelude::*;
use rustc_hash::{FxHashMap, FxHasher};
use std::{cmp::Reverse, collections::BinaryHeap, hash::BuildHasherDefault, ops::Not};

use crate::{loader::Octree32, nav_points::NavPoint};

type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;
type Cost = f64;
type VisitedList = FxIndexMap<TUVec3u32, Visits>;
pub type AreaCost = FxHashMap<TUVec3u32, Cost>;

/// this would work with the current grid size ...
const WALLRUN_MAX_DISTANCE: u32 = 20;
pub const DEFAULT_MAX_ITERATIONS: usize = u16::MAX as usize * 30;

#[derive(Debug, Clone, Copy)]
pub enum Goal {
    Point(TUVec3u32),
    ClosestToPoint(TUVec3u32),
    /// more like max distance honestly
    Distance(usize),
    Area(TUVec3u32, f64),
}

#[derive(Debug, Clone, Copy)]
pub struct Node {
    index: usize,
    cost: Cost,
    estimated_cost: Cost,
}

/// a place to visit or visited
#[derive(Debug, Clone, Copy)]
pub struct Visits {
    parent: usize,
    cost: Cost,
    ground_distance: u32,
    wallrun_distance: u32,
    /// the the wall jump been used?
    wallhop: bool,
    // when was it used?
    wallhop_offset: u32,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.estimated_cost.total_cmp(&other.estimated_cost)
    }
}
impl Eq for Node {}

impl Goal {
    pub fn is_inside(&self, point: &TUVec3u32) -> bool {
        match self {
            Goal::Point(tuvec3u32) => point == tuvec3u32,
            Goal::ClosestToPoint(tuvec3u32) => point == tuvec3u32,
            Goal::Distance(_) => false,
            Goal::Area(_, _) => self.distance(point) <= 0.,
        }
    }

    pub fn distance(&self, point: &TUVec3u32) -> f64 {
        fn distance3(pos: &TUVec3u32, target: &TUVec3u32) -> f64 {
            (((pos.0.x as i64 - target.0.x as i64).pow(2)
                + (pos.0.y as i64 - target.0.y as i64).pow(2)
                + (pos.0.z as i64 - target.0.z as i64).pow(2)) as f64)
                .sqrt()
        }

        match self {
            Goal::Point(tuvec3u32) => distance3(point, tuvec3u32),
            Goal::ClosestToPoint(tuvec3u32) => distance3(point, tuvec3u32),
            Goal::Distance(_) => 1.,
            Goal::Area(tuvec3u32, radius) => (distance3(point, tuvec3u32) - *radius).max(0.),
        }
    }
}

pub fn find_path<const MAX_ITERATIONS: usize>(
    octtree: &Octree32,
    area_cost: AreaCost,
    start: TUVec3u32,
    end: Goal,
    cell_size: f32,
) -> Option<Vec<NavPoint>> {
    // log::info!("{start:?} and {end:?}");
    if octtree.get(&start.0).is_some()
        || match end {
            Goal::Point(tuvec3u32) | Goal::ClosestToPoint(tuvec3u32) | Goal::Area(tuvec3u32, _) => {
                octtree.get(&tuvec3u32.0).is_some()
            }
            Goal::Distance(_) => false,
        }
    {
        return None;
    }

    // posistions to be evaluated
    let mut open_list = BinaryHeap::<Reverse<Node>>::new();

    let mut visited_list = VisitedList::default();

    let start_index = visited_list
        .insert_full(
            start,
            Visits {
                parent: usize::MAX,
                cost: 0.,
                ground_distance: find_ground_distance(octtree, start),
                wallrun_distance: 0,
                wallhop: false,
                wallhop_offset: 0,
            },
        )
        .0;
    open_list.push(Reverse(Node {
        index: start_index,
        cost: 0.,
        estimated_cost: 0.,
    }));

    let mut iterations = 0usize;
    while let Some(Reverse(node)) = open_list.pop()
        && iterations < MAX_ITERATIONS
    {
        iterations += 1;
        let (
            &node_pos,
            &Visits {
                parent,
                cost,
                ground_distance,
                wallrun_distance,
                wallhop,
                wallhop_offset,
            },
        ) = visited_list.get_index(node.index).unwrap();

        // If cost of new node from BinaryHeap is higher than the best cost, skip it
        // This implies we've already found a better path to this node
        if node.cost > cost
            && (visited_list
                .get_index(parent)
                .map(
                    |(
                        parent_pos,
                        Visits {
                            parent: _,
                            cost: _,
                            ground_distance: parent_distance,
                            wallrun_distance,
                            wallhop: _,
                            wallhop_offset: _,
                        },
                    )| { (parent_pos, *parent_distance, *wallrun_distance) },
                )
                .map(|(parent_pos, parent_distance, wallrun_distance)| {
                    fall_condition(&node_pos, ground_distance, parent_pos, parent_distance)
                        && wallrun_distance < WALLRUN_MAX_DISTANCE
                })
                .unwrap_or_default()
                || pass_ground_distance(ground_distance)
                || pass_hop_distance(ground_distance, wallhop_offset))
        {
            continue;
        }

        // Check if we've reached the goal
        if end.is_inside(&node_pos)
            || (matches!(end, Goal::Distance(distance) if path_length(&visited_list, node.index, start_index ) >= distance)
                && visited_list
                    .get_index(node.index)
                    .map(|(point, _)| pass_ground_distance(find_ground_distance(octtree, *point)))
                    .unwrap_or_default())
        {
            return get_path(visited_list, node.index, start_index, octtree, cell_size);
        }

        for (neighbor, edge_cost) in get_neighbors(
            octtree,
            &visited_list,
            &node_pos,
            ground_distance,
            wallhop_offset,
        ) {
            // new cost to reach this node = edge cost + node cost
            // This is confirmed cost, not heuristic
            let new_cost = edge_cost + cost;

            // calculate heuristic cost
            let neighboor_ground_distance = find_ground_distance(octtree, neighbor);
            let estimated_cost = heuristic(neighbor, start, end)
                + area_cost.get(&neighbor).copied().unwrap_or_default();
            let wallrun_distance = if find_wall_point(node_pos, octtree).is_some() {
                wallrun_distance + 1
            } else {
                0
            };
            // wallhop can only happen when the bot is on the ground and somewhere before the max jump distance
            let wallhop = (wallhop || neighboor_ground_distance <= 1)
                && pass_ground_distance(neighboor_ground_distance);
            let wallhop_offset = if get_neighbors_h(node_pos, octtree)
                .filter(|&(_, is_empty)| is_empty)
                .count()
                < 4
            {
                wallhop_offset
            } else {
                neighboor_ground_distance.min(MAX_DISTANCE)
            };

            let neighbor_index = match visited_list.entry(neighbor) {
                Entry::Vacant(entry) => {
                    // This is the first time we're seeing this neighbor
                    let index = entry.index();
                    entry.insert(Visits {
                        parent: node.index,
                        cost: new_cost,
                        ground_distance: neighboor_ground_distance,
                        wallrun_distance,
                        wallhop,
                        wallhop_offset,
                    });
                    index
                }
                Entry::Occupied(mut e) => {
                    if e.get().cost > new_cost {
                        // We've found a better path to this neighbor
                        e.insert(Visits {
                            parent: node.index,
                            cost: new_cost,
                            ground_distance: neighboor_ground_distance,
                            wallrun_distance,
                            wallhop,
                            wallhop_offset,
                        });
                        e.index()
                    } else {
                        // The existing path is better, do nothing
                        continue;
                    }
                }
            };

            // Only add to the queue if we've found a better path
            open_list.push(Reverse(Node {
                index: neighbor_index,
                cost: new_cost,
                estimated_cost: new_cost + estimated_cost,
            }));
        }
    }

    match end {
        Goal::ClosestToPoint(_) => {
            let end_index = visited_list
                .keys()
                .filter_map(|point| Some((end.distance(point), visited_list.get_index_of(point)?)))
                .reduce(|closer, other| if closer.0 < other.0 { closer } else { other })
                .unwrap_or((0., usize::MAX))
                .1;
            get_path(visited_list, end_index, start_index, octtree, cell_size)
        }
        Goal::Distance(_) => {
            let end_index = visited_list
                .keys()
                .flat_map(|point| {
                    pass_ground_distance(find_ground_distance(octtree, *point))
                        .then_some(())
                        .and_then(|_| visited_list.get_index_of(point))
                })
                .map(|index| (path_length(&visited_list, index, start_index), index))
                .reduce(|closer, other| if closer.0 < other.0 { closer } else { other })
                .unwrap_or((0, usize::MAX))
                .1;
            get_path(visited_list, end_index, start_index, octtree, cell_size)
        }
        Goal::Area(_, distance) => {
            let end_index = visited_list
                .keys()
                .filter_map(|point| Some((end.distance(point), visited_list.get_index_of(point)?)))
                .fold(None::<(f64, usize)>, |closer, other| {
                    if let Some(closer) = closer
                        && closer.0 < other.0
                    {
                        Some(closer)
                    } else if other.0 < distance {
                        Some(other)
                    } else {
                        None
                    }
                })
                .unwrap_or((0., usize::MAX))
                .1;
            get_path(visited_list, end_index, start_index, octtree, cell_size)
        }

        Goal::Point(_) => None,
    }
}

fn heuristic(neighbor: TUVec3u32, start: TUVec3u32, end: Goal) -> Cost {
    (end.distance(&neighbor) / end.distance(&start)) * 2.0
}

pub fn get_neighbors_h<'b>(
    point: TUVec3u32,
    octtree: &'b Octree<u32, TUVec3u32>,
) -> impl Iterator<Item = (TUVec3u32, bool)> + 'b {
    const ITEMS: &[[i32; 3]] = &[[1, 0, 0], [0, 1, 0], [-1, 0, 0], [0, -1, 0]];
    ITEMS
        .iter()
        .filter_map(move |offset| {
            Some(TUVec3u32::new(
                point.0.x.checked_add_signed(offset[0])?,
                point.0.y.checked_add_signed(offset[1])?,
                point.0.z.checked_add_signed(offset[2])?,
            ))
        })
        .map(move |neighboor| (neighboor, octtree.get(&neighboor.0).is_none()))
}

fn get_neighbors<'a>(
    octtree: &'a Octree<u32, TUVec3u32>,
    visited_list: &VisitedList,
    point: &'a TUVec3u32,
    ground_distance: u32,
    wallhop_offset: u32,
) -> Vec<(TUVec3u32, Cost)> {
    [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
        [-1, 0, 0],
        [0, -1, 0],
        [0, 0, -1],
    ]
    .into_iter()
    .filter_map(|offset| {
        Some((
            TUVec3u32::new(
                point.0.x.checked_add_signed(offset[0])?,
                point.0.y.checked_add_signed(offset[1])?,
                point.0.z.checked_add_signed(offset[2])?,
            ),
            visited_list
                .get(point)
                .map(
                    |&Visits {
                         parent: _,
                         cost: _,
                         ground_distance: distance,
                         wallrun_distance,
                         wallhop: _,
                         wallhop_offset: _,
                     }| (distance, wallrun_distance),
                )
                .unwrap_or_else(|| (find_ground_distance(octtree, *point), 0)),
        ))
    })
    .filter(
        |(neighboor_point, (neighboor_ground_distance, wallrun_distance))| {
            octtree.get(&neighboor_point.0).is_none()
                && (pass_ground_distance(*neighboor_ground_distance)
                    || fall_condition(
                        neighboor_point,
                        *neighboor_ground_distance,
                        point,
                        ground_distance,
                    )
                    || (get_neighbors_h(*neighboor_point, octtree)
                        .filter(|&(_, is_empty)| is_empty)
                        .count()
                        < 4
                        && pass_hop_distance(wallhop_offset, wallhop_offset)
                        && neighboor_point.0.z == point.0.z)
                    || (find_wall_point(*neighboor_point, octtree).is_some()
                        && neighboor_point.0.z == point.0.z))
                && *wallrun_distance < WALLRUN_MAX_DISTANCE
        },
    )
    .map(|(point, (_ground_distance, _wallrun_distance))| {
        (
            // this is some cost function
            point, 1.,
        )
    })
    .collect()
}

fn get_path(
    visited_list: VisitedList,
    mut index: usize,
    start: usize,
    octtree: &Octree32,
    cell_size: f32,
) -> Option<Vec<NavPoint>> {
    let mut path = Vec::new();

    while index != start {
        if let Some((
            pos,
            &Visits {
                parent: parent_index,
                cost: _,
                ground_distance,
                wallrun_distance: _,
                wallhop: _,
                wallhop_offset: _,
            },
        )) = visited_list.get_index(index)
        {
            path.push(NavPoint::new(
                visited_list
                    .get_index(parent_index)
                    .map(|(parent, _)| {
                        parent.0.x == pos.0.x
                            && parent.0.y == pos.0.y
                            && parent.0.z.saturating_sub(0) == pos.0.z
                    })
                    .unwrap_or_default()
                    .not()
                    .then(|| {
                        (pos.0.z.saturating_sub(MAX_DISTANCE.saturating_sub(1))..=pos.0.z)
                            .rev()
                            .find(|z| {
                                octtree
                                    .get(&TUVec3::new(pos.0.x, pos.0.y, z.saturating_sub(1)))
                                    .is_some()
                            })
                            .map(|z| TUVec3u32::new(pos.0.x, pos.0.y, z))
                    })
                    .flatten()
                    .unwrap_or(*pos),
                ground_distance,
                cell_size,
            ));
            index = parent_index
        } else {
            return None;
        }
    }

    if path.is_empty() {
        return None;
    }

    path.reverse();
    Some(path)
}

fn path_length(visited_list: &VisitedList, mut index: usize, start: usize) -> usize {
    let mut length = 0;

    while index != start {
        if let Some((
            _,
            &Visits {
                parent: parent_index,
                cost: _,
                ground_distance: _,
                wallrun_distance: _,
                wallhop: _,
                wallhop_offset: _,
            },
        )) = visited_list.get_index(index)
        {
            length += 1;
            index = parent_index
        } else {
            return 0;
        }
    }

    length
}

pub fn find_wall_point(point: TUVec3u32, octtree: &Octree32) -> Option<TUVec3u32> {
    if octtree
        .get(&TUVec3::new(
            point.0.x,
            point.0.y,
            point.0.z.saturating_add(1),
        ))
        .is_some()
    {
        return None;
    }

    let wall_point = get_neighbors_h(point, octtree)
        .find_map(|(point, is_empty)| is_empty.not().then_some(point))?;

    if octtree
        .get(&TUVec3::new(
            wall_point.0.x,
            wall_point.0.y,
            wall_point.0.z.saturating_add(1),
        ))
        .is_none()
        || octtree
            .get(&TUVec3::new(
                wall_point.0.x,
                wall_point.0.y,
                wall_point.0.z.saturating_sub(1),
            ))
            .is_none()
    {
        return None;
    }
    get_neighbors_h(point, octtree)
        .filter(|(_, is_empty)| *is_empty)
        .flat_map(|(point, _)| get_neighbors_h(point, octtree).zip([point; 4]))
        .filter_map(|((next_wall_point, is_empty), next_point)| {
            is_empty.not().then_some((next_wall_point, next_point))
        })
        .find_map(|(next_wall_point, next_point)| {
            ((wall_point.0.x == next_wall_point.0.x
                && (wall_point.0.y as i32 - next_wall_point.0.y as i32).abs() == 1)
                || (wall_point.0.y == next_wall_point.0.y
                    && (wall_point.0.x as i32 - next_wall_point.0.x as i32).abs() == 1))
                .then_some(next_point)
        })
}

/// the consequence of this are unknown to me but I just want to give bots the ability to fall down from any heigth
/// the bots now like to jump up and down sometimes
fn fall_condition(
    pos: &TUVec3u32,
    distance: u32,
    parent_pos: &TUVec3u32,
    parent_distance: u32,
) -> bool {
    (pass_ground_distance(parent_distance) && !pass_ground_distance(distance))
        || (!pass_ground_distance(distance)
            && parent_pos.0.z > pos.0.z
            && [parent_pos.0.x, parent_pos.0.y] == [pos.0.x, pos.0.y])
}

const MAX_DISTANCE: u32 = 6;
const HOP_DISTANCE: u32 = 3;
fn find_ground_distance(octtree: &Octree<u32, TUVec3u32>, point: TUVec3u32) -> u32 {
    (point.0.z.saturating_sub(MAX_DISTANCE + HOP_DISTANCE + 1)..point.0.z)
        .rev()
        .position(|z| octtree.get(&TUVec3::new(point.0.x, point.0.y, z)).is_some())
        .unwrap_or(usize::MAX) as u32
}

fn pass_ground_distance(distance: u32) -> bool {
    distance < MAX_DISTANCE
}
fn pass_hop_distance(distance: u32, wallhop_offset: u32) -> bool {
    distance.saturating_sub(wallhop_offset) < HOP_DISTANCE
}
