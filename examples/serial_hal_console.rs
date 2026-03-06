//! Bidirectional UART2 console example.
//!
//! Prints a prompt over the ST-Link USB-UART bridge (115 200 baud), reads a
//! line of text entered by the user, and echoes it back.
//!
//! Connect with any serial terminal (e.g. `minicom`, `screen`, PuTTY) at
//! 115200 8N1.

#![no_main]
#![no_std]

use panic_halt as _;

use nucleo_f103rb as board;
use board::hal::{pac, prelude::*, serial::{Config, Serial}};
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();

    let mut flash = p.FLASH.constrain();
    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut afio = p.AFIO.constrain();
    let mut gpioa = p.GPIOA.split();

    let tx_pin = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
    let rx_pin = gpioa.pa3;

    let serial = Serial::new(
        p.USART2,
        (tx_pin, rx_pin),
        &mut afio.mapr,
        Config::default().baudrate(115_200.bps()),
        &clocks,
    );

    let (tx, rx) = serial.split();
    board::console::init(tx, rx);

    board::println!("Nucleo-F103RB console ready. Type something and press Enter:");

    let mut buf = [0u8; 64];
    loop {
        board::print!("> ");
        let n = board::console::read_line(&mut buf);
        if let Ok(s) = core::str::from_utf8(&buf[..n]) {
            board::println!("You typed: {}", s);
        }
    }
}
