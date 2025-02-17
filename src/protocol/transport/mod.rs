// use heapless::{arc_pool, box_pool, pool::boxed::Box};
use init::{InitSyn, Z_MID_T_INIT};
use open::{OpenSyn, Z_MID_T_OPEN};
// use once_cell::unsync::Lazy;

use crate::{
    iobuf::{Reader, Writer},
    transport::TransportError,
};

pub mod init;
pub mod open;

const Z_DEFAULT_MULTICAST_BATCH_SIZE: u16 = 8192;
const Z_DEFAULT_RESOLUTION_SIZE: u8 = 2;

// Zenoh messages at zenoh-transport level
#[derive(Debug, PartialEq, Eq)]
pub enum TransportBody<'c> {
    Join,
    InitSyn(InitSyn<'c>),
    InitAck(InitSyn<'c>),
    OpenSyn(OpenSyn<'c>),
    OpenAck(OpenSyn<'c>),
    Close,
    KeepAlive,
    Frame,
    Fragment,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TransportMessage<'c> {
    pub body: TransportBody<'c>,
}

impl<'c> TransportMessage<'c> {
    pub fn encode<W: Writer>(&self, writer: &mut W) -> Result<(), TransportError> {
        match &self.body {
            TransportBody::InitSyn(b) => {
                b.encode(writer)?;
            }
            TransportBody::OpenSyn(b) => {
                b.encode(writer)?;
            }
            _ => todo!(),
        }

        Ok(())
    }

    pub fn decode<'r: 'c, R: Reader>(reader: &'r mut R) -> Result<Self, TransportError> {
        let header = reader.read_u8()?;

        match header & 0x1f {
            Z_MID_T_INIT => init::InitSyn::decode(reader, header),
            Z_MID_T_OPEN => open::OpenSyn::decode(reader, header),
            _ => {
                unimplemented!("Unknown message type: {}", header);
            }
        }
    }
}

// Global only cookie
// #[derive(PartialEq, Eq)]
// pub struct Cookie {
//     cookie: [u8; 1024],
//     len: usize,
// }

// #[cfg(feature = "defmt")]
// impl defmt::Format for Cookie {
//     fn format(&self, fmt: defmt::Formatter) {
//         defmt::write!(fmt, "{=[u8]:?}", &self.cookie[..self.len]);
//     }
// }

// impl Cookie {
//     pub fn as_slice(&self) -> &[u8] {
//         &self.cookie[..self.len]
//     }

// fn from_slice(slice: &[u8]) -> Self {
//     let mut cookie = [0u8; 1024];
//     cookie[..slice.len()].copy_from_slice(slice);
//     Cookie {
//         cookie,
//         len: slice.len(),
//     }
// }
// }

// impl core::fmt::Debug for Cookie {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         f.debug_list()
//             .entries(self.cookie[..self.len].iter())
//             .finish()
//     }
// }

// impl core::fmt::Debug for Box<Cookie> {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {

//     }
// }
// box_pool!(CookieStorage: Cookie);

// static CookieStorage:

// impl CookieBlock {

// }
