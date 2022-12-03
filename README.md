<div align="center">
<h1> ESP (Ikea) Vindriktning & Rust 🦀</h1>

Upgraded Ikea Vindriktning with ESP32

  <img height="300" src="./images/ikea-vindriktning.jpg"/>

&plus;

  <img height="300" src="./images/laskakit-esp-vindriktning.jpg"/>
</div>

### Basic features

- Air quality measurement (PM2.5 + CO2)
- Smart LEDs for displaying results
- Simple HTTP server (using WiFI) for various stuff (_work in-progress_)

## Lifecycle

1. turn on the fan for 10 seconds to get fresh air
2. measure C02 & PM2.5
3. sleep for 50 seconds
4. repeat

## REST API

TODO

## Components

- IKEA Vindriktning https://www.ikea.com/cz/cs/p/vindriktning-senzor-kvality-vzduchu-80515910/
- ESP32 board https://www.laskakit.cz/laskakit-esp-vindriktning-esp-32-i2c/
- SCD41 CO2 sensor https://www.laskakit.cz/laskakit-scd41-senzor-co2--teploty-a-vlhkosti-vzduchu/

## Notes

- needs custom fork of Rust to support ESP chips <s>https://github.com/esp-rs/rust-build</s> now via https://github.com/esp-rs/espup
- <s>to update Rust ESP version, run `./install-rust-toolchain.sh --toolchain-version 1.xx.0.0`</s> now via `espup`
- `cargo build` needs `source export-esp.sh` first
- to make this work with `rust-analyzer`, add this to your config:

  ```json
    "rust-analyzer.server.extraEnv": {
      "LIBCLANG_PATH": "/Users/tomas.vojtasek/.espressif/tools/xtensa-esp32-elf-clang/esp-14.0.0-20220415-aarch64-apple-darwin/lib/"
    },
  ```

  The path has to be absolute, won't work with the leading `~/`

- binary larger than 1 MB won't flash without `partition.csv` file (1MB is probably a default value)

- `opt-level = "s"` is currenty broken in `rustc 1.65.0` (miscompilation issues)
