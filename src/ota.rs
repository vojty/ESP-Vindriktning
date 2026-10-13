use anyhow::Result;
use embedded_svc::http::server::Request;
use embedded_svc::http::Headers;
use embedded_svc::io::Write;
use esp_idf_svc::http::server::EspHttpConnection;
use esp_idf_svc::ota::{EspOta, Slot};
use log::*;
use serde::Serialize;

use crate::http::SendJson;
use crate::utils::sleep_ms;

const CHUNK_SIZE: usize = 4096;

/// Confirms that the running firmware works.
/// Until then, a firmware installed by OTA is on probation and the bootloader returns to the previous one on the next reboot.
pub fn mark_running_slot_valid() -> Result<()> {
    let mut ota = EspOta::new()?;
    let slot = ota.get_running_slot()?;
    info!("Running firmware: {:?}", SlotInfo::from(slot));
    if let Some(invalid) = ota.get_last_invalid_slot()? {
        warn!("Firmware in {} was rolled back", invalid.label);
    }
    ota.mark_running_slot_valid()?;
    Ok(())
}

#[derive(Serialize, Debug)]
struct SlotInfo {
    partition: String,
    version: Option<String>,
    /// Build date and time
    released: Option<String>,
}

impl From<Slot> for SlotInfo {
    fn from(slot: Slot) -> Self {
        Self {
            partition: slot.label.to_string(),
            version: slot.firmware.as_ref().map(|f| f.version.to_string()),
            released: slot.firmware.as_ref().map(|f| f.released.to_string()),
        }
    }
}

#[derive(Serialize)]
struct FirmwareInfo {
    running: SlotInfo,
    /// Partition of the last firmware that failed to confirm it works, `null` if there was none
    rolled_back: Option<String>,
}

/// `GET /firmware`
pub fn handle_info(req: Request<&mut EspHttpConnection>) -> Result<()> {
    let ota = EspOta::new()?;
    let info = FirmwareInfo {
        running: ota.get_running_slot()?.into(),
        rolled_back: ota
            .get_last_invalid_slot()?
            .map(|slot| slot.label.to_string()),
    };
    drop(ota);
    req.send_json(&info)?;
    Ok(())
}

/// `POST /ota`, the body is the firmware image (`make bin`). Restarts into the new firmware on success.
pub fn handle_update(mut req: Request<&mut EspHttpConnection>) -> Result<()> {
    let Some(length) = req.content_len() else {
        req.into_status_response(411)?
            .write_all(b"Content-Length is required")?;
        return Ok(());
    };

    // Only one instance can exist, e.g. a parallel upload
    let Ok(mut ota) = EspOta::new() else {
        req.into_status_response(409)?
            .write_all(b"Update already in progress")?;
        return Ok(());
    };

    info!("OTA update started, {} bytes", length);
    let result = (|| -> Result<()> {
        // Erases only what is needed, fails if the image doesn't fit the slot
        let mut update = ota.initiate_update_with_known_size(length as usize)?;
        let mut buffer = vec![0; CHUNK_SIZE];
        let mut received = 0;
        while received < length {
            let read = req.read(&mut buffer)?;
            if read == 0 {
                anyhow::bail!("Connection closed after {} of {} bytes", received, length);
            }
            // Checks the image header in the first chunk, an invalid image fails here
            update.write(&buffer[..read])?;
            received += read as u64;
        }
        // Validates the whole image and sets it as the boot partition, dropping `update` on error aborts it
        update.complete()?;
        Ok(())
    })();
    drop(ota);

    if let Err(e) = result {
        error!("OTA update failed: {:?}", e);
        req.into_status_response(400)?
            .write_all(format!("Update failed: {e}").as_bytes())?;
        return Ok(());
    }

    info!("OTA update installed, restarting");
    req.into_ok_response()?
        .write_all(b"Update installed, restarting")?;

    // Restart once the handler returns and the response is sent
    std::thread::spawn(|| {
        sleep_ms(1_000);
        esp_idf_svc::hal::reset::restart();
    });
    Ok(())
}
