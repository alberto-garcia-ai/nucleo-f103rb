#![no_main]
#![no_std]

use panic_halt as _;

use nucleo_f103rb as board;

use board::hal::{pac, prelude::*, serial::*};
use cortex_m_rt::entry;
use nb::block;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();

    let mut flash = p.FLASH.constrain();
    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut afio = p.AFIO.constrain();

    /* Split GPIO pins */
    let mut gpioa = p.GPIOA.split();

    /* Prepare pins USART2 is connected to */
    let tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
    let rx = gpioa.pa3;

    /* Setup USART2 which is connected to the on board ST-Link for output */
    let serial = Serial::new(
        p.USART2,
        (tx, rx),
        &mut afio.mapr,
        Config::default().baudrate(115_200.bps()),
        &clocks,
    );

    let (mut tx, mut rx) = serial.split();

    /* Print a nice hello message */
    let s = b"\r\nPlease type characters to echo:\r\n";

    let _ = s.iter().map(|c| block!(tx.write(*c))).last();

    /* Endless loop */
    loop {
        /* Read and echo back */
        if let Ok(c) = block!(rx.read()) {
            let _ = block!(tx.write(c));
        }
    }
}
