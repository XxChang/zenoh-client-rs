use cobs::{DecodeError, DestBufTooSmallError};
use embedded_hal::delay::DelayNs;
use thiserror::Error;

use crate::transport::TransportError;

pub mod serial;

#[derive(Debug, Error)]
pub enum LinkError {
    #[error("Invalid Frame")]
    InvalidFrame(#[from] DestBufTooSmallError),
    #[error("Decode Error")]
    DecodeError(#[from] DecodeError),
    #[error("Crc Error")]
    CrcError,
    #[error("Invalid Parameter")]
    InvalidParameter,
    #[error("Io Error")]
    IoError,
}

pub trait LinkIntf: Sized {
    fn open(&mut self) -> Result<(), LinkError>;

    fn send(&mut self, msg: &[u8]) -> Result<(), LinkError>;

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, LinkError>;
}

pub trait Endpoint: Sized {
    type L: LinkIntf;

    fn create_link_from_endpoint(ep: Self) -> Link<Self::L>;
}

pub struct Link<Intf> {
    intf: Intf,
    pub mtu: usize,
    pub cap: LinkCapabilities,
}

impl<RX, TX, Delay> Endpoint for serial::SerialIntf<RX, TX, Delay>
where
    RX: embedded_io::Read,
    TX: embedded_io::Write,
    Delay: DelayNs,
{
    type L = serial::SerialIntf<RX, TX, Delay>;

    fn create_link_from_endpoint(ep: Self) -> Link<Self::L> {
        Link {
            intf: ep,
            mtu: 1500,
            cap: LinkCapabilities::new(TransportCap::Unicast, TransportFlow::DATAGRAM, false),
        }
    }
}

impl<RX, TX, Delay> LinkIntf for serial::SerialIntf<RX, TX, Delay>
where
    RX: embedded_io::Read,
    TX: embedded_io::Write,
    Delay: DelayNs,
{
    fn open(&mut self) -> Result<(), LinkError> {
        self.connect()?;
        Ok(())
    }

    fn send(&mut self, msg: &[u8]) -> Result<(), LinkError> {
        self.send(msg)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, LinkError> {
        self.recv(buf)
    }
}

impl<I> Link<I>
where
    I: LinkIntf,
{
    pub fn open(&mut self) -> Result<(), LinkError> {
        self.intf.open()
    }

    pub fn send_msg(&mut self, msg: &[u8]) -> Result<(), TransportError> {
        match self.cap.flow() {
            TransportFlow::DATAGRAM => {}
            TransportFlow::STREAM => {
                unimplemented!()
            }
        }

        self.intf.send(msg)?;

        Ok(())
    }

    pub fn recv_msg(&mut self, data: &mut [u8]) -> Result<usize, TransportError> {
        let msg = match self.cap.flow() {
            TransportFlow::STREAM => {
                unimplemented!()
            }
            TransportFlow::DATAGRAM => {
                let size = self.intf.recv(data)?;
                size
            }
        };

        Ok(msg)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportCap {
    Unicast = 0,
    Multicast = 1,
    Raweth = 2,
}

impl From<u8> for TransportCap {
    fn from(b: u8) -> Self {
        match b {
            0 => TransportCap::Unicast,
            1 => TransportCap::Multicast,
            2 => TransportCap::Raweth,
            _ => unreachable!(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFlow {
    DATAGRAM = 0,
    STREAM = 1,
}

impl From<u8> for TransportFlow {
    fn from(b: u8) -> Self {
        match b {
            0 => TransportFlow::DATAGRAM,
            1 => TransportFlow::STREAM,
            _ => unreachable!(),
        }
    }
}

#[derive(Default)]
pub struct LinkCapabilities(u8);

impl LinkCapabilities {
    pub fn new(transport: TransportCap, flow: TransportFlow, reliable: bool) -> Self {
        let mut lc = LinkCapabilities(0);
        lc.set_transport(transport);
        lc.set_flow(flow);
        lc.set_reliable(reliable);
        lc
    }

    pub fn transport(&self) -> TransportCap {
        let b = (self.0 >> 6) & 0b11;
        TransportCap::from(b)
    }

    pub fn set_transport(&mut self, t: TransportCap) {
        self.0 = self.0 | ((t as u8) << 6);
    }

    pub fn flow(&self) -> TransportFlow {
        let b = (self.0 >> 5) & 0b1;
        TransportFlow::from(b)
    }

    pub fn set_flow(&mut self, f: TransportFlow) {
        self.0 = self.0 | ((f as u8) << 5);
    }

    pub fn reliable(&self) -> bool {
        (self.0 >> 4) & 0b1 == 1
    }

    pub fn set_reliable(&mut self, r: bool) {
        if r {
            self.0 = self.0 | (1 << 4);
        } else {
            self.0 = self.0 & !(1 << 4);
        }
    }
}

pub fn open<L: LinkIntf, E: Endpoint<L = L>>(ep: E) -> Result<Link<L>, LinkError> {
    let mut l = E::create_link_from_endpoint(ep);

    l.open()?;

    Ok(l)
}
