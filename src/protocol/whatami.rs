#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WhatAmI {
    Router = 0b00,
    Peer = 0b01,
    #[default]
    Client = 0b10,
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

impl From<u8> for WhatAmI {
    #[inline]
    fn from(b: u8) -> Self {
        match b & 0b0000_0011 {
            0b00 => WhatAmI::Router,
            0b01 => WhatAmI::Peer,
            0b10 => WhatAmI::Client,
            _ => unreachable!(),
        }
    }
}
