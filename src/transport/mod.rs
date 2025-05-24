use crate::{
    link::{Endpoint, LinkIntf, TransportCap},
    protocol::whatami::WhatAmI,
    Config,
};
use thiserror::Error;

mod unicast;

pub enum Transport<L> {
    Unicast(unicast::Unicast<L>),
    Multicast,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Link Error")]
    LinkError(#[from] crate::link::LinkError),
    #[error("Encode Error")]
    EncodeError(#[from] crate::iobuf::DidntWrite),
    #[error("Decode Error")]
    DecodeError(#[from] crate::iobuf::DidntRead),
    #[error("More cookie been allocated")]
    MoreCookieAllocated,
    #[error("Unexpect Message")]
    UnexpectMsg,
    #[error("Unexpect open sn resolution")]
    OpenSnResolution,
}

fn new_client<L: LinkIntf, E: Endpoint<L = L>>(
    ep: E,
    cfg: &Config,
) -> Result<Transport<L>, TransportError> {
    #[cfg(feature = "defmt")]
    defmt::debug!("Opening link");

    let zl = crate::link::open(ep)?;
    match zl.cap.transport() {
        TransportCap::Unicast => {
            let mut unicast = unicast::Unicast::new(zl);
            let params = unicast.handshake(cfg.mode, cfg.id)?;
            unicast.update(&params)?;
            Ok(Transport::Unicast(unicast))
        }
        TransportCap::Multicast => {
            unimplemented!()
        }
        _ => {
            unimplemented!()
        }
    }
}

impl<L: LinkIntf> Transport<L> {
    pub fn new<E: Endpoint<L = L>>(ep: E, cfg: &Config) -> Result<Transport<L>, TransportError> {
        match cfg.mode {
            WhatAmI::Client => new_client(ep, cfg),
            _ => {
                unimplemented!()
            }
        }
    }
}
