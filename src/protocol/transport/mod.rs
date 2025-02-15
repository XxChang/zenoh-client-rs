use init::{InitSyn, Z_MID_T_INIT};

use crate::{
    iobuf::{Reader, Writer},
    transport::TransportError,
};

pub mod init;

const Z_DEFAULT_MULTICAST_BATCH_SIZE: u16 = 8192;
const Z_DEFAULT_RESOLUTION_SIZE: u8 = 2;

// Zenoh messages at zenoh-transport level
#[derive(Debug, PartialEq, Eq)]
pub enum TransportBody {
    Join,
    InitSyn(InitSyn),
    InitAck(InitSyn),
    Open,
    Close,
    KeepAlive,
    Frame,
    Fragment,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TransportMessage {
    pub body: TransportBody,
}

impl TransportMessage {
    pub fn encode<W: Writer>(&self, writer: &mut W) -> Result<(), TransportError> {
        match &self.body {
            TransportBody::InitSyn(b) => {
                b.encode(writer)?;
            }
            _ => todo!(),
        }

        Ok(())
    }

    pub fn decode<R: Reader>(reader: &mut R) -> Result<Self, TransportError> {
        let header = reader.read_u8()?;

        match header & 0x1f {
            Z_MID_T_INIT => init::InitSyn::decode(reader, header),
            _ => {
                unimplemented!("Unknown message type: {}", header);
            }
        }
    }
}
