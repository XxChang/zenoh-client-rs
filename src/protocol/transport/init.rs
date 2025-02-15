//! # Init message
//!
//! NOTE: 16 bits (2 bytes) may be prepended to the serialized message indicating the total length
//!       in bytes of the message, resulting in the maximum length of a message being 65_535 bytes.
//!       This is necessary in those stream-oriented transports (e.g., TCP) that do not preserve
//!       the boundary of the serialized messages. The length is encoded as little-endian.
//!       In any case, the length of a message must not exceed 65_535 bytes.
//!
//! The INIT message is sent on a specific Locator to initiate a session with the peer associated
//! with that Locator. The initiator MUST send an INIT message with the A flag set to 0.  If the
//! corresponding peer deems appropriate to initialize a session with the initiator, the corresponding
//! peer MUST reply with an INIT message with the A flag set to 1.
//!
//! Flags:
//! - A: Ack          if A==0 then the message is an InitSyn else it is an InitAck
//! - S: Size params  if S==1 then size parameters are exchanged
//! - Z: Extensions   if Z==1 then zenoh extensions will follow.
//!
//!  7 6 5 4 3 2 1 0
//! +-+-+-+-+-+-+-+-+
//! |Z|S|A|   INIT  |
//! +-+-+-+---------+
//! |    version    |
//! +---------------+
//! |zid_len|x|x|wai| (#)(*)
//! +-------+-+-+---+
//! ~      [u8]     ~ -- ZenohID of the sender of the INIT message
//! +---------------+
//! |x|x|kid|rid|fsn| \                -- SN/ID resolution (+)
//! +---------------+  | if Flag(S)==1
//! |      u16      |  |               -- Batch Size ($)
//! |               | /
//! +---------------+
//! ~    <u8;z16>   ~ -- if Flag(A)==1 -- Cookie
//! +---------------+
//! ~   [InitExts]  ~ -- if Flag(Z)==1
//! +---------------+
//!
//! If A==1 and S==0 then size parameters are (ie. S flag) are accepted.
//!
//! (*) WhatAmI. It indicates the role of the zenoh node sending the INIT
//! message.
//!    The valid WhatAmI values are:
//!    - 0b00: Router
//!    - 0b01: Peer
//!    - 0b10: Client
//!    - 0b11: Reserved
//!
//! (#) ZID length. It indicates how many bytes are used for the ZenohID bytes.
//!     A ZenohID is minimum 1 byte and maximum 16 bytes. Therefore, the actual
//!     length is computed as:
//!         real_zid_len := 1 + zid_len
//!
//! (+) Sequence Number/ID resolution. It indicates the resolution and
//! consequently the wire overhead
//!     of various SN and ID in Zenoh.
//!     - fsn: frame/fragment sequence number resolution. Used in Frame/Fragment
//!     messages.
//!     - rid: request ID resolution. Used in Request/Response messages.
//!     - kid: key expression ID resolution. Used in Push/Request/Response
//!     messages. The valid SN/ID resolution values are:
//!     - 0b00: 8 bits
//!     - 0b01: 16 bits
//!     - 0b10: 32 bits
//!     - 0b11: 64 bits
//!
//! ($) Batch Size. It indicates the maximum size of a batch the sender of the
//!
#![allow(static_mut_refs)]

use crate::{
    iobuf::{Reader, Writer},
    protocol::{whatami::WhatAmI, Varint, ZenohID},
    transport::TransportError,
    Z_BATCH_UNICAST_SIZE, Z_PROTO_VERSION, Z_REQ_RESOLUTION, Z_SN_RESOLUTION,
};

use super::{
    TransportBody, TransportMessage, Z_DEFAULT_MULTICAST_BATCH_SIZE, Z_DEFAULT_RESOLUTION_SIZE,
};
use heapless::{
    box_pool,
    pool::boxed::{Box, BoxBlock},
};

// Global only cookie
#[derive(PartialEq, Eq)]
pub struct Cookie {
    cookie: [u8; 1024],
    len: usize,
}

#[cfg(feature = "defmt")]
impl defmt::Format for Cookie {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{=[u8]:?}", &self.cookie[..self.len]);
    }
}

impl Cookie {
    pub fn as_slice(&self) -> &[u8] {
        &self.cookie[..self.len]
    }

    fn from_slice(slice: &[u8]) -> Self {
        let mut cookie = [0u8; 1024];
        cookie[..slice.len()].copy_from_slice(slice);
        Cookie {
            cookie,
            len: slice.len(),
        }
    }
}

impl core::fmt::Debug for Cookie {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(self.cookie[..self.len].iter())
            .finish()
    }
}

box_pool!(P: Cookie);

pub(crate) const Z_MID_T_INIT: u8 = 0x01;

pub mod flag {
    pub const A: u8 = 1 << 5; // 0x20 Ack           if A==0 then the message is an InitSyn else it is an InitAck
    pub const S: u8 = 1 << 6; // 0x40 Size params   if S==1 then size parameters are exchanged
    pub const Z: u8 = 1 << 7; // 0x80 Extensions    if Z==1 then an extension will follow
}

#[derive(Debug, PartialEq, Eq)]
pub struct InitSyn {
    zid: ZenohID,
    cookie: Option<Box<P>>,
    batch_size: u16,
    whatami: WhatAmI,
    req_id_res: u8,
    seq_num_res: u8,
    version: u8,
}

impl InitSyn {
    pub fn new(whatami: WhatAmI, zid: ZenohID) -> TransportMessage {
        let block: &'static mut BoxBlock<Cookie> = unsafe {
            static mut B: BoxBlock<Cookie> = BoxBlock::new();
            &mut B
        };

        P.manage(block);

        TransportMessage {
            body: TransportBody::InitSyn(InitSyn {
                version: Z_PROTO_VERSION,
                whatami,
                zid,
                cookie: None,
                req_id_res: Z_REQ_RESOLUTION,
                seq_num_res: Z_SN_RESOLUTION,
                batch_size: Z_BATCH_UNICAST_SIZE,
            }),
        }
    }

    pub fn header(&self) -> u8 {
        let mut header = Z_MID_T_INIT;

        if self.batch_size != Z_DEFAULT_MULTICAST_BATCH_SIZE
            || self.seq_num_res != Z_DEFAULT_RESOLUTION_SIZE
            || self.req_id_res != Z_DEFAULT_RESOLUTION_SIZE
        {
            header |= flag::S;
        }

        header
    }

    pub fn encode<W: Writer>(&self, writer: &mut W) -> Result<(), TransportError> {
        #[cfg(feature = "defmt")]
        defmt::debug!("Encoding _Z_MID_T_INIT");

        let header = self.header();

        writer.write_u8(header)?;

        writer.write_u8(self.version)?;

        let whatami = match &self.whatami {
            WhatAmI::Router => 0b00,
            WhatAmI::Peer => 0b01,
            WhatAmI::Client => 0b10,
        };
        let flags = ((self.zid.size() as u8 - 1) << 4) | whatami;
        writer.write_u8(flags)?;

        let zid = self.zid.to_le_bytes();
        writer.write_exact(&zid[..self.zid.size()])?;

        if header & flag::S == flag::S {
            let mut cbyte = 0u8;
            cbyte |= self.seq_num_res & 0x03;
            cbyte |= (self.req_id_res & 0x03) << 2;
            writer.write_u8(cbyte)?;
            writer.write_exact(&self.batch_size.to_le_bytes())?;
        }

        if header & flag::A == flag::A {
            if let Some(cookie) = &self.cookie {
                writer.write_exact(cookie.as_slice())?;
            }
        }

        Ok(())
    }

    pub fn decode<R: Reader>(
        reader: &mut R,
        header: u8,
    ) -> Result<TransportMessage, TransportError> {
        #[cfg(feature = "defmt")]
        defmt::debug!("Decoding _Z_MID_T_INIT");

        let version = reader.read_u8()?;

        let cbyte = reader.read_u8()?;

        let whatami = WhatAmI::from(cbyte);
        let zid_len = (((cbyte & 0xF0) >> 4) + 1) as usize;

        let mut zid_bytes = [0u8; 16];
        reader.read_exact(&mut zid_bytes[0..zid_len])?;

        let zid = u128::from_le_bytes(zid_bytes);
        let zid = ZenohID::from(zid);

        let (seq_num_res, req_id_res, batch_size) = if header & flag::S == flag::S {
            let cbyte = reader.read_u8()?;
            let seq_num_res = cbyte & 0x03;
            let req_id_res = (cbyte & 0x0C) >> 2;
            let mut batch_size_bytes = [0u8; 2];
            reader.read_exact(&mut batch_size_bytes)?;
            let batch_size = u16::from_le_bytes(batch_size_bytes);

            (seq_num_res, req_id_res, batch_size)
        } else {
            (
                Z_DEFAULT_RESOLUTION_SIZE,
                Z_DEFAULT_RESOLUTION_SIZE,
                Z_DEFAULT_MULTICAST_BATCH_SIZE,
            )
        };

        let cookie = if header & flag::A == flag::A {
            let cookie_len = Varint::<u64>::decode(reader)? as usize;

            let cookie = reader.read_slice_in_place(cookie_len)?;

            let cookie = P
                .alloc(Cookie::from_slice(cookie))
                .map_err(|_| TransportError::MoreCookieAllocated)?;

            // #[cfg(feature = "defmt")]
            // defmt::debug!("cookie: {:X}", *cookie);

            Some(cookie)
        } else {
            None
        };

        if header & flag::Z == flag::Z {
            unimplemented!()
        }

        if header & flag::A == flag::A {
            Ok(TransportMessage {
                body: TransportBody::InitAck(InitSyn {
                    zid,
                    cookie,
                    batch_size,
                    whatami,
                    req_id_res,
                    seq_num_res,
                    version,
                }),
            })
        } else {
            Ok(TransportMessage {
                body: TransportBody::InitSyn(InitSyn {
                    zid,
                    cookie,
                    batch_size,
                    whatami,
                    req_id_res,
                    seq_num_res,
                    version,
                }),
            })
        }
    }
}
