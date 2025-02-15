use cobs::decode_in_place_with_sentinel;
use crctab::compute_crc32;
use embedded_hal::delay::DelayNs;
use heapless::{Deque, Vec};

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

const KIND_FIELD_LEN: usize = 1;
const LEN_FIELD_LEN: usize = 2;
const CRC32_LEN: usize = 4;

pub(crate) fn deserialize_from(source: &mut [u8]) -> Result<(usize, u8), super::LinkError> {
    decode_in_place_with_sentinel(source, 0)?;

    let header = source[0];

    let wire_size = u16::from_le_bytes([source[1], source[2]]) as usize;

    if wire_size + KIND_FIELD_LEN + LEN_FIELD_LEN + CRC32_LEN > source.len() {
        return Err(super::LinkError::DecodeError(
            cobs::DecodeError::TargetBufTooSmall,
        ));
    }

    let compute_crc = compute_crc32(
        &source[KIND_FIELD_LEN + LEN_FIELD_LEN..KIND_FIELD_LEN + wire_size + LEN_FIELD_LEN],
    );

    let received_crc = &source[KIND_FIELD_LEN + LEN_FIELD_LEN + wire_size
        ..KIND_FIELD_LEN + LEN_FIELD_LEN + wire_size + CRC32_LEN];

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

enum CodecState {
    Header,
    LenLSB,
    LenMSB,
    Data,
    Crc,
}

pub struct SerialIntf<RX, TX, Delay> {
    rx: RX,
    tx: TX,

    delay: Delay,

    codec_state: CodecState,
}

impl<RX, TX, Delay> SerialIntf<RX, TX, Delay>
where
    RX: embedded_io::Read,
    TX: embedded_io::Write,
    Delay: DelayNs,
{
    pub fn name(&self) -> &'static str {
        "Serial"
    }

    pub fn new(rx: RX, tx: TX, delay: Delay) -> Self {
        Self {
            rx,
            tx,

            delay,

            codec_state: CodecState::Header,
        }
    }

    fn send_patch(&mut self, overhead: u8, data: &[u8]) -> Result<(), super::LinkError> {
        self.tx
            .write_all(&[overhead])
            .map_err(|_| super::LinkError::IoError)?;
        self.tx
            .write_all(data)
            .map_err(|_e| super::LinkError::IoError)
    }

    fn internal_send(&mut self, header: u8, data: &[u8]) -> Result<(), super::LinkError> {
        let bytes_len = data.len();
        let crc = compute_crc32(data);
        let len_bytes = (bytes_len as u16).to_le_bytes();
        let crc_bytes = crc.to_le_bytes();

        let mut overhead = 1;

        let mut prev_data = Deque::<u8, 5>::new();
        let mut data_start_idx = 0usize;
        let mut data_idx = 0usize;
        let mut crc_start_idx = 0usize;
        let mut crc_idx = 0usize;
        self.codec_state = CodecState::Header;

        loop {
            match self.codec_state {
                CodecState::Header => {
                    if header == 0x00 {
                        self.send_patch(overhead, &[])?;
                        overhead = 1;
                    } else {
                        overhead += 1;
                        prev_data
                            .push_back(header)
                            .map_err(|_| super::LinkError::IoError)?;
                    }

                    self.codec_state = CodecState::LenLSB;
                }
                CodecState::LenLSB => {
                    if len_bytes[0] == 0x00 {
                        let mut send_data = Vec::<u8, 1>::new();
                        if let Some(d) = prev_data.pop_front() {
                            send_data.push(d).map_err(|_| super::LinkError::IoError)?;
                        }
                        self.send_patch(overhead, send_data.as_slice())?;
                        overhead = 1;
                    } else {
                        overhead += 1;
                        prev_data
                            .push_back(len_bytes[0])
                            .map_err(|_| super::LinkError::IoError)?;
                    }

                    self.codec_state = CodecState::LenMSB;
                }
                CodecState::LenMSB => {
                    if len_bytes[1] == 0x00 {
                        let mut send_data = Vec::<u8, 2>::new();
                        while let Some(d) = prev_data.pop_front() {
                            send_data.push(d).map_err(|_| super::LinkError::IoError)?;
                        }
                        self.send_patch(overhead, send_data.as_slice())?;
                        overhead = 1;
                    } else {
                        overhead += 1;
                        prev_data
                            .push_back(len_bytes[1])
                            .map_err(|_| super::LinkError::IoError)?;
                    }

                    self.codec_state = CodecState::Data;
                }
                CodecState::Data => {
                    if data.is_empty() {
                        self.codec_state = CodecState::Crc;
                        continue;
                    }

                    if overhead == 0xff {
                        let mut data_end_idx = data_start_idx + overhead as usize - 1;
                        if !prev_data.is_empty() {
                            data_end_idx -= prev_data.len();
                            let mut send_data = Vec::<u8, 3>::new();
                            while let Some(d) = prev_data.pop_front() {
                                send_data.push(d).map_err(|_| super::LinkError::IoError)?;
                            }
                            self.send_patch(overhead, send_data.as_slice())?;
                            self.tx
                                .write_all(&data[data_start_idx..data_end_idx])
                                .map_err(|_| super::LinkError::IoError)?;
                        } else {
                            self.send_patch(overhead, &data[data_start_idx..data_end_idx])?;
                        }
                        data_start_idx = data_end_idx;
                        overhead = 1;
                    } else if data[data_idx] == 0x00 {
                        let mut data_end_idx = data_start_idx + overhead as usize - 1;
                        if !prev_data.is_empty() {
                            data_end_idx -= prev_data.len();
                            let mut send_data = Vec::<u8, 3>::new();
                            while let Some(d) = prev_data.pop_front() {
                                send_data.push(d).map_err(|_| super::LinkError::IoError)?;
                            }
                            self.send_patch(overhead, send_data.as_slice())?;
                            self.tx
                                .write_all(&data[data_start_idx..data_end_idx])
                                .map_err(|_| super::LinkError::IoError)?;
                        } else {
                            self.send_patch(overhead, &data[data_start_idx..data_end_idx])?;
                        }
                        // Skip
                        data_start_idx = data_end_idx + 1;
                        overhead = 1;
                    } else {
                        overhead += 1;
                    }

                    data_idx += 1;
                    if data_idx >= bytes_len {
                        self.codec_state = CodecState::Crc;
                    }
                }
                CodecState::Crc => {
                    if overhead == 0xff {
                        // if prev_data is not empty
                        // there are no zero in data seq
                        let mut crc_end_idx = crc_start_idx + overhead as usize - 1;
                        if !prev_data.is_empty() {
                            crc_end_idx = crc_end_idx - prev_data.len() - bytes_len;
                            let mut send_data = Vec::<u8, 3>::new();
                            while let Some(d) = prev_data.pop_front() {
                                send_data.push(d).map_err(|_| super::LinkError::IoError)?;
                            }
                            self.send_patch(overhead, send_data.as_slice())?;
                            self.tx
                                .write_all(&data[data_start_idx..])
                                .map_err(|_| super::LinkError::IoError)?;
                            self.tx
                                .write_all(&crc_bytes[crc_start_idx..crc_end_idx])
                                .map_err(|_| super::LinkError::IoError)?;
                        } else if data_start_idx < bytes_len {
                            crc_end_idx = crc_end_idx - data[data_start_idx..].len();
                            self.send_patch(overhead, &data[data_start_idx..])?;
                            data_start_idx = bytes_len;
                            self.tx
                                .write_all(&crc_bytes[crc_start_idx..crc_end_idx])
                                .map_err(|_| super::LinkError::IoError)?;
                        } else {
                            self.send_patch(overhead, &crc_bytes[crc_start_idx..crc_end_idx])?;
                        }
                        crc_start_idx = crc_end_idx;
                        overhead = 1;
                    } else if crc_bytes[crc_idx] == 0x00 {
                        let mut crc_end_idx = crc_start_idx + overhead as usize - 1;
                        if !prev_data.is_empty() {
                            crc_end_idx = crc_end_idx - prev_data.len() - bytes_len;
                            let mut send_data = Vec::<u8, 3>::new();
                            while let Some(d) = prev_data.pop_front() {
                                send_data.push(d).map_err(|_| super::LinkError::IoError)?;
                            }
                            self.send_patch(overhead, send_data.as_slice())?;
                            self.tx
                                .write_all(&data[data_start_idx..])
                                .map_err(|_| super::LinkError::IoError)?;
                            self.tx
                                .write_all(&crc_bytes[crc_start_idx..crc_end_idx])
                                .map_err(|_| super::LinkError::IoError)?;
                        } else if data_start_idx < bytes_len {
                            crc_end_idx = crc_end_idx - data[data_start_idx..].len();
                            self.send_patch(overhead, &data[data_start_idx..])?;
                            data_start_idx = bytes_len;
                            self.tx
                                .write_all(&crc_bytes[crc_start_idx..crc_end_idx])
                                .map_err(|_| super::LinkError::IoError)?;
                        } else {
                            self.send_patch(overhead, &crc_bytes[crc_start_idx..crc_end_idx])?;
                        }
                        overhead = 1;
                        // skip
                        crc_start_idx = crc_end_idx + 1;
                    } else {
                        overhead += 1;
                    }

                    crc_idx += 1;

                    if crc_idx >= 4 {
                        let mut send_data = Vec::<u8, 3>::new();
                        while let Some(d) = prev_data.pop_front() {
                            send_data.push(d).map_err(|_| super::LinkError::IoError)?;
                        }
                        self.send_patch(overhead, &send_data.as_slice())?;
                        if data_start_idx < bytes_len {
                            self.tx
                                .write_all(&data[data_start_idx..])
                                .map_err(|_| super::LinkError::IoError)?;
                        }
                        if crc_start_idx < 4 {
                            self.tx
                                .write_all(&crc_bytes[crc_start_idx..])
                                .map_err(|_| super::LinkError::IoError)?;
                        }
                        break;
                    }
                }
            }
        }

        self.tx
            .write_all(&[0])
            .map_err(|_| super::LinkError::IoError)?;
        self.tx.flush().map_err(|_| super::LinkError::IoError)?;

        Ok(())
    }

    fn internal_read(&mut self, buf: &mut [u8]) -> Result<(usize, u8), super::LinkError> {
        let mut start_count = 0;

        // Read
        loop {
            if start_count == buf.len() {
                return Ok((0, 0));
            }

            self.rx
                .read_exact(core::slice::from_mut(&mut buf[start_count]))
                .map_err(|_| super::LinkError::IoError)?;

            if buf[start_count] == 0 {
                break;
            }

            start_count += 1;
        }

        start_count += 1;

        #[cfg(feature = "defmt")]
        defmt::trace!("recv {:X}", buf[..start_count]);

        let (wire_size, head) = deserialize_from(&mut buf[0..start_count])?;
        buf.copy_within(3..3 + wire_size, 0);
        Ok((wire_size, head))
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), super::LinkError> {
        self.internal_send(0, data)
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> Result<usize, super::LinkError> {
        let (size, _) = self.internal_read(buf)?;
        Ok(size)
    }

    // pub fn recv_in_place(&mut self) -> Result<(&[u8], u8), super::LinkError> {
    //     self.internal_read_in_place()
    // }

    pub fn connect(&mut self) -> Result<(), super::LinkError> {
        let mut buff = [0u8; COBS_BUF_SIZE];

        loop {
            self.internal_send(flags::INIT, &[])?;
            #[cfg(feature = "defmt")]
            defmt::debug!("Sent INIT");

            let (_size, header) = self.internal_read(&mut buff)?;

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
