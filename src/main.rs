use anyhow::*;
use embedded_svc::http::Method;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::WifiEvent;
use log::*;
use serde::Serialize;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;

use board::Board;
use clock::Clock;
use http::BodyParser;
use http::SendJson;
use leds::Leds;
use leds::INITIAL_BRIGHTNESS;
use utils::sleep_ms;
use wifi::WifiConnectFix;

mod board;
mod clock;
mod fan;
mod http;
mod leds;
mod logging;
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

fn set_brightness(leds: &Arc<RwLock<Leds>>, clock: &Arc<Mutex<Clock>>) {
    let datetime = clock.lock().unwrap().get_datetime().unwrap();

    let new_brightness = if datetime.hour() >= 22 || datetime.hour() < 6 {
        1
    } else {
        INITIAL_BRIGHTNESS
    };

    if new_brightness != leds.read().unwrap().get_brightness() {
        info!("Setting brightness to {}", new_brightness);
        leds.write()
            .unwrap()
            .set_brightness(new_brightness)
            .flush()
            .unwrap();
    }
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

    // Init color
    board.leds.set_initial_color();

    // Setup wifi
    let mut blocking_wifi = wifi::wifi(modem, sysloop.clone())?;
    // Try to reconnect if we get disconnected
    let _sub = sysloop.subscribe(move |parsed_event: &WifiEvent| {
        if *parsed_event == WifiEvent::StaDisconnected {
            blocking_wifi.connect_with_retry().unwrap();
        }
    })?;

    // Wait for data
    board.leds.set_waiting_color();

    // NTP client
    let clock = Arc::new(Mutex::new(Clock::new()));
    clock.lock().unwrap().sync();
    // Sync clock every minute
    let clock_sync_timer = EspTaskTimerService::new()?.timer({
        let clock = clock.clone();
        move || {
            clock.lock().unwrap().sync();
        }
    })?;
    clock_sync_timer.every(Duration::from_secs(60))?;

    let state = State {
        measured_data: MeasuredData::default(),
        settings: Settings {
            brightness: INITIAL_BRIGHTNESS,
        },
    };
    let state = Arc::new(RwLock::new(state));
    let leds = Arc::new(RwLock::new(board.leds));
    let _server = httpd(state.clone(), leds.clone())?;

    // Schedule timer for night mode
    set_brightness(&leds, &clock);
    let night_mode_timer = EspTaskTimerService::new()?.timer({
        let leds = leds.clone();
        let clock = clock.clone();
        move || {
            set_brightness(&leds, &clock);
        }
    })?;
    night_mode_timer.every(Duration::from_secs(60))?;

    loop {
        // Get fresh air
        board.fan.enable().unwrap();
        sleep_ms(10_000);
        board.fan.disable().unwrap();

        // Read data
        let co2 = board.scd41.read_co2().unwrap_or_else(|e| {
            error!("Error reading CO2: {:?}", e);
            0
        });
        let pm25 = board.pm1006.read_pm25().unwrap_or_else(|e| {
            error!("Error reading PM2.5: {:?}", e);
            0
        });
        info!("CO2: {} ppm, PM2.5: {} ug/m3", co2, pm25);

        // Store data
        state.write().unwrap().measured_data.co2 = co2;
        state.write().unwrap().measured_data.pm25 = pm25;
        state.write().unwrap().measured_data.timestamp = clock.lock().unwrap().get_unix_timestamp();

        // Update LEDs
        leds.write().unwrap().visualize_measures(co2, pm25);

        // Log data
        match logging::log_data(&logging::LogEntry::new(co2, pm25)) {
            Ok(_) => info!("Data logged successfully"),
            Err(e) => error!("Error logging data: {}", e),
        }

        sleep_ms(50_000);
    }
}
