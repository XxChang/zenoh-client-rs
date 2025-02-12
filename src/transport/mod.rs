use crate::link::{Endpoint, TransportCap};
use thiserror::Error;

mod unicast;

pub enum Transport {
    Unicast,
    Multicast,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Link Error")]
    LinkError(#[from] crate::link::LinkError),
}

pub fn new_client<E: Endpoint>(ep: E) -> Result<(), TransportError> {
    let zl = crate::link::open(ep)?;

    match zl.cap.transport() {
        TransportCap::Unicast => {

        },
        TransportCap::Multicast => {
            unimplemented!()
        },
        _ => {
            unimplemented!()
        },
    }
    
    
    Ok(())
}
