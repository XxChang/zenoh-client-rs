#![no_std]
#![no_main]

use link::{LinkIntf, TransportCap};
use protocol::{whatami::WhatAmI, ZenohID};

mod iobuf;
pub mod link;
pub mod protocol;
pub mod transport;

const Z_BATCH_UNICAST_SIZE: u16 = 2048;
const Z_MAX_MTU: usize = 2048;
const Z_PROTO_VERSION: u8 = 0x09;
const Z_SN_RESOLUTION: u8 = 0x02;
const Z_REQ_RESOLUTION: u8 = 0x02;

pub struct Config {
    pub id: ZenohID,
    pub mode: WhatAmI,
}

impl Config {
    pub fn new(id: ZenohID, mode: WhatAmI) -> Self {
        Config { id, mode }
    }
}

pub fn open(cfg: &Config) {
    match cfg.mode {
        WhatAmI::Client => {}
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
