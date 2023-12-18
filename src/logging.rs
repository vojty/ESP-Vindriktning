use anyhow::{bail, Result};
use core::str;
use embedded_svc::{
    http::{client::Client, Method},
    io::{Read, Write},
};
use log::*;
use serde::Serialize;

use esp_idf_svc::http::client::{Configuration, EspHttpConnection};

const URL: &str = env!("LOG_URL");
const API_KEY: &str = env!("LOG_API_KEY");

#[derive(Serialize)]
pub struct LogEntry {
    co2: u16,
    pm25: u16,
}

impl LogEntry {
    pub fn new(co2: u16, pm25: u16) -> Self {
        Self { co2, pm25 }
    }
}

fn print_response(response: &mut impl Read) -> Result<()> {
    // https://github.com/esp-rs/esp-idf-svc/blob/master/examples/http_request.rs#L88
    let mut buf = [0_u8; 256];
    let mut offset = 0;
    let mut total = 0;

    loop {
        if let Ok(size) = Read::read(response, &mut buf[offset..]) {
            if size == 0 {
                break;
            }
            total += size;
            let size_plus_offset = size + offset;
            match str::from_utf8(&buf[..size_plus_offset]) {
                Ok(text) => {
                    info!("{}", text);
                    offset = 0;
                }
                Err(error) => {
                    let valid_up_to = error.valid_up_to();
                    unsafe {
                        error!("{}", str::from_utf8_unchecked(&buf[..valid_up_to]));
                    }
                    buf.copy_within(valid_up_to.., 0);
                    offset = size_plus_offset - valid_up_to;
                }
            }
        }
    }
    if total > 0 {
        info!("Received {} bytes", total);
    }
    Ok(())
}

pub fn log_data(log_entry: &LogEntry) -> Result<()> {
    // 1. Create a new EspHttpClient. (Check documentation)
    // ANCHOR: connection
    let connection = EspHttpConnection::new(&Configuration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    // ANCHOR_END: connection
    let mut client = Client::wrap(connection);

    // 2. Open a GET request to `url`
    let headers = [("content-type", "application/json"), ("apikey", API_KEY)];
    let mut request = client.request(Method::Post, URL.as_ref(), &headers)?;

    let payload = serde_json::to_string(&log_entry)?;
    request.write_all(payload.as_bytes())?;
    request.flush()?;

    let mut response = request.submit()?;
    let status = response.status();

    print_response(&mut response)?;

    if !(200..=299).contains(&status) {
        bail!("Unexpected response code: {}", status);
    }

    Ok(())
}
