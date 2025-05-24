#![deny(unsafe_code)]
#![allow(clippy::empty_loop)]
#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m_rt::entry;
use defmt_rtt as _;
use panic_probe as _;
use stm32f1xx_hal::{
    pac,
    prelude::*,
    timer::{Channel, Tim2NoRemap},
};

#[entry]
fn main() -> ! {
    defmt::info!("pwm test example");

    let p = pac::Peripherals::take().unwrap();

    let mut flash = p.FLASH.constrain();
    let rcc = p.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut afio = p.AFIO.constrain();

    let mut gpioa = p.GPIOA.split();

    // TIM2
    let c1 = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
    let c2 = gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl);
    let c3 = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);

    let pins = (c1, c2, c3);

    let mut pwm = p
        .TIM2
        .pwm_hz::<Tim2NoRemap, _, _>(pins, &mut afio.mapr, 1.kHz(), &clocks);

    pwm.enable(Channel::C1);
    pwm.enable(Channel::C2);
    pwm.enable(Channel::C3);

    // asm::bkpt();

    // Return to the original frequency
    pwm.set_period(1.kHz());

    // asm::bkpt();

    let max = pwm.get_max_duty();
    let duty = (max as f32 * 0.5) as u16;
    defmt::info!("duty {}", duty);
    pwm.set_duty(Channel::C3, (max as f32 * 0.7) as u16);
    pwm.set_duty(Channel::C1, (max as f32 * 0.7) as u16);
    pwm.set_duty(Channel::C2, (max as f32 * 0.7) as u16);

    loop {}
}
