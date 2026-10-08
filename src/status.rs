use serde::Serialize;

/// Consecutive failed reads after which the last value is considered stale
const STALE_AFTER: u32 = 2;
/// Consecutive failed reads after which the sensor is considered broken
const FAILED_AFTER: u32 = 5;

#[derive(Serialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Health {
    /// No successful read yet
    #[default]
    Waiting,
    Ok,
    /// A few reads failed, showing the last good value
    Stale,
    /// Too many reads failed (or never got a value), needs attention
    Failed,
}

#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct SensorStatus {
    /// Consecutive failed reads
    pub failures: u32,
    pub health: Health,
    #[serde(skip)]
    has_value: bool,
}

impl SensorStatus {
    pub fn update(&mut self, success: bool) {
        if success {
            self.failures = 0;
            self.has_value = true;
        } else {
            self.failures += 1;
        }

        self.health = match (self.failures, self.has_value) {
            (n, _) if n >= FAILED_AFTER => Health::Failed,
            (_, false) => Health::Waiting,
            (n, true) if n >= STALE_AFTER => Health::Stale,
            _ => Health::Ok,
        };
    }
}
