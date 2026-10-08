use anyhow::*;
use embedded_svc::http::Method;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::timer::EspTaskTimerService;
use log::*;
use serde::Serialize;
use std::collections::VecDeque;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use time::OffsetDateTime;

use board::Board;
use clock::Clock;
use http::BodyParser;
use http::SendJson;
use leds::Leds;
use leds::INITIAL_BRIGHTNESS;
use status::SensorStatus;
use utils::sleep_ms;
use wifi::WifiConnectFix;

mod board;
mod clock;
mod fan;
mod http;
mod leds;
mod logging;
mod pm_uart;
mod scd41;
mod status;
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

    server.fn_handler::<anyhow::Error, _>("/brightness", Method::Put, move |mut req| {
        let brightness: u8 = req.parse_body().unwrap();
        state.write().unwrap().settings.brightness = brightness;
        info!("Brightness set to {}", brightness);
        let mut leds = leds.write().unwrap();
        leds.set_brightness(brightness).flush().unwrap();

        req.into_ok_response().unwrap();
        Ok(())
    })?;

    server.fn_handler::<anyhow::Error, _>("/restart", Method::Post, |_req| {
        // panic will cause a restart of the device
        panic!("User requested a restart!")
    })?;
    Ok(server)
}

const HISTORY_LENGTH: usize = 5;

/// A single measurement, `None` (null in JSON) when the sensor read failed
#[derive(Serialize, Clone, Copy)]
struct Measurement {
    co2: Option<u16>,
    pm25: Option<u16>,
    #[serde(with = "time::serde::rfc3339::option")]
    timestamp: Option<OffsetDateTime>,
}

#[derive(Serialize, Default)]
struct MeasuredData {
    // Last successfully read values
    co2: u16,
    pm25: u16,
    #[serde(with = "time::serde::rfc3339::option")]
    timestamp: Option<OffsetDateTime>,
    // Last HISTORY_LENGTH measurements, oldest first
    history: VecDeque<Measurement>,
    co2_status: SensorStatus,
    pm25_status: SensorStatus,
}

impl MeasuredData {
    fn add_measurement(&mut self, measurement: Measurement) {
        self.co2_status.update(measurement.co2.is_some());
        self.pm25_status.update(measurement.pm25.is_some());
        if let Some(co2) = measurement.co2 {
            self.co2 = co2;
        }
        if let Some(pm25) = measurement.pm25 {
            self.pm25 = pm25;
        }
        if measurement.co2.is_some() || measurement.pm25.is_some() {
            self.timestamp = measurement.timestamp;
        }

        if self.history.len() >= HISTORY_LENGTH {
            self.history.pop_front();
        }
        self.history.push_back(measurement);
    }
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

const CLOCK_SYNC_INTERVAL: Duration = Duration::from_secs(60 * 60);

fn set_brightness(leds: &Arc<RwLock<Leds>>, clock: &Arc<Mutex<Clock>>) {
    // Clock was never synced, keep the current brightness
    let Some(datetime) = clock.lock().unwrap().get_datetime() else {
        return;
    };

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
        rmt,
        ..
    } = peripherals;

    // Create board
    let mut board = Board::new(pins, i2c1, uart1, rmt);
    board.init();

    // Init color
    board.leds.set_initial_color();

    // Setup wifi
    // Reconnecting is handled in the main loop, see `ensure_connected`
    let mut blocking_wifi = wifi::wifi(modem, sysloop.clone())?;

    // Wait for data
    board.leds.set_waiting_color();

    // NTP client
    let clock = Arc::new(Mutex::new(Clock::new()));
    clock.lock().unwrap().sync();
    // Sync clock every hour, retry every minute if the last sync failed
    let clock_sync_timer = EspTaskTimerService::new()?.timer({
        let clock = clock.clone();
        move || {
            let mut clock = clock.lock().unwrap();
            if clock.needs_sync(CLOCK_SYNC_INTERVAL) {
                clock.sync();
            }
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

    // Blink LEDs signalling a problem
    let blink_timer = EspTaskTimerService::new()?.timer({
        let leds = leds.clone();
        move || leds.write().unwrap().toggle_blink()
    })?;
    blink_timer.every(Duration::from_secs(1))?;

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

    if !logging::is_enabled() {
        info!("LOG_URL or LOG_API_KEY not set, data logging disabled");
    }

    loop {
        // Reconnect wifi if the connection was lost
        let wifi_ok = match blocking_wifi.ensure_connected() {
            Ok(reconnected) => {
                if reconnected {
                    clock.lock().unwrap().sync();
                }
                true
            }
            Err(e) => {
                error!("Wifi reconnect failed: {:?}", e);
                false
            }
        };

        // Get fresh air
        board.fan.enable().unwrap();
        sleep_ms(10_000);
        board.fan.disable().unwrap();

        // Read data
        // On error keep the previous value instead of reporting 0
        let co2 = board
            .scd41
            .read_co2()
            .map_err(|e| error!("Error reading CO2: {:?}", e))
            .ok();
        let pm25 = board
            .pm1006
            .read_pm25()
            .map_err(|e| error!("Error reading PM2.5: {}", e))
            .ok();
        info!("CO2: {:?} ppm, PM2.5: {:?} ug/m3", co2, pm25);

        // Store data
        let timestamp = clock.lock().unwrap().get_unix_timestamp();
        let (last_co2, last_pm25) = {
            let data = &mut state.write().unwrap().measured_data;
            data.add_measurement(Measurement {
                co2,
                pm25,
                timestamp: timestamp.and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok()),
            });
            (
                (data.co2, data.co2_status.health),
                (data.pm25, data.pm25_status.health),
            )
        };

        // Update LEDs
        // Clock that was never synced breaks timestamps and night mode
        let system_ok = wifi_ok && timestamp.is_some();
        leds.write()
            .unwrap()
            .visualize_measures(last_co2, last_pm25, system_ok);

        // Log data, only complete measurements
        if let (true, Some(co2), Some(pm25)) = (logging::is_enabled(), co2, pm25) {
            match logging::log_data(&logging::LogEntry::new(co2, pm25)) {
                Ok(_) => info!("Data logged successfully"),
                Err(e) => error!("Error logging data: {}", e),
            }
        }

        sleep_ms(50_000);
    }
}
