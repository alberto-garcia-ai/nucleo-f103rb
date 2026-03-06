//! I2C bus scanner — an interactive guide to I2C on the Nucleo-F103RB.
//!
//! # What is I2C?
//!
//! I2C (Inter-Integrated Circuit) is a two-wire serial bus consisting of:
//!   - **SCL** (Serial Clock): driven by the master to synchronise data
//!   - **SDA** (Serial Data): bidirectional data line shared by all devices
//!
//! A single master drives the clock and initiates every transfer. Multiple
//! slave devices share the same two wires, each identified by a unique 7-bit
//! address. When the master addresses a device, that device responds with an
//! ACK (acknowledge) bit; if no device is present the line stays HIGH, which
//! the master reads as NACK (no-acknowledge).
//!
//! # What this example does
//!
//! Scans all valid 7-bit I2C addresses (0x08 to 0x77) and prints the ones
//! where a device responds. Connect your I2C device(s) before flashing, then
//! open a serial terminal at 115200 baud to see the scan results.
//!
//! This is useful as a first step with any new I2C component: it confirms the
//! bus is wired correctly and tells you which address your device is using.
//!
//! # Hardware wiring (I2C1)
//!
//! ```text
//!  Nucleo-F103RB          I2C device(s)
//!  ─────────────          ─────────────
//!  PB6  (SCL)  ──┬── 4.7kΩ ── 3V3 ──── device SCL
//!                │
//!  PB7  (SDA)  ──┴── 4.7kΩ ── 3V3 ──── device SDA
//!
//!  GND          ─────────────────────── device GND
//!  3V3          ─────────────────────── device VCC  (if 3.3 V powered)
//! ```
//!
//! **Pull-up resistors are mandatory.** I2C uses open-drain signalling: devices
//! only actively pull lines LOW. External resistors (typically 4.7 kΩ) must
//! pull the lines HIGH when no device is driving them. Without pull-ups the
//! bus will not function. Many sensor breakout boards include them on-board.
//!
//! # Reading and writing after scanning
//!
//! Once you know the device address, the three embedded-hal traits give you
//! all the operations you need:
//!
//! ```rust,ignore
//! use embedded_hal::blocking::i2c::{Write, Read, WriteRead};
//!
//! // Send bytes to a device (e.g. write a register value)
//! i2c.write(addr, &[register, value])?;
//!
//! // Receive bytes from a device (e.g. read a register)
//! let mut buf = [0u8; 2];
//! i2c.read(addr, &mut buf)?;
//!
//! // Write then read without releasing the bus in between (most common for
//! // register reads: send register address, then receive register value)
//! i2c.write_read(addr, &[register], &mut buf)?;
//! ```

#![no_main]
#![no_std]

use panic_halt as _;

use nucleo_f103rb as board;
use board::hal::{
    i2c::{BlockingI2c, Mode},
    pac,
    prelude::*,
    serial::{Config, Serial},
};
use cortex_m_rt::entry;

// Bring the Write trait into scope so we can call i2c.write(...).
// The three embedded-hal I2C traits (Write, Read, WriteRead) define the
// standard interface that all HAL I2C drivers implement.
use embedded_hal::blocking::i2c::Write;

#[entry]
fn main() -> ! {
    /* Take ownership of the core (Cortex-M) and device (STM32) peripherals.
     * Each peripheral singleton can only be taken once; subsequent calls
     * return None. We unwrap here because this runs at startup with no
     * competition. */
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    /* ── Clocks ────────────────────────────────────────────────────────────
     * Freeze the RCC (Reset and Clock Control) configuration.
     * The default configuration uses the internal 8 MHz HSI oscillator.
     * The I2C peripheral is clocked from APB1 (PCLK1 = 8 MHz by default),
     * which is fast enough for Standard Mode I2C at 100 kHz. */
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    /* ── Enable the DWT cycle counter ──────────────────────────────────────
     * BlockingI2c measures elapsed time using the ARM DWT (Data Watchpoint
     * and Trace) cycle counter rather than a dedicated hardware timer. This
     * lets it implement microsecond-accurate timeouts without consuming a
     * timer peripheral. The counter must be started before BlockingI2c is
     * created, otherwise timeout calculations will be incorrect. */
    let mut dwt = cp.DWT;
    dwt.enable_cycle_counter();

    let mut afio = dp.AFIO.constrain();

    /* ── Configure I2C1 GPIO pins ──────────────────────────────────────────
     * I2C uses open-drain signalling on both SCL and SDA:
     *   - The I2C peripheral can only drive the pin LOW (assert 0).
     *   - External pull-up resistors bring the line HIGH when released.
     *   - This allows multiple devices to share the bus safely — two devices
     *     driving HIGH simultaneously is not possible.
     *
     * `into_alternate_open_drain` configures the pin for use by the I2C
     * alternate function in open-drain mode. Using push-pull mode here
     * would violate the I2C electrical specification.
     *
     * I2C1 default pin mapping: PB6 = SCL, PB7 = SDA
     * Alternative mapping:      PB8 = SCL, PB9 = SDA  (via AFIO remap) */
    let mut gpiob = dp.GPIOB.split();
    let scl = gpiob.pb6.into_alternate_open_drain(&mut gpiob.crl); // I2C1 SCL
    let sda = gpiob.pb7.into_alternate_open_drain(&mut gpiob.crl); // I2C1 SDA

    /* ── Create the BlockingI2c peripheral ────────────────────────────────
     * `BlockingI2c` wraps the non-blocking `I2c` driver with busy-wait
     * loops and DWT-based timeouts, making it straightforward to use in
     * simple polling applications.
     *
     * Timeout parameters (all in microseconds unless noted):
     *   start_timeout_us — max time to wait for the START condition on the bus
     *   start_retries    — number of START retries before returning an error
     *   addr_timeout_us  — max time to wait for the device to ACK its address
     *   data_timeout_us  — max time to wait for ACK after each data byte
     *
     * Mode::Standard sets the SCL frequency to 100 kHz. Use Mode::Fast for
     * 400 kHz if all devices on the bus support it. */
    let mut i2c = BlockingI2c::i2c1(
        dp.I2C1,
        (scl, sda),
        &mut afio.mapr,
        Mode::Standard {
            frequency: 100.kHz(),
        },
        clocks,
        1_000, /* start_timeout_us */
        10,    /* start_retries    */
        1_000, /* addr_timeout_us  */
        1_000, /* data_timeout_us  */
    );

    /* ── Set up the UART2 console for scan output ──────────────────────────
     * USART2 is connected to the on-board ST-Link USB bridge, so any serial
     * terminal on the host PC receives the output at 115200 baud 8N1. */
    let mut gpioa = dp.GPIOA.split();
    let tx_pin = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
    let rx_pin = gpioa.pa3;
    let serial = Serial::new(
        dp.USART2,
        (tx_pin, rx_pin),
        &mut afio.mapr,
        Config::default().baudrate(115_200.bps()),
        &clocks,
    );
    let (tx, rx) = serial.split();
    board::console::init(tx, rx);

    board::println!("I2C bus scanner — Nucleo-F103RB");
    board::println!("SCL: PB6   SDA: PB7   Speed: 100 kHz (Standard Mode)");
    board::println!("Scanning addresses 0x08 to 0x77...");
    board::println!("");

    /* ── Scan all valid 7-bit I2C addresses ───────────────────────────────
     * The 7-bit address space covers 0x00–0x7F (128 addresses). The I2C
     * specification reserves several ranges:
     *   0x00        — General Call address (broadcast)
     *   0x01–0x07   — Reserved for special protocol extensions
     *   0x78–0x7F   — Reserved (used for 10-bit addressing extension)
     *
     * The usable range for standard 7-bit slave devices is therefore 0x08–0x77.
     *
     * Scanning technique — zero-length write:
     *   The master sends: START | ADDR<<1 | W | STOP
     *   If a device exists at ADDR, it ACKs the address byte → Ok(())
     *   If nothing is there,    it NACKs              → Err(Acknowledge)
     *
     * No data bytes are sent, so the transaction is safe for any device. */
    let mut found = 0u32;

    for addr in 0x08u8..=0x77 {
        if i2c.write(addr, &[]).is_ok() {
            board::println!("  [0x{:02X}] Device found", addr);
            found += 1;
        }
    }

    board::println!("");
    if found == 0 {
        board::println!("Scan complete: no devices found.");
        board::println!("  → Check that pull-up resistors are present on SCL and SDA.");
        board::println!("  → Verify device power supply and GND connection.");
        board::println!("  → Confirm the device address with its datasheet.");
    } else {
        board::println!("Scan complete: {} device(s) found.", found);
    }

    /* ── Idle ──────────────────────────────────────────────────────────────
     * The scan is done. Sleep with wfi (Wait For Interrupt) to stop burning
     * CPU cycles. The board will remain powered and the serial output will
     * persist in the terminal history. */
    loop {
        cortex_m::asm::wfi();
    }
}
