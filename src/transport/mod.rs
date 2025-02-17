use crate::{
    link::{Endpoint, LinkIntf, TransportCap},
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

pub fn new_client<L: LinkIntf, E: Endpoint<L = L>>(
    ep: E,
    cfg: &Config,
) -> Result<(), TransportError> {
    #[cfg(feature = "defmt")]
    defmt::debug!("Opening link");

    let zl = crate::link::open(ep)?;
    match zl.cap.transport() {
        TransportCap::Unicast => {
            let mut unicast = unicast::Unicast::new(zl);
            unicast.handshake(cfg.mode, cfg.id)?;
        }
        TransportCap::Multicast => {
            unimplemented!()
        }
        _ => {
            unimplemented!()
        }
    }

    Ok(())
}

pub trait RandomGenerator {
    fn get_random<T>(self) -> T;
}
