use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_svc::hal::peripheral;
use esp_idf_svc::sys::esp_wifi_set_max_tx_power;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    wifi::{BlockingWifi, EspWifi},
};
use log::*;

use crate::utils::sleep_ms;

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

/**
 * This is a workaround for a bug in the ESP-IDF wifi stack.
 * see https://github.com/esp-rs/esp-idf-svc/issues/304#issuecomment-1865823612
 */
pub trait WifiConnectFix {
    fn connect_with_retry(&mut self) -> anyhow::Result<()>;
}

impl WifiConnectFix for BlockingWifi<EspWifi<'_>> {
    fn connect_with_retry(&mut self) -> anyhow::Result<()> {
        let mut retry_delay_ms = 1_000;
        loop {
            info!("Connecting wifi...");
            match self.connect() {
                Ok(()) => break,
                Err(e) => {
                    warn!(
                        "Wifi connect failed, reason {}, retrying in {}s",
                        e,
                        retry_delay_ms / 1000
                    );
                    sleep_ms(retry_delay_ms);

                    // increase the delay exponentially, but cap it at 10s
                    retry_delay_ms = std::cmp::min(retry_delay_ms * 2, 10_000);

                    self.stop()?;
                    self.start()?;
                }
            }
        }

        info!("Waiting for DHCP lease...");

        self.wait_netif_up()?;
        Ok(())
    }
}

// https://github.com/ivmarkov/rust-esp32-std-demo/blob/main/src/main.rs#L1266
pub fn wifi(
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let esp_wifi = EspWifi::new(modem, sysloop.clone(), None)?;

    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    info!("Starting wifi...");

    wifi.start()?;

    // https://github.com/espressif/arduino-esp32/issues/6767
    // https://github.com/espressif/arduino-esp32/blob/0ef29868749865e754c62595e4b16e7da24608c0/libraries/WiFi/src/WiFiGeneric.cpp#L637
    unsafe {
        esp_wifi_set_max_tx_power(34);
    }

    info!("Scanning...");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == WIFI_SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            WIFI_SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            WIFI_SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASSWORD.try_into().unwrap(),
        channel,
        ..Default::default()
    }))?;

    wifi.start()?;

    wifi.connect_with_retry()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(wifi)
}
