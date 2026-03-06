# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Board Support Package (BSP) crate for the STM32 Nucleo-F103RB board, written in Rust. It is a `no_std` embedded crate targeting the `thumbv7m-none-eabi` (ARMv7M Cortex-M3) architecture.

## Build Commands

The default build target is pre-configured as `thumbv7m-none-eabi` in `.cargo/config`. The target toolchain must be installed:

```sh
rustup target add thumbv7m-none-eabi
```

Build the library:
```sh
cargo build
```

Build a specific example:
```sh
cargo build --example gpio_hal_blinky
cargo build --example serial_hal_blocking_echo
```

Build in release mode (applies LTO and size optimization `opt-level = "s"`):
```sh
cargo build --release --example gpio_hal_blinky
```

There are no tests in this crate (embedded bare-metal, no test harness).

## Flashing Firmware

Requires OpenOCD and the ST-Link V2-1 (built into the board):

```sh
./openocd_program.sh target/thumbv7m-none-eabi/debug/examples/gpio_hal_blinky
```

This runs `openocd -f nucleo.cfg -c "program <elf> verify reset exit"`.

## Debugging

The runner in `.cargo/config` is `arm-none-eabi-gdb`. In a separate terminal, start OpenOCD:
```sh
openocd -f nucleo.cfg
```

Then run via cargo (launches GDB connected to OpenOCD):
```sh
cargo run --example gpio_hal_blinky
```

## Architecture

`src/lib.rs` is the entire library — it is a thin re-export layer:
- `stm32f1xx_hal` (with features `stm32f103`, `medium`, `rt`) is re-exported as `hal`
- Also re-exports `hal::pac`, `hal::prelude`, `cortex_m`, and `cortex_m_rt`

Examples consume this crate as `use nucleo_f103rb as board;` and then access peripherals via `board::hal::pac`, `board::hal::prelude::*`, etc.

**Key hardware mappings (from examples):**
- LED: `GPIOA` pin `PA5` (push-pull output)
- USART2 (connected to ST-Link for serial): TX=`PA2`, RX=`PA3`

**Memory layout** (`memory.x`): FLASH at `0x08000000` (64K), RAM at `0x20000000` (20K).

**Linker**: Uses `link.x` from `cortex-m-rt` via `-C link-arg=-Tlink.x` rustflag.

All examples must have `#![no_main]` and `#![no_std]` and use `#[entry]` from `cortex_m_rt`.
