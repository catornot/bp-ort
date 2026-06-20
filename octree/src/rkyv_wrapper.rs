use bevy_math::UVec3;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
#[rkyv(remote = UVec3)]
#[rkyv(compare(PartialEq), derive(Debug))]
#[repr(C)]
pub struct UVec3Def {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl From<UVec3Def> for UVec3 {
    fn from(value: UVec3Def) -> Self {
        UVec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}
