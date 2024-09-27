use esp_idf_svc::hal::{
    delay, gpio, gpio::PinDriver, gpio::Pins, i2c::I2cConfig, i2c::I2cDriver, i2c::I2C1, rmt::RMT,
    uart::UartConfig, uart::UartDriver, uart::UART1, units::Hertz, units::KiloHertz,
};
use pm1006::pm1006::Pm1006;

use crate::fan::Fan;
use crate::leds::Leds;
use crate::scd41::Scd41;

pub struct Board {
    pub scd41: Scd41<I2cDriver<'static>, delay::FreeRtos>,
    pub pm1006: Pm1006<UartDriver<'static>>,
    pub leds: Leds,
    pub fan: Fan<'static, gpio::Gpio12>,
}

impl Board {
    pub fn new(pins: Pins, i2c1: I2C1, uart1: UART1, rmt: RMT) -> Self {
        // Fan
        let fan_pin = PinDriver::output(pins.gpio12).unwrap();
        let fan = Fan::new(fan_pin);

        // SCD41
        let config = I2cConfig::default().baudrate(KiloHertz(100).into());
        let i2c = I2cDriver::new(i2c1, pins.gpio21, pins.gpio22, &config).unwrap();
        let delay = delay::FreeRtos {};
        let scd41 = Scd41::new(i2c, delay);

        // PM1006
        let config = UartConfig::new().baudrate(Hertz(9_600));
        let uart_driver = UartDriver::new(
            uart1,
            pins.gpio17,
            pins.gpio16,
            Option::<gpio::Gpio0>::None,
            Option::<gpio::Gpio1>::None,
            &config,
        )
        .unwrap();

        // Clear RX buffer to avoid reading old data
        match uart_driver.clear_rx() {
            Ok(_) => log::info!("Cleared RX buffer"),
            Err(e) => log::warn!("Failed to clear RX buffer: {}", e),
        }

        let pm1006 = Pm1006::new(uart_driver);

        // LEDs
        let led_pin = pins.gpio25;
        let led_channel = rmt.channel0;
        let mut leds = Leds::new(led_channel, led_pin);
        leds.set_brightness(20);

        Self {
            pm1006,
            scd41,
            leds,
            fan,
        }
    }

    pub fn init(&mut self) {
        self.scd41.init().unwrap();
    }
}
