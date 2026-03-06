//! Simple CAN echo example.
//! Requires an external CAN transceiver (e.g. SN65HVD230) connected to
//! PA11 (RX) and PA12 (TX) for CAN1, or PB5 (RX) and PB6 (TX) for CAN2
//! (connectivity feature only).

#![no_main]
#![no_std]

use bxcan::Fifo;
use panic_halt as _;

use bxcan::filter::Mask32;
use cortex_m_rt::entry;
use nb::block;
use nucleo_f103rb as board;
use board::hal::{can::Can, pac, prelude::*};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    // To meet CAN clock accuracy requirements an external crystal or ceramic
    // resonator must be used. The Nucleo-F103RB provides 8MHz via the ST-Link MCO.
    rcc.cfgr.use_hse(8.MHz()).freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain();

    let mut can1 = {
        /* CAN1 shares its clock gate with USB on the non-connectivity STM32F103RB */
        let can = Can::new(dp.CAN1, dp.USB);

        let mut gpioa = dp.GPIOA.split();
        let rx = gpioa.pa11.into_floating_input(&mut gpioa.crh);
        let tx = gpioa.pa12.into_alternate_push_pull(&mut gpioa.crh);
        can.assign_pins((tx, rx), &mut afio.mapr);

        // APB1 (PCLK1): 8MHz, Bit rate: 125kBit/s, Sample Point 87.5%
        // Value was calculated with http://www.bittiming.can-wiki.info/
        bxcan::Can::builder(can)
            .set_bit_timing(0x001c_0003)
            .leave_disabled()
    };

    // Configure filters so that can frames can be received.
    let mut filters = can1.modify_filters();
    filters.enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

    // Drop filters to leave filter configuration mode.
    drop(filters);

    // Split the peripheral into transmitter and receiver parts.
    let mut can = can1;
    block!(can.enable_non_blocking()).unwrap();

    // Echo back received packages in sequence.
    // See the `can_hal_rtic` example for an echo implementation that adheres to
    // correct frame ordering based on the transfer id.
    loop {
        if let Ok(frame) = block!(can.receive()) {
            block!(can.transmit(&frame)).unwrap();
        }
    }
}
