use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

use scd4x::scd4x::Scd4x;

#[derive(Debug)]
pub struct Scd41<I2C, D> {
    sensor: Scd4x<I2C, D>,
}

impl<I2C, D, E> Scd41<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u32>,
{
    pub fn new(i2c: I2C, delay: D) -> Self {
        let sensor = Scd4x::new(i2c, delay);
        Self { sensor }
    }

    pub fn init(&mut self) -> Result<(), scd4x::error::Error<E>> {
        self.sensor.wake_up();
        self.sensor.stop_periodic_measurement()?;
        self.sensor.reinit()?;
        self.sensor.start_periodic_measurement()?;
        Ok(())
    }

    pub fn read_co2(&mut self) -> Result<u16, scd4x::error::Error<E>> {
        let m = self.sensor.measurement()?;
        Ok(m.co2)
    }
}
