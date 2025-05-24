use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::iobuf::ZVec;
use crate::link::{Link, LinkIntf};
use crate::protocol::transport::init::InitSyn;
use crate::protocol::transport::open::OpenSyn;
use crate::protocol::transport::{TransportBody, TransportMessage};
use crate::protocol::{whatami::WhatAmI, ZenohID};
use crate::Z_TRANSPORT_LEASE;

use super::TransportError;

pub struct Unicast<L> {
    intf: Link<L>,
    cache: ZVec,
    open_cache: ZVec,
}

pub struct UnicastParams {
    pub zid: ZenohID,
    pub batch_size: u16,
    pub initial_sn_rx: u32,
    pub initial_sn_tx: u32,
    pub lease: u32,
    pub whatami: WhatAmI,
    pub key_id_res: u8,
    pub req_id_res: u8,
    pub seq_num_res: u8,
    pub is_qos: bool,
}

impl Default for UnicastParams {
    fn default() -> Self {
        Self {
            zid: Default::default(),
            batch_size: 0,
            initial_sn_rx: 0,
            initial_sn_tx: 0,
            lease: Z_TRANSPORT_LEASE,
            whatami: Default::default(),
            key_id_res: 0,
            req_id_res: 0,
            seq_num_res: 0,
            is_qos: false,
        }
    }
}

impl<L: LinkIntf> Unicast<L> {
    pub fn new(intf: Link<L>) -> Self {
        Unicast {
            intf,
            cache: ZVec::new(),
            open_cache: ZVec::new(),
        }
    }

    pub fn handshake(
        &mut self,
        whatami: WhatAmI,
        zid: ZenohID,
    ) -> Result<UnicastParams, TransportError> {
        let ism = InitSyn::new(whatami, zid);
        let mut params: UnicastParams = Default::default();

        let (seq_num_res, req_id_res, batch_size) = if let TransportMessage {
            body: TransportBody::InitSyn(ism),
        } = &ism
        {
            (ism.seq_num_res, ism.req_id_res, ism.batch_size)
        } else {
            return Err(TransportError::UnexpectMsg);
        };
        params.seq_num_res = seq_num_res;
        params.req_id_res = req_id_res;
        params.batch_size = batch_size;

        #[cfg(feature = "defmt")]
        defmt::debug!("Sending Z_INIT(Syn)");

        ism.encode(&mut self.cache)?;
        self.intf.send_msg(&self.cache.as_slice())?;
        self.cache.clear();

        let mut s = self.cache.extract_slice(self.intf.mtu)?;
        let size = self.intf.recv_msg(s.as_mut())?;
        s.truncate(size);
        let iam = TransportMessage::decode(&mut s)?;

        let iam = if let TransportMessage {
            body: TransportBody::InitAck(iam),
        } = iam
        {
            #[cfg(feature = "defmt")]
            defmt::debug!("Received Z_INIT(Ack)");
            iam
        } else {
            return Err(TransportError::UnexpectMsg);
        };
        // Any of the size parameters in the InitAck must be less or equal than the one in the InitSyn,
        // otherwise the InitAck message is considered invalid and it should be treated as a
        // CLOSE message with L==0 by the Initiating Peer -- the recipient of the InitAck message.
        params.seq_num_res = if params.seq_num_res >= iam.seq_num_res {
            iam.seq_num_res
        } else {
            return Err(TransportError::OpenSnResolution);
        };

        params.req_id_res = if params.req_id_res >= iam.req_id_res {
            iam.req_id_res
        } else {
            return Err(TransportError::OpenSnResolution);
        };

        params.batch_size = if params.batch_size >= iam.batch_size {
            iam.batch_size
        } else {
            return Err(TransportError::OpenSnResolution);
        };

        params.key_id_res = 0x08 << params.key_id_res;
        params.req_id_res = 0x08 << params.req_id_res;

        params.initial_sn_tx = SmallRng::seed_from_u64(0).random();
        params.initial_sn_tx = params.initial_sn_tx & !_z_sn_modulo_mask(params.seq_num_res);

        params.zid = iam.zid;

        OpenSyn::new(
            Z_TRANSPORT_LEASE,
            params.initial_sn_tx,
            Some(&iam.cookie.unwrap()),
        )
        .encode(&mut self.open_cache)?;
        #[cfg(feature = "defmt")]
        defmt::debug!("Sending Z_OPEN(Syn)");
        self.intf.send_msg(&self.open_cache.as_slice())?;
        self.open_cache.clear();

        let mut s = self.open_cache.extract_slice(self.intf.mtu)?;
        let size = self.intf.recv_msg(s.as_mut())?;
        s.truncate(size);
        let oam = TransportMessage::decode(&mut s)?;

        let oam = if let TransportMessage {
            body: TransportBody::OpenAck(oam),
        } = oam
        {
            #[cfg(feature = "defmt")]
            defmt::debug!("Received Z_OPEN(Ack)");
            oam
        } else {
            return Err(TransportError::UnexpectMsg);
        };

        #[cfg(feature = "defmt")]
        defmt::debug!("sn {}", oam.initial_sn);

        params.lease = oam.lease;
        params.initial_sn_rx = oam.initial_sn;

        Ok(params)
    }

    pub fn update(&mut self, _params: &UnicastParams) -> Result<(), TransportError> {
        Ok(())
    }
}

fn _z_sn_modulo_mask(bits: u8) -> u32 {
    match bits {
        0x00 => (u8::MAX >> 1) as u32,
        0x01 => (u16::MAX >> 2) as u32,
        0x02 => u32::MAX >> 4,
        0x03 => (u64::MAX >> 1) as u32,
        _ => unreachable!(),
    }
}
