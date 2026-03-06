//! Global UART2 console backed by the on-board ST-Link USB-UART bridge.
//!
//! Call [`init`] once from `main()` after splitting your [`Serial`] peripheral,
//! then use [`crate::print!`] / [`crate::println!`] for output and
//! [`read_byte`] / [`read_line`] for input.
//!
//! [`Serial`]: crate::hal::serial::Serial

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use crate::hal::serial::{Rx, Tx};
use crate::hal::pac::USART2;

// TX is protected by a critical-section Mutex so that write_str calls from
// different interrupt levels cannot interleave output.
static TX: Mutex<RefCell<Option<Tx<USART2>>>> = Mutex::new(RefCell::new(None));

// RX is accessed only from non-interrupt (main) context via read_byte /
// read_line, which block waiting for input. We use static mut to avoid
// holding the critical section (interrupts disabled) during the blocking
// poll loop, which would prevent other interrupt handlers from running.
static mut RX: Option<Rx<USART2>> = None;

/// Initialize the UART2 console. Call once from `main()` after splitting the
/// Serial peripheral. Ownership of both halves is transferred to the module.
pub fn init(tx: Tx<USART2>, rx: Rx<USART2>) {
    cortex_m::interrupt::free(|cs| {
        TX.borrow(cs).replace(Some(tx));
    });
    // Safety: called once before any concurrent use; no interrupts use RX.
    unsafe { RX = Some(rx) };
}

/// Read a single byte from UART2, blocking until one arrives.
/// Returns `None` if the console has not been initialized.
pub fn read_byte() -> Option<u8> {
    // Safety: RX is only accessed from main-thread context (non-interrupt).
    // read_byte / read_line are blocking calls not meant for use inside ISRs.
    // addr_of_mut! avoids creating a mutable reference to the static directly.
    unsafe {
        (*core::ptr::addr_of_mut!(RX))
            .as_mut()
            .map(|rx| nb::block!(rx.read()).unwrap_or(0))
    }
}

fn write_byte(b: u8) {
    cortex_m::interrupt::free(|cs| {
        if let Some(tx) = TX.borrow(cs).borrow_mut().as_mut() {
            let _ = nb::block!(tx.write(b));
        }
    });
}

/// Read bytes from UART2 into `buf` until `\r` or `\n` is received or the
/// buffer is full. Returns the number of bytes written (excluding the
/// line terminator). Returns `0` if the console has not been initialized.
///
/// Characters are echoed back as they are typed. Backspace (`0x08`) and
/// DEL (`0x7f`) erase the previous character both in the buffer and on the
/// terminal display.
pub fn read_line(buf: &mut [u8]) -> usize {
    let mut pos = 0;
    loop {
        match read_byte() {
            Some(b'\r') | Some(b'\n') => {
                write_byte(b'\r');
                write_byte(b'\n');
                break;
            }
            // Backspace (0x08) or DEL (0x7f)
            Some(0x08) | Some(0x7f) => {
                if pos > 0 {
                    pos -= 1;
                    // BS + space + BS erases the character in the terminal
                    write_byte(0x08);
                    write_byte(b' ');
                    write_byte(0x08);
                }
            }
            Some(b) if pos < buf.len() => {
                buf[pos] = b;
                pos += 1;
                write_byte(b); // echo character back
            }
            None => break,
            _ => {} // buffer full — discard input until Enter
        }
    }
    pos
}

/// Zero-sized token implementing [`core::fmt::Write`] via the global UART2 TX.
/// Prefer the [`crate::print!`] / [`crate::println!`] macros over using this
/// type directly.
pub struct Console;

impl core::fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        cortex_m::interrupt::free(|cs| {
            if let Some(tx) = TX.borrow(cs).borrow_mut().as_mut() {
                core::fmt::Write::write_str(tx, s)
            } else {
                Err(core::fmt::Error)
            }
        })
    }
}
