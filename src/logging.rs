use anyhow::{bail, Result};
use core::str;
use embedded_svc::{
    http::{client::Client, Method},
    io::{Read, Write},
};
use log::*;
use serde::Serialize;

use esp_idf_svc::http::client::{Configuration, EspHttpConnection};

// Optional - when either one is missing, data logging is disabled
const URL: Option<&str> = option_env!("LOG_URL");
const API_KEY: Option<&str> = option_env!("LOG_API_KEY");

fn config() -> Option<(&'static str, &'static str)> {
    match (URL, API_KEY) {
        (Some(url), Some(api_key)) if !url.is_empty() && !api_key.is_empty() => {
            Some((url, api_key))
        }
        _ => None,
    }
}

pub fn is_enabled() -> bool {
    config().is_some()
}

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
    let Some((url, api_key)) = config() else {
        return Ok(());
    };

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
    let headers = [("content-type", "application/json"), ("apikey", api_key)];
    let mut request = client.request(Method::Post, url, &headers)?;

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
