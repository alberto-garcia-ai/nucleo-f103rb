#![no_std]
#![allow(non_camel_case_types)]

pub use stm32f1xx_hal as hal;

pub use hal::pac;
pub use hal::prelude;
pub use cortex_m;
pub use cortex_m_rt;

pub mod console;

/// Print formatted text to the UART2 console (no newline).
/// Requires [`console::init`] to have been called first.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut c = $crate::console::Console;
        let _ = write!(c, $($arg)*);
    }};
}

/// Print formatted text to the UART2 console followed by `\r\n`.
/// Requires [`console::init`] to have been called first.
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\r\n") };
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut c = $crate::console::Console;
        let _ = write!(c, $($arg)*);
        let _ = write!(c, "\r\n");
    }};
}
