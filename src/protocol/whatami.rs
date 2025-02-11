#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhatAmI {
    Router = 0b001,
    Peer = 0b010,
    #[default]
    Client = 0b100,
}

impl WhatAmI {
    const STR_R: &'static str = "router";
    const STR_P: &'static str = "peer";
    const STR_C: &'static str = "client";

    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Router => Self::STR_R,
            Self::Peer => Self::STR_P,
            Self::Client => Self::STR_C,
        }
    }
}
