pub mod whatami;
pub mod transport;

pub struct ZenohID(u128);

impl From<u128> for ZenohID {
    fn from(id: u128) -> Self {
        ZenohID(id)
    }
}
