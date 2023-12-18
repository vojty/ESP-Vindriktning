use anyhow::*;
use embedded_svc::http::Method;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::http::server::EspHttpServer;
use http::SendJson;
use log::*;
use serde::Serialize;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::RwLock;

use board::Board;
use leds::{Color, LedPosition, Leds};
use utils::{get_co2_color, get_pm25_color, sleep_ms};

use crate::clock::Clock;
use crate::http::BodyParser;

mod board;
mod clock;
mod fan;
mod http;
mod leds;
mod logging;
mod pm1006;
mod scd41;
mod utils;
mod wifi;

fn httpd(state: Arc<RwLock<State>>, leds: Arc<RwLock<Leds>>) -> Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(&Default::default())?;

    server.fn_handler("/data", Method::Get, {
        let state = state.clone();
        move |req| {
            let data = &state.read().unwrap().measured_data;
            req.send_json(data)
        }
    })?;

    server.fn_handler("/brightness", Method::Put, move |mut req| {
        let brightness: u8 = req.parse_body().unwrap();
        state.write().unwrap().settings.brightness = brightness;
        info!("Brightness set to {}", brightness);
        let mut leds = leds.write().unwrap();
        leds.set_brightness(brightness).flush().unwrap();

        req.into_ok_response().unwrap();
        Ok(())
    })?;

    server.fn_handler("/restart", Method::Post, |_req| {
        // panic will cause a restart of the device
        panic!("User requested a restart!")
    })?;
    Ok(server)
}

#[derive(Serialize, Default)]
struct MeasuredData {
    co2: u16,
    pm25: u16,
    timestamp: Option<i64>,
}

#[derive(Serialize)]
struct Settings {
    brightness: u8,
}

#[derive(Serialize)]
struct State {
    measured_data: MeasuredData,
    settings: Settings,
}

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let Peripherals {
        modem,
        pins,
        i2c1,
        uart1,
        ..
    } = peripherals;

    // Create board
    let mut board = Board::new(pins, i2c1, uart1);
    board.init();

    let initial_brightness = 20;
    // Set initial color
    let initial_color = Color::new(255, 0, 255).brightness(initial_brightness); // Fuchsia / Magenta / Violet
    board
        .leds
        .set_color(LedPosition::Top, initial_color)
        .set_color(LedPosition::Bottom, initial_color)
        .set_color(LedPosition::Center, initial_color)
        .flush()
        .unwrap();

    // Setup wifi
    let _wifi = wifi::wifi(modem, sysloop)?;

    let waiting_color = Color::new(0, 255, 255).brightness(initial_brightness); // cyan
    board
        .leds
        .set_color(LedPosition::Top, waiting_color)
        .set_color(LedPosition::Bottom, waiting_color)
        .set_color(LedPosition::Center, waiting_color)
        .flush()
        .unwrap();

    // NTP client
    let clock = Clock::new();

    let state = State {
        measured_data: MeasuredData::default(),
        settings: Settings {
            brightness: initial_brightness,
        },
    };
    let state = Arc::new(RwLock::new(state));
    let leds = Arc::new(RwLock::new(board.leds));
    let _server = httpd(state.clone(), leds.clone())?;

    loop {
        // Get fresh air
        board.fan.enable().unwrap();
        sleep_ms(10_000);
        board.fan.disable().unwrap();

        // Read data
        let co2 = board.scd41.read_co2().unwrap();
        let pm25 = board.pm1006.read_pm25().unwrap();
        info!("CO2: {} ppm, PM2.5: {} ug/m3", co2, pm25);

        // Store data
        state.write().unwrap().measured_data.co2 = co2;
        state.write().unwrap().measured_data.pm25 = pm25;
        state.write().unwrap().measured_data.timestamp = clock.get_timestamp().ok();

        let co2_color = get_co2_color(co2);
        let pm25_color = get_pm25_color(pm25);
        let brightness = state.read().unwrap().settings.brightness;

        // Update LEDs
        let mut leds = leds.write().unwrap();
        leds.set_color(LedPosition::Bottom, pm25_color.brightness(brightness))
            .set_color(
                LedPosition::Center,
                pm25_color.mix(&co2_color).brightness(10),
            )
            .set_color(LedPosition::Top, co2_color.brightness(brightness))
            .flush()
            .unwrap();

        // Log data
        match logging::log_data(&logging::LogEntry::new(co2, pm25)) {
            Ok(_) => info!("Data logged successfully"),
            Err(e) => error!("Error logging data: {}", e),
        }

        sleep_ms(50_000);
    }
}
