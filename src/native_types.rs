use std::mem;

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignonState {
    Unknown = -1, // custom
    #[default]
    None = 0, // no state yet; about to connect
    Challenge = 1, // client challenging server; all OOB packets
    Connected = 2, // client is connected to server; netchans ready
    New = 3,      // just got serverinfo and string tables
    Prespawn = 4, // received signon buffers
    Gettingdata = 5, // respawn-defined signonstate, assumedly this is for persistence
    Spawn = 6,    // ready to receive entity packets
    Firstsnap = 7, // another respawn-defined one
    Full = 8,     // we are fully connected; first non-delta packet received
    Changelevel = 9, // server is changing level; please wait
}

impl From<i32> for SignonState {
    fn from(value: i32) -> Self {
        if value < Self::None as i32 || value > Self::Changelevel as i32 {
            return Self::Unknown;
        }

        unsafe { mem::transmute(value) }
    }
}
