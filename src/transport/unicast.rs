use crate::link::{Link, LinkIntf};
use crate::protocol::transport::init::InitSyn;
use crate::protocol::transport::{TransportBody, TransportMessage};
use crate::protocol::{whatami::WhatAmI, ZenohID};

use super::TransportError;

pub struct Unicast<L> {
    intf: Link<L>,
}

impl<L: LinkIntf> Unicast<L> {
    pub fn new(intf: Link<L>) -> Self {
        Unicast { intf }
    }

    pub fn handshake(&mut self, whatami: WhatAmI, zid: ZenohID) -> Result<(), TransportError> {
        let ism = InitSyn::new(whatami, zid);

        #[cfg(feature = "defmt")]
        defmt::debug!("Sending Z_INIT(Syn)");

        self.intf.send_msg(&ism)?;

        let iam = self.intf.recv_msg()?;
        if let TransportMessage {
            body: TransportBody::InitAck(ism),
        } = iam
        {
            #[cfg(feature = "defmt")]
            defmt::debug!("Received Z_INIT(Ack)");
        } else {
        }
        // #[cfg(feature = "defmt")]

        // if let Ok(TransportMessage { b }) = self.intf.recv_msg()
        Ok(())
    }
}
