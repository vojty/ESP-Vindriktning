use log::*;
use sntp_request::SntpRequest;
use std::time::Instant;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt, Tz};

pub struct Clock {
    sntp: SntpRequest,
    last_update: Option<Instant>,
    timestamp: i64,
    timezone: &'static Tz,
}

const TIMEZONE: &str = env!("TIMEZONE");

impl Clock {
    pub fn new() -> Self {
        let sntp = SntpRequest::new();
        let timezone = timezones::get_by_name(TIMEZONE).unwrap_or(timezones::db::GMT);
        info!("Timezone: {:?}", timezone);
        Self {
            timezone,
            timestamp: 0,
            sntp,
            last_update: None,
        }
    }

    pub fn sync(&mut self) {
        // sync with remote server
        let result = self.sntp.get_unix_time(); // in seconds
        self.last_update = Some(Instant::now());

        // update the local timestamp
        match result {
            Ok(timestamp) => {
                info!("Sync successful, timestamp: {}", timestamp);
                self.timestamp = timestamp;
            }
            Err(e) => {
                error!("Sync failed: {:?}", e);
            }
        }
    }

    pub fn get_unix_timestamp(&self) -> Option<i64> {
        self.last_update.map(|last_update| {
            let now = Instant::now();
            let duration = now.duration_since(last_update);

            self.timestamp + duration.as_secs() as i64
        })
    }

    pub fn get_datetime(&self) -> Option<OffsetDateTime> {
        if let Some(timestamp) = self.get_unix_timestamp() {
            let datetime = OffsetDateTime::from_unix_timestamp(timestamp);
            match datetime {
                Ok(datetime) => Some(datetime.to_timezone(self.timezone)),
                Err(e) => {
                    error!(
                        "Failed to convert timestamp {} to datetime: {}",
                        timestamp, e
                    );
                    None
                }
            }
        } else {
            None
        }
    }
}
