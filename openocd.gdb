# GDB init script for Nucleo-F103RB + OpenOCD
# Used automatically by `cargo run` via .cargo/config.toml

# Connect to the OpenOCD GDB server (default port 3333)
target extended-remote :3333

# Flash the ELF binary onto the board
load

# Reset the MCU and halt it at the reset vector, then run
monitor reset halt
continue
