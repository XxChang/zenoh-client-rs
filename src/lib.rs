#![no_std]
#![no_main]

use link::{LinkIntf, TransportCap};
use protocol::{whatami::WhatAmI, ZenohID};

pub mod link;
pub mod protocol;
pub mod transport;

pub struct Config {
    id: ZenohID,
    mode: WhatAmI,
}

impl Config {
    pub fn new(id: ZenohID, mode: WhatAmI) -> Self {
        Config {
            id,
            mode,
        }
    }
}

pub fn open(cfg: &Config) {
    match cfg.mode {
        WhatAmI::Client => {
            
        },
        _ => {
            unimplemented!()
        }
    }
}

// pub fn open<L: LinkIntf<I=L, Endpoint = L>>(cfg: &Config, ep: L) -> Result<(), link::LinkError> {
//     let mut l = L::new(ep);

//     l.open()?;
    
//     match l.cap.transport() {
//         TransportCap::Unicast => {

//         },
//         TransportCap::Multicast => {
//             unimplemented!()
//         },
//         TransportCap::Raweth => {
//             unimplemented!()
//         },
//     }

//     Ok(())
// }
