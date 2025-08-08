use indexmap::{map::Entry, IndexMap};
use oktree::prelude::*;
use rustc_hash::FxHasher;
use std::{cmp::Reverse, collections::BinaryHeap, hash::BuildHasherDefault};

use crate::loader::Octree32;

type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;

type Cost = f64;

#[derive(Debug, Clone, Copy)]
pub struct Node {
    index: usize,
    cost: Cost,
    estimated_cost: Cost,
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
        self.cost.total_cmp(&other.cost)
    }
}
impl Eq for Node {}

pub fn find_path(octtree: &Octree32, start: TUVec3u32, end: TUVec3u32) -> Option<Vec<TUVec3u32>> {
    // posistions to be evaluated
    let mut open_list = BinaryHeap::<Reverse<Node>>::new();

    let mut visited_list = FxIndexMap::<TUVec3u32, (usize, Cost)>::default();

    let start_index = visited_list.insert_full(start, (usize::MAX, 0.)).0;
    open_list.push(Reverse(Node {
        index: start_index,
        cost: 0.,
        estimated_cost: 0.,
    }));

    let mut iterations = 0usize;
    while let Some(Reverse(node)) = open_list.pop()
        && iterations < u16::MAX as usize * 4
    {
        iterations += 1;
        let (&node_pos, &(_, cost)) = visited_list.get_index(node.index).unwrap();

        // If cost of new node from BinaryHeap is higher than the best cost, skip it
        // This implies we've already found a better path to this node
        if node.cost > cost {
            continue;
        }

        // Check if we've reached the goal
        if end == node_pos {
            log::info!("yay");
            return get_path(visited_list, node.index, start_index);
        }

        for (neighbor, edge_cost) in get_neighbors(octtree, &node_pos) {
            // new cost to reach this node = edge cost + node cost
            // This is confirmed cost, not heuristic
            let new_cost = edge_cost + cost;

            // calculate heuristic cost
            let estimated_cost = heuristic(neighbor, end, start);

            let neighbor_index = match visited_list.entry(neighbor) {
                Entry::Vacant(entry) => {
                    // This is the first time we're seeing this neighbor
                    let index = entry.index();
                    entry.insert((node.index, new_cost));
                    index
                }
                Entry::Occupied(mut e) => {
                    if e.get().1 > new_cost {
                        // We've found a better path to this neighbor
                        e.insert((node.index, new_cost));
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

    None
}

fn heuristic(neighbor: TUVec3u32, end: TUVec3u32, start: TUVec3u32) -> Cost {
    fn distance3(pos: TUVec3u32, target: TUVec3u32) -> f64 {
        (((pos.0.x - target.0.x).pow(2)
            + (pos.0.y - target.0.y).pow(2)
            + (pos.0.z - target.0.z).pow(2)) as f64)
            .sqrt()
    }

    distance3(neighbor, end) / distance3(start, end)
}

fn get_neighbors<'a>(
    octtree: &'a Octree<u32, TUVec3u32>,
    point: &'a TUVec3u32,
) -> impl Iterator<Item = (TUVec3u32, Cost)> + 'a {
    [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
        [-1, 0, 0],
        [0, -1, 0],
        [0, 0, -1],
    ]
    .into_iter()
    .map(|offset| {
        TUVec3u32::new(
            point.0.x.saturating_add_signed(offset[0]),
            point.0.y.saturating_add_signed(offset[1]),
            point.0.z.saturating_add_signed(offset[2]),
        )
    })
    .filter(|point| octtree.get(&point.0).is_none())
    .map(|point| (point, 1.))
}

fn get_path(
    visited_list: FxIndexMap<TUVec3u32, (usize, f64)>,
    mut index: usize,
    start: usize,
) -> Option<Vec<TUVec3u32>> {
    let mut path = Vec::new();

    while index != start {
        if let Some((pos, &(parent_index, _))) = visited_list.get_index(index) {
            path.push(*pos);
            index = parent_index
        } else {
            log::info!("skill issue");
            return None;
        }
    }

    if path.is_empty() {
        return None;
    }

    path.reverse();
    Some(path)
}
