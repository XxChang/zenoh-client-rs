use crate::protocol::whatami::WhatAmI;

pub enum Transport {
    Unicast,
    Multicast,
}

impl Transport {
    pub fn new(mode: &WhatAmI) {
        if *mode == WhatAmI::Client {
        } else {
            unimplemented!()
        }
    }
}
