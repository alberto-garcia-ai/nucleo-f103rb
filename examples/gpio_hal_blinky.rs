#![no_main]
#![no_std]

use panic_halt as _;

use nucleo_f103rb as board;

use board::hal::{pac, prelude::*};
use cortex_m_rt::entry;

use nb::block;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut gpioa = dp.GPIOA.split();

    let mut led = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);
    let mut timer = dp.TIM2.counter_ms(&clocks);
    timer.start(2000u32.millis()).unwrap();

    loop {
        led.toggle();
        block!(timer.wait()).unwrap();
    }
}
