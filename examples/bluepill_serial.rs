#![allow(clippy::empty_loop)]
#![deny(unsafe_code)]
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_hal_compat::ForwardCompat;
use panic_probe as _;
use stm32f1xx_hal::{pac::{self, USART1}, prelude::*, serial::{Config, Rx, Serial}};
use zenoh_client_rs::{link::serial::SerialIntf, protocol::{whatami::WhatAmI, ZenohID}};

struct WrapperRx(pub Rx<USART1>);

impl embedded_io::ErrorType for WrapperRx {
    type Error = core::convert::Infallible;
}

impl embedded_io::Read for WrapperRx {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut idx = 0;

        while idx < buf.len() {
            match self.0.read() {
                Ok(byte) => {
                    buf[idx] = byte;
                    idx += 1;
                },
                Err(nb::Error::WouldBlock) => {
                    break;
                },
                _ => {
                    unreachable!()
                }
            }
        };

        Ok(idx)
    }
}

#[entry]
fn main() -> ! {
    defmt::info!("bluepill serial example");

    let p = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut flash = p.FLASH.constrain();
    let rcc = p.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut gpioa = p.GPIOA.split();

    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let rx = gpioa.pa10;

    let mut afio = p.AFIO.constrain();

    let serial = Serial::new(
        p.USART1, 
        (tx, rx), 
        &mut afio.mapr, 
        Config::default().baudrate(115200.bps()), 
        &clocks);

    let (tx, rx) = serial.split();
    
    let rx = WrapperRx(rx);
    
    let tx = tx.forward();

    let delay = cp.SYST.delay(&clocks);
    let delay = delay.forward();

    let intf = SerialIntf::new(rx, tx, delay);
    
    let id = ZenohID::from(0x49);
    let mode = WhatAmI::default();
    let cfg = zenoh_client_rs::Config::new(id, mode);

    zenoh_client_rs::open(&cfg, intf).unwrap();

    loop {}
}