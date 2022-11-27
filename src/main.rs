use anyhow::*;
use board::Board;
use embedded_svc::httpd::registry::*;
use embedded_svc::httpd::Response;

use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::httpd as idf;

use esp_idf_sys as _;
use leds::{Color, LedPosition, Leds};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log::*;
use serde::Serialize;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::RwLock;
use utils::{get_co2_color, get_pm25_color, sleep_ms};

mod board;
mod fan;
mod leds;
mod pm1006;
mod scd41;
mod utils;
mod wifi;

fn httpd(data: Arc<RwLock<MeasuredData>>, leds: Arc<RwLock<Leds>>) -> Result<idf::Server> {
    let server = idf::ServerRegistry::new()
        .at("/")
        .get({
            let data = data.clone();
            move |_| {
                let data = data.read().unwrap();
                let json = serde_json::to_string(&*data).unwrap();
                let response = Response::new(200)
                    .content_type("application/json")
                    .body(json.into());
                Ok(response)
            }
        })?
        .at("/brightness")
        .put(move |mut req| {
            let body = req.as_string().unwrap();
            let brightness: u8 = serde_json::from_str(&body).unwrap();

            let mut data = data.write().unwrap();
            data.brightness = brightness;
            info!("Brightness set to {}", brightness);

            leds.write().unwrap().set_brightness(brightness);
            leds.write().unwrap().flush();
            Ok(Response::new(200))
        })?
        .at("/restart")
        .get(|_| {
            // this will cause a restart
            panic!("User requested a restart!")
        })?;

    server.start(&Default::default())
}

#[derive(Serialize, Default)]
struct MeasuredData {
    co2: u16,
    pm25: u16,
    brightness: u8,
}

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

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

    let _wifi = wifi::wifi(modem, sysloop)?;

    let app_data = MeasuredData {
        brightness: 20,
        ..Default::default()
    };
    let app_data = Arc::new(RwLock::new(app_data));
    // Init components
    let mut board = Board::new(pins, i2c1, uart1);
    let leds = Arc::new(RwLock::new(board.leds));

    let default_color = Color::new(255, 0, 255).brightness(255); // magenta

    leds.write()
        .unwrap()
        .set_color(LedPosition::Top, default_color);
    leds.write()
        .unwrap()
        .set_color(LedPosition::Bottom, default_color);
    leds.write()
        .unwrap()
        .set_color(LedPosition::Center, default_color);
    leds.write().unwrap().flush();

    let _server = httpd(app_data.clone(), leds.clone())?;

    board.scd41.init();

    loop {
        // Get fresh air
        board.fan.enable();
        sleep_ms(10_000);
        board.fan.disable();

        // Read data
        let co2 = board.scd41.read_co2().unwrap();
        let pm25 = board.pm1006.read_pm2().unwrap();

        // Store data
        app_data.write().unwrap().co2 = co2;
        app_data.write().unwrap().pm25 = pm25;

        let co2_color = get_co2_color(co2);
        let pm25_color = get_pm25_color(pm25);
        let brightness = app_data.read().unwrap().brightness;

        // Update LEDs
        leds.write()
            .unwrap()
            .set_color(LedPosition::Bottom, pm25_color.brightness(brightness));
        leds.write().unwrap().set_color(
            LedPosition::Center,
            pm25_color.mix(&co2_color).brightness(10),
        );
        leds.write()
            .unwrap()
            .set_color(LedPosition::Top, co2_color.brightness(brightness));
        leds.write().unwrap().flush();
        info!("CO2: {} ppm, PM2.5: {} µg/m3", co2, pm25);

        sleep_ms(50_000);
    }
}
