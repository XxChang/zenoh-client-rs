//! NOTE: 16 bits (2 bytes) may be prepended to the serialized message indicating the total length
//!       in bytes of the message, resulting in the maximum length of a message being 65_535 bytes.
//!       This is necessary in those stream-oriented transports (e.g., TCP) that do not preserve
//!       the boundary of the serialized messages. The length is encoded as little-endian.
//!       In any case, the length of a message must not exceed 65_535 bytes.
//!
//! The OPEN message is sent on a link to finally open an initialized session with the peer.
//!
//! Flags:
//! - A Ack           if A==1 then the message is an acknowledgment (aka OpenAck), otherwise OpenSyn
//! - T Lease period  if T==1 then the lease period is in seconds else in milliseconds
//! - Z Extensions    if Z==1 then Zenoh extensions are present
//!
//!  7 6 5 4 3 2 1 0
//! +-+-+-+-+-+-+-+-+
//! |Z|T|A|   OPEN  |
//! +-+-+-+---------+
//! %     lease     % -- Lease period of the sender of the OPEN message
//! +---------------+
//! %  initial_sn   % -- Initial SN proposed by the sender of the OPEN(*)
//! +---------------+
//! ~    <u8;z16>   ~ if Flag(A)==0 (**) -- Cookie
//! +---------------+
//! ~   [OpenExts]  ~ if Flag(Z)==1
//! +---------------+
//!
//! (*)     The initial sequence number MUST be compatible with the sequence number resolution agreed in the
//!         [`super::InitSyn`]-[`super::InitAck`] message exchange
//! (**)    The cookie MUST be the same received in the [`super::InitAck`]from the corresponding zenoh node
//!

use crate::{
    iobuf::{Reader, Writer},
    protocol::{transport::TransportBody, Varint},
    transport::TransportError,
};

use super::TransportMessage;

pub(crate) const Z_MID_T_OPEN: u8 = 0x02;

pub mod flag {
    pub const A: u8 = 1 << 5; // 0x20 Ack           if A==0 then the message is an InitSyn else it is an InitAck
    pub const T: u8 = 1 << 6; // 0x40 Lease period  if T==1 then the lease period is in seconds else in milliseconds
    pub const Z: u8 = 1 << 7; // 0x80 Extensions    if Z==1 then an extension will follow
}

#[derive(Debug, PartialEq, Eq)]
pub struct OpenSyn<'a> {
    pub lease: u32,
    pub initial_sn: u32,
    pub cookie: Option<&'a [u8]>,
}

impl<'a> OpenSyn<'a> {
    pub fn new(lease: u32, initial_sn: u32, cookie: Option<&'a [u8]>) -> Self {
        Self {
            lease,
            initial_sn,
            cookie,
        }
    }

    pub fn header(&self) -> u8 {
        let mut header = Z_MID_T_OPEN;

        if (self.lease % 1000) == 0 {
            header |= flag::T;
        }

        header
    }

    pub fn encode<W: Writer>(&self, writer: &mut W) -> Result<(), TransportError> {
        #[cfg(feature = "defmt")]
        defmt::debug!("Encoding _Z_MID_T_OPEN");

        let header = self.header();

        writer.write_u8(header)?;

        if header & flag::T == flag::T {
            Varint::<u64>::encode(writer, self.lease as u64 / 1000)?;
        } else {
            Varint::<u64>::encode(writer, self.lease as u64)?;
        }

        Varint::<u64>::encode(writer, self.initial_sn as u64)?;

        if header & flag::A == 0 {
            if let Some(cookie) = self.cookie {
                Varint::<u64>::encode(writer, cookie.len() as u64)?;
                writer.write(cookie)?;
            }
        }

        Ok(())
    }

    pub fn decode<R: Reader>(
        reader: &mut R,
        header: u8,
    ) -> Result<TransportMessage, TransportError> {
        #[cfg(feature = "defmt")]
        defmt::debug!("Decoding _Z_MID_T_OPEN");

        let lease = Varint::<u32>::decode(reader)?;
        let lease = if header & flag::T == flag::T {
            lease * 1000
        } else {
            lease
        };

        let initial_sn = Varint::<u32>::decode(reader)?;
        let cookie = if header & flag::A == flag::A {
            None
        } else {
            let cookie_len = Varint::<u64>::decode(reader)? as usize;
            let cookie = reader.read_slice_in_place(cookie_len)?;
            Some(cookie)
        };

        if header & flag::Z == flag::Z {
            unimplemented!()
        }

        if header & flag::A == flag::A {
            Ok(TransportMessage {
                body: TransportBody::OpenAck(OpenSyn {
                    lease,
                    initial_sn,
                    cookie,
                }),
            })
        } else {
            Ok(TransportMessage {
                body: TransportBody::OpenSyn(OpenSyn {
                    lease,
                    initial_sn,
                    cookie,
                }),
            })
        }
    }
}
