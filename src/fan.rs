use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, LowSpeed};
use esp_idf_svc::sys::EspError;

/// Default fan speed in percent
pub const INITIAL_FAN_SPEED: u8 = 100;

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
        let duty = self.pwm.get_max_duty() * self.speed as u32 / 100;
        self.pwm.set_duty(duty)
    }

    pub fn disable(&mut self) -> Result<(), EspError> {
        self.pwm.set_duty(0)
    }
}
