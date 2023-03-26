use super::bindings::CBasePlayerUnion;

pub type CbasePlayerPtr = *const CBasePlayerUnion;

pub struct CbasePlayer {
    inner: &'static CBasePlayerUnion,
}

impl TryFrom<CbasePlayerPtr> for CbasePlayer {
    type Error = ();

    fn try_from(value: CbasePlayerPtr) -> Result<Self, Self::Error> {
        unsafe {
            Ok(Self {
                inner: value.as_ref().ok_or(())?,
            })
        }
    }
}

impl From<CbasePlayer> for CbasePlayerPtr {
    fn from(val: CbasePlayer) -> Self {
        val.inner as CbasePlayerPtr
    }
}

#[allow(dead_code)]
impl CbasePlayer {
    pub fn get_index(&self) -> u32 {
        unsafe { self.inner.player_index.player_index }
    }

    pub fn get_team(&self) -> i32 {
        unsafe { self.inner.team.team }
    }

    pub fn set_clan_tag(&self, new_tag: String) {
        let mut tag = unsafe { self.inner.community_clan_tag.community_clan_tag };

        for (index, c) in new_tag.chars().enumerate() {
            tag[index] = c as u8 as i8
        }
    }
}
