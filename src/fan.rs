use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, LowSpeed};
use esp_idf_svc::sys::EspError;

use crate::utils::sleep_ms;

/// Default fan speed in percent
pub const INITIAL_FAN_SPEED: u8 = 60;

/// Time at full speed before slowing down, a stopped fan may not start at low speed
const KICKSTART_MS: u64 = 1000;

/// Fan driven by PWM, speed is set in percent (0-100)
pub struct Fan<'a> {
    pwm: LedcDriver<'a>,
    // Dropping the timer driver resets the timer, keep it alive with the channel
    _timer: LedcTimerDriver<'a, LowSpeed>,
    speed: u8,
}

impl<'a> Fan<'a> {
    pub fn new(mut pwm: LedcDriver<'a>, timer: LedcTimerDriver<'a, LowSpeed>) -> Self {
        pwm.set_duty(0).unwrap();
        Self {
            pwm,
            _timer: timer,
            speed: INITIAL_FAN_SPEED,
        }
    }

    /// Speed used by the next `enable`
    pub fn set_speed(&mut self, speed: u8) {
        self.speed = speed.min(100);
    }

    pub fn enable(&mut self) -> Result<(), EspError> {
        if self.speed == 0 {
            return self.pwm.set_duty(0);
        }
        let max_duty = self.pwm.get_max_duty();
        self.pwm.set_duty(max_duty)?;
        if self.speed < 100 {
            sleep_ms(KICKSTART_MS);
            self.pwm.set_duty(max_duty * self.speed as u32 / 100)?;
        }
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), EspError> {
        self.pwm.set_duty(0)
    }
}
