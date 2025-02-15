use crate::{iobuf::Reader, transport::TransportError};

pub mod transport;
pub mod whatami;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ZenohID([u8; ZenohID::MAX_SIZE]);

impl ZenohID {
    pub const MAX_SIZE: usize = u128::BITS as usize / 8;

    #[inline]
    pub fn size(&self) -> usize {
        Self::MAX_SIZE - (u128::from_le_bytes(self.0).leading_zeros() as usize / 8)
    }

    #[inline]
    pub fn to_le_bytes(&self) -> [u8; ZenohID::MAX_SIZE] {
        self.0
    }
}

impl From<u128> for ZenohID {
    fn from(id: u128) -> Self {
        ZenohID(id.to_le_bytes())
    }
}

pub(crate) struct Varint<T> {
    _p: core::marker::PhantomData<T>,
}

impl<T> Varint<T> {
    // pub fn encode(&mut self, value: T) -> usize
    // where
    //     T: num_traits::PrimInt,
    // {
    //     let mut value = value;
    //     let mut i = 0;
    //     loop {
    //         let mut byte = (value & T::from(0x7F).unwrap()).to_u8().unwrap();
    //         value = value >> 7;
    //         if value != T::zero() {
    //             byte |= 0x80;
    //         }
    //         self.bytes[i] = byte;
    //         i += 1;
    //         if value == T::zero() {
    //             break;
    //         }
    //     }
    //     i
    // }

    pub fn decode<R: Reader>(reader: &mut R) -> Result<T, TransportError>
    where
        T: num_traits::PrimInt,
    {
        let size = core::mem::size_of::<T>();

        let mut value = T::zero();
        let mut shift = 0;
        for _ in 0..size + 1 {
            let byte = reader.read_u8()?;
            value = value | T::from(byte & 0x7F).unwrap() << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }

        Ok(value)
    }
}
