use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use scd4x::Scd4x;

#[derive(Debug)]
pub struct Scd41<I2C, D> {
    sensor: Scd4x<I2C, D>,
}

impl<I2C, D, E> Scd41<I2C, D>
where
    I2C: I2c<Error = E>,
    D: DelayNs,
{
    pub fn new(i2c: I2C, delay: D) -> Self {
        let sensor = Scd4x::new(i2c, delay);
        Self { sensor }
    }

    pub fn init(&mut self) -> Result<(), scd4x::Error<E>> {
        self.sensor.wake_up();
        self.sensor.stop_periodic_measurement()?;
        self.sensor.reinit()?;
        self.sensor.start_periodic_measurement()?;
        Ok(())
    }

    pub fn read_co2(&mut self) -> Result<u16, scd4x::Error<E>> {
        let m = self.sensor.measurement()?;
        Ok(m.co2)
    }
}
