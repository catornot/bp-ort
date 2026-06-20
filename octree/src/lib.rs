//! an octree made to be generated once and that's it

use bevy_math::{UVec3, Vec3};
use rkyv::{Archive, Deserialize, Serialize};
use std::ops::Not;

mod rkyv_wrapper;

pub const SPACE_DIVISION: u32 = u32::MAX / 2;

#[derive(Archive, Serialize, Deserialize, Debug)]
#[rkyv(derive(Debug))]
pub struct Octree {
    nodes: Vec<Node>,
    #[rkyv(with = rkyv_wrapper::UVec3Def)]
    min: UVec3,
    #[rkyv(with = rkyv_wrapper::UVec3Def)]
    max: UVec3,
    first: Node,
    cell_size: f32,
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Node {
    depth: u32,
    inner: NodeInner,
}

/// node index of [[u32::MAX]] is invalid
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
enum NodeInner {
    Children([u32; 8]),
    Leafs(u8),
    Leaf(bool),
}

pub trait ToPoint {
    fn into_point(self, cell_size: f32) -> UVec3;

    fn from_point(point: UVec3, cell_size: f32) -> Self;
}

impl Octree {
    pub fn new<T>(points: impl Iterator<Item = T>, cell_size: f32) -> Self
    where
        T: ToPoint,
    {
        let points: Vec<_> = points
            .map(move |value| value.into_point(cell_size))
            .collect();

        let max = points
            .iter()
            .copied()
            .fold(UVec3::ZERO, |max, point| max.max(point));
        let min = points
            .iter()
            .copied()
            .fold(UVec3::ZERO, |max, point| max.min(point));

        if points.is_empty() {
            return Self {
                nodes: Vec::new(),
                min,
                max,
                first: Node {
                    depth: 0,
                    inner: NodeInner::Leaf(false),
                },
                cell_size,
            };
        }

        let mut nodes = Vec::with_capacity(points.len());
        Self {
            first: Node::build(min, max, 0, points.to_vec(), &mut nodes),
            nodes,
            min,
            max,
            cell_size,
        }
    }

    pub fn get<T>(&self, point: T) -> bool
    where
        T: ToPoint,
    {
        self.first.find(
            point.into_point(self.cell_size),
            self.min,
            self.max,
            &self.nodes,
        )
    }
}

impl Node {
    fn build(
        min: UVec3,
        max: UVec3,
        depth: u32,
        points: Vec<UVec3>,
        nodes_pool: &mut Vec<Self>,
    ) -> Self {
        let inner = match points.len() {
            0 => NodeInner::Leaf(false),
            1 => NodeInner::Leaf(true),
            0..=8
                if add_up_bounds(min, max, &points)
                    .iter()
                    .all(|count| *count <= 1) =>
            {
                NodeInner::Leafs(
                    add_up_bounds(min, max, &points)
                        .iter()
                        .copied()
                        .enumerate()
                        .fold(0u8, |acc, (index, count)| {
                            acc | ((count == 1) as u32 as u8) << index
                        }),
                )
            }
            _ => NodeInner::Children(build_child_bounds(min, max).map(|(min, max)| {
                let node = Node::build(
                    min,
                    max,
                    depth + 1,
                    points
                        .iter()
                        .copied()
                        .filter(|point| is_in_bounds(min, max, *point))
                        .collect(),
                    nodes_pool,
                );
                let index = nodes_pool.len();
                nodes_pool.push(node);

                index as u32
            })),
        };

        Self { depth, inner }
    }

    fn find(&self, position: UVec3, min: UVec3, max: UVec3, nodes_pool: &[Self]) -> bool {
        match self.inner {
            NodeInner::Children(children_indices) => {
                let center = UVec3::new(
                    (max.x + min.x) / 2,
                    (max.y + min.y) / 2,
                    (max.z + min.z) / 2,
                );
                let x = (position.x < center.x).not() as usize;
                let y = (position.y < center.y).not() as usize;
                let z = (position.z < center.z).not() as usize;

                let index = x | y << 1 | z << 2;
                // TODO: couldn't maybe optimize this somehow
                let (min, max) = build_child_bounds(min, max)[index];
                nodes_pool[children_indices[index] as usize].find(position, min, max, nodes_pool)
            }
            NodeInner::Leafs(leafs) => {
                let center = UVec3::new(
                    (max.x + min.x) / 2,
                    (max.y + min.y) / 2,
                    (max.z + min.z) / 2,
                );
                let x = (position.x < center.x).not() as u32;
                let y = (position.y < center.y).not() as u32;
                let z = (position.z < center.z).not() as u32;

                let index = x | y << 1 | z << 2;
                (leafs >> index) & 1 == 1
            }
            NodeInner::Leaf(exists) => exists,
        }
    }
}

fn build_child_bounds(min: UVec3, max: UVec3) -> [(UVec3, UVec3); 8] {
    let center = UVec3::new(
        (max.x + min.x) / 2,
        (max.y + min.y) / 2,
        (max.z + min.z) / 2,
    );

    [
        (min, center),
        (
            UVec3::new(center.x, min.y, min.z),
            UVec3::new(max.x, center.y, center.z),
        ),
        (
            UVec3::new(center.x, min.y, center.z),
            UVec3::new(max.x, center.y, max.z),
        ),
        (
            UVec3::new(min.x, min.y, center.z),
            UVec3::new(center.x, center.y, max.z),
        ),
        (
            UVec3::new(min.x, center.y, min.z),
            UVec3::new(center.x, max.y, center.z),
        ),
        (
            UVec3::new(center.x, center.y, min.z),
            UVec3::new(max.x, max.y, center.z),
        ),
        (center, max),
        (
            UVec3::new(min.x, center.y, center.z),
            UVec3::new(center.x, max.y, max.z),
        ),
    ]
}

fn add_up_bounds(min: UVec3, max: UVec3, points: &[UVec3]) -> [u32; 8] {
    build_child_bounds(min, max).map(|(min, max)| {
        points
            .iter()
            .copied()
            .filter(|point| is_in_bounds(min, max, *point))
            .count() as u32
    })
}

fn is_in_bounds(min: UVec3, max: UVec3, point: UVec3) -> bool {
    min.x <= point.x
        && min.y <= point.y
        && min.z <= point.z
        && max.x >= point.x
        && max.y >= point.y
        && max.z >= point.z
}

impl ToPoint for UVec3 {
    fn into_point(self, _: f32) -> UVec3 {
        self
    }

    fn from_point(point: UVec3, _: f32) -> Self {
        point
    }
}

impl ToPoint for Vec3 {
    fn into_point(self, cell_size: f32) -> UVec3 {
        self.to_array()
            .map(|magnitude| ((magnitude / cell_size) + SPACE_DIVISION as f32) as u32)
            .into()
    }

    fn from_point(point: UVec3, cell_size: f32) -> Self {
        point
            .to_array()
            .map(|magnitude| (magnitude as f32 * cell_size) - SPACE_DIVISION as f32)
            .into()
    }
}
