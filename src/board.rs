use esp_idf_hal::delay;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::gpio::Pins;
use esp_idf_hal::i2c::I2cConfig;
use esp_idf_hal::i2c::I2cDriver;
use esp_idf_hal::i2c::I2C1;
use esp_idf_hal::uart::UartConfig;
use esp_idf_hal::uart::UartDriver;
use esp_idf_hal::uart::UART1;
use esp_idf_hal::units::Hertz;
use esp_idf_hal::units::KiloHertz;

use crate::fan::Fan;
use crate::leds::Leds;
use crate::pm1006::Pm1006;
use crate::scd41::Scd41;

pub struct Board<'a> {
    pub scd41: Scd41<I2cDriver<'a>, delay::FreeRtos>,
    pub pm1006: Pm1006<'a>,
    pub leds: Leds,
    pub fan: Fan<'a, gpio::Gpio12>,
}

impl Board<'_> {
    pub fn new(pins: Pins, i2c1: I2C1, uart1: UART1) -> Self {
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

        let pm1006 = Pm1006::new(uart_driver);

        // LEDs
        let mut leds = Leds::new();
        leds.set_brightness(20);

        Self {
            pm1006,
            scd41,
            leds,
            fan,
        }
    }
}
