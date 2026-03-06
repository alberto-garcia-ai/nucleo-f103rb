//! Semihosting output with hprintln! — Nucleo-F103RB
//!
//! # What is semihosting?
//!
//! ARM semihosting lets the microcontroller send output to and receive input
//! from the host PC through the debug interface (ST-Link), without needing a
//! UART connection. It works by executing a `BKPT` instruction with a special
//! immediate value; the debugger (OpenOCD) intercepts it, performs the host
//! I/O, and resumes the MCU transparently.
//!
//! # When to use semihosting vs UART
//!
//! - **Semihosting (`hprintln!`)**: quick debugging when a debugger is already
//!   attached. No extra wiring needed beyond the ST-Link that is built into the
//!   Nucleo board.
//! - **UART console (`board::println!`)**: faster, works without a debugger,
//!   suitable for production logging or when the debugger is not connected.
//!   See the `serial_hal_console` example.
//!
//! > ⚠️ **Performance warning**: semihosting is slow. Each `hprintln!` call
//! > halts the CPU and waits for the host to perform the I/O before resuming.
//! > Do not use it in timing-sensitive or interrupt-heavy code paths.
//!
//! > ⚠️ **Debugger required**: if the MCU executes a semihosting call without
//! > a debugger attached and semihosting enabled, it will trigger a hard fault.
//! > Use `panic-halt` (as this example does) or `panic-semihosting` as the
//! > panic handler, but never ship semihosting calls in production firmware.
//!
//! # How to run
//!
//! **Terminal 1** — Start OpenOCD and leave it running. Semihosting is
//! enabled automatically by `nucleo.cfg`:
//! ```sh
//! openocd -f nucleo.cfg
//! ```
//!
//! **Terminal 2** — Build, flash, and run in one command. `.cargo/config.toml`
//! sets the runner to `gdb-multiarch -x openocd.gdb`, which connects to
//! OpenOCD, flashes the binary, and starts execution automatically:
//! ```sh
//! cargo run --example semihosting_hprintln
//! ```
//!
//! Output from `hprintln!` appears in **Terminal 1** (the OpenOCD window),
//! not in the terminal where you ran `cargo run`.

#![no_main]
#![no_std]

use panic_halt as _;

use nucleo_f103rb as board;
use board::hal::{pac, prelude::*};
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use nb::block;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut gpioa = dp.GPIOA.split();

    /* PA5 is the on-board green LED. Toggling it provides a visible heartbeat
     * that confirms the MCU is running even if the semihosting output window
     * is not visible. */
    let mut led = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);

    /* TIM2 generates a 1-second period so the OpenOCD console output is
     * readable at a comfortable pace. */
    let mut timer = dp.TIM2.counter_ms(&clocks);
    timer.start(1_000u32.millis()).unwrap();

    /* hprintln! formats a string and sends it to the host via the semihosting
     * syscall interface. In cortex-m-semihosting 0.5 the macro returns (),
     * so there is no error value to handle. */
    hprintln!("Semihosting ready — Nucleo-F103RB");
    hprintln!("LED blinks once per second; counter increments each blink.");
    hprintln!("");

    let mut count: u32 = 0;
    loop {
        led.toggle();
        count += 1;

        /* Formatted output works the same as println! — use {} for Display,
         * {:?} for Debug, {:02X} for zero-padded hex, etc. */
        hprintln!("blink #{}", count);

        block!(timer.wait()).unwrap();
    }
}
