use log::*;

use crate::utils::sleep_ms;

/// Number of PM2.5 readings taken per measurement
pub const SAMPLE_COUNT: usize = 6;
/// Delay between readings, the sensor updates its value about once per second
pub const SAMPLE_INTERVAL_MS: u64 = 1_000;

/**
 * Takes `SAMPLE_COUNT` readings and returns their median, which drops occasional
 * outliers (sporadic spikes) instead of averaging them in.
 * Failed readings are skipped, the error of the last one is returned only if all fail.
 */
pub fn read_median<E: std::fmt::Display>(
    mut read: impl FnMut() -> Result<u16, E>,
) -> Result<u16, E> {
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut last_error = None;

    for i in 0..SAMPLE_COUNT {
        if i > 0 {
            sleep_ms(SAMPLE_INTERVAL_MS);
        }
        match read() {
            Ok(value) => samples.push(value),
            Err(e) => {
                warn!("PM2.5 sample {} failed: {}", i + 1, e);
                last_error = Some(e);
            }
        }
    }

    match (median(&mut samples), last_error) {
        (Some(value), _) => {
            info!("PM2.5 samples: {:?}, median: {}", samples, value);
            Ok(value)
        }
        (None, Some(e)) => Err(e),
        (None, None) => unreachable!("SAMPLE_COUNT must be > 0"),
    }
}

/// Median of the values (mean of the two middle ones for an even count), `None` if empty
fn median(values: &mut [u16]) -> Option<u16> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some(((values[mid - 1] as u32 + values[mid] as u32 + 1) / 2) as u16)
    } else {
        Some(values[mid])
    }
}
