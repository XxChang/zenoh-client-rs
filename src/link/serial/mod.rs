use cobs::{decode_with_sentinel, CobsEncoder};
use crctab::compute_crc32;
use embedded_hal::delay::DelayNs;

mod crctab;

mod flags {
    pub const INIT: u8 = 0x01;
    pub const ACK: u8 = 0x02;
    pub const RESET: u8 = 0x04;
}

/// ZSerial Frame Format
///
/// Using COBS
///
/// +-+-+----+------------+--------+-+
/// |O|H|XXXX|ZZZZ....ZZZZ|CCCCCCCC|0|
/// +-+----+------------+--------+-+
/// |O| |Len |   Data     |  CRC32 |C|
/// +-+-+-2--+----N-------+---4----+-+
///
/// Header: 1byte
/// +---------------+
/// |7|6|5|4|3|2|1|0|
/// +---------------+
/// |x|x|x|x|x|R|A|I|
/// +---------------+
///
/// Flags:
/// I - Init
/// A - Ack
/// R - Reset
///
/// Max Frame Size: 1510
/// Max MTU: 1500
/// Max On-the-wire length: 1516 (MFS + Overhead Byte (OHB) + Kind Byte + End of packet (EOP))

const COBS_BUF_SIZE: usize = 1517;
const SERIAL_CONNECT_THROTTLE_TIME_MS: u32 = 250;

pub(crate) fn serialize_into(
    header: u8,
    data: &[u8],
    dest: &mut [u8],
) -> Result<usize, super::LinkError> {
    let mut enc = CobsEncoder::new(dest);
    #[cfg(feature = "defmt")]
    defmt::debug!("SerialIntf::serialize_into: header={:02X}, data={:02X}", header, data);

    enc.push(&[header])?;

    let len_bytes = (data.len() as u16).to_le_bytes();

    enc.push(&len_bytes)?;

    enc.push(data)?;

    let crc_bytes = compute_crc32(data).to_le_bytes();
    enc.push(&crc_bytes)?;

    let mut written = enc.finalize();

    for x in &mut dest[..written] {
        *x ^= 0x00;
    }

    dest[written] = 0;
    written += 1;

    Ok(written)
}

const KIND_FIELD_LEN: usize = 1;
const LEN_FIELD_LEN: usize = 2;
const CRC32_LEN: usize = 4;

pub(crate) fn deserialize_from(
    source: &[u8],
    dest: &mut [u8],
) -> Result<(usize, u8), super::LinkError> {
    decode_with_sentinel(source, dest, 0)?;

    let header = dest[0];

    let wire_size = u16::from_le_bytes([dest[1], dest[2]]) as usize;

    if wire_size + KIND_FIELD_LEN + LEN_FIELD_LEN + CRC32_LEN > dest.len() {
        return Err(super::LinkError::DecodeError(
            cobs::DecodeError::TargetBufTooSmall,
        ));
    }

    let compute_crc = compute_crc32(&dest[KIND_FIELD_LEN + LEN_FIELD_LEN..KIND_FIELD_LEN + wire_size + LEN_FIELD_LEN]);

    let received_crc = &dest[KIND_FIELD_LEN + LEN_FIELD_LEN + wire_size..KIND_FIELD_LEN + LEN_FIELD_LEN + wire_size + CRC32_LEN];

    let received_crc = u32::from_le_bytes([
        received_crc[0],
        received_crc[1],
        received_crc[2],
        received_crc[3],
    ]);

    if compute_crc != received_crc {
        return Err(super::LinkError::CrcError);
    }

    Ok((wire_size, header))
}

pub struct SerialIntf<RX, TX, Delay> {
    rx: RX,
    tx: TX,
    send_buf: [u8; COBS_BUF_SIZE],
    recv_buf: [u8; COBS_BUF_SIZE],
    delay: Delay,
}

impl<RX, TX, Delay> SerialIntf<RX, TX, Delay>
where
    RX: embedded_io::Read,
    TX: embedded_io::Write,
    Delay: DelayNs,
{
    pub fn new(rx: RX, tx: TX, delay: Delay) -> Self {
        Self {
            rx,
            tx,
            send_buf: [0u8; COBS_BUF_SIZE],
            recv_buf: [0u8; COBS_BUF_SIZE],
            delay,
        }
    }

    fn internal_send(&mut self, header: u8, data: &[u8]) -> Result<(), super::LinkError> {
        let len = serialize_into(header, data, &mut self.send_buf)?;
        #[cfg(feature = "defmt")]
        defmt::debug!("Sending {:02X}", self.send_buf[..len]);
        
        self.tx
            .write_all(&self.send_buf[..len])
            .map_err(|_e| super::LinkError::IoError)?;
        self.tx
            .flush()
            .map_err(|_| super::LinkError::IoError)?;
        
        Ok(())
    }

    fn internal_read(&mut self, buf: &mut [u8]) -> Result<(usize, u8), super::LinkError> {
        let mut start_count = 0;

        // Read
        loop {
            if start_count == COBS_BUF_SIZE {
                return Ok((0, 0));
            }

            self.rx
                .read_exact(core::slice::from_mut(&mut self.recv_buf[start_count]))
                .map_err(|_e| super::LinkError::IoError)?;

            if self.recv_buf[start_count] == 0 {
                break;
            }

            start_count += 1;
        }

        start_count += 1;

        deserialize_from(&self.recv_buf[0..start_count], buf)
    }

    pub fn connect(&mut self) -> Result<(), super::LinkError> {
        let mut buff = [0u8; COBS_BUF_SIZE];

        loop {
            self.internal_send(flags::INIT, &[])?;
            #[cfg(feature = "defmt")]
            defmt::debug!("Sent INIT");

            let (_size, header) = self.internal_read(&mut buff)?;
            #[cfg(feature = "defmt")]
            defmt::debug!("Received {:02X}", header);

            if header & (flags::ACK | flags::INIT) == flags::ACK | flags::INIT {
                #[cfg(feature = "defmt")]
                defmt::debug!("Connected");
                break;
            } else if header & flags::RESET == flags::RESET {
                self.delay.delay_ms(SERIAL_CONNECT_THROTTLE_TIME_MS);
                #[cfg(feature = "defmt")]
                defmt::debug!("Reset");
            } else {
                #[cfg(feature = "defmt")]
                defmt::error!("Unknown Header received: {:X}", header);
                return Err(super::LinkError::IoError);
            }
        }

        Ok(())
    }
}
