<div align="center">
<h1> ESP (Ikea) Vindriktning & Rust 🦀</h1>

Upgraded Ikea Vindriktning with ESP32

  <img height="300" src="./images/ikea-vindriktning.jpg"/>

&plus;

  <img height="300" src="./images/laskakit-esp-vindriktning.jpg"/>
</div>

### Basic features

- Air quality monitoring (PM2.5 + CO2)
- Smart LEDs for displaying results
- Simple HTTP server (over WiFi) for various stuff (_work in-progress_)

## Lifecycle

1. turn on the fan for 10 seconds to get fresh air
2. measure C02 & PM2.5
3. sleep for 50 seconds
4. repeat

## LEDs

Top LED shows CO2, bottom LED PM2.5, center LED a mix of both.

| Color | CO2 (ppm) | PM2.5 (µg/m³) |
|---|---|---|
| aqua | ≤ 400 | – |
| green | ≤ 1000 | ≤ 12 |
| yellow | ≤ 1500 | ≤ 35 |
| orange | ≤ 2000 | ≤ 55 |
| red | > 2000 | ≤ 150 |
| dark red | – | > 150 |

### Status and errors

**Blinking always means something needs attention.** Blue is used only for errors.

| LEDs | Meaning |
|---|---|
| all magenta | booting / connecting to WiFi |
| white | waiting for the first reading |
| top/bottom blinking in its color | 2–4 failed reads in a row, showing the last good value |
| top/bottom blinking blue | 5+ failed reads in a row (~5 min), sensor needs attention |
| center blinking | WiFi disconnected or clock never synced |
| all blinking blue | both sensors failed |

A single failed read is ignored. Blinking LEDs stay visible in night mode. `GET /data` also returns `co2_status` / `pm25_status` with `failures` (consecutive) and `health` (`waiting`, `ok`, `stale`, `failed`).

## REST API

TODO

## Components

- IKEA Vindriktning https://www.ikea.com/cz/cs/p/vindriktning-senzor-kvality-vzduchu-80515910/
- ESP32 board https://www.laskakit.cz/laskakit-esp-vindriktning-esp-32-i2c/
- SCD41 CO2 sensor https://www.laskakit.cz/laskakit-scd41-senzor-co2--teploty-a-vlhkosti-vzduchu/

## Development

1. Install `espup` (https://github.com/esp-rs/espup#installation)

```
cargo install espup
```

2. Install toolchains

```
espup install --esp-idf-version 4.4
```

3. Set up the environment variables running: `source ~/export-esp.sh`. **This step must be done every time you open a new terminal.**

4. To make this work with `rust-analyzer`, edit `.vscode/settings.json` like this:

```
  "rust-analyzer.server.extraEnv": {
    "LIBCLANG_PATH": "the same path as in ~/export-esp.sh, has to be absolute"
  },
```

## Flashing

```
make flash
```

## Notes

- binary larger than 1 MB won't flash without `partition.csv` file (1MB is probably a default value)

- `opt-level = "s"` is currenty broken in `rustc 1.65.0` (miscompilation issues)
