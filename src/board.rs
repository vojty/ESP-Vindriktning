use esp_idf_svc::hal::{
    delay, gpio, gpio::Pins, i2c::I2cConfig, i2c::I2cDriver, i2c::I2C1, ledc::config::TimerConfig,
    ledc::LedcDriver, ledc::LedcTimerDriver, ledc::CHANNEL0, ledc::TIMER0, uart::UartConfig,
    uart::UartDriver, uart::UART1, units::Hertz, units::KiloHertz,
};
use pm1006::pm1006::Pm1006;

use crate::fan::Fan;
use crate::leds::Leds;
use crate::pm_uart::PmUart;
use crate::scd41::Scd41;

pub struct Board {
    pub scd41: Scd41<I2cDriver<'static>, delay::FreeRtos>,
    pub pm1006: Pm1006<PmUart>,
    pub leds: Leds,
    pub fan: Fan<'static>,
}

impl Board {
    pub fn new(
        pins: Pins,
        i2c1: I2C1<'static>,
        uart1: UART1<'static>,
        fan_timer: TIMER0<'static>,
        fan_channel: CHANNEL0<'static>,
    ) -> Self {
        // Fan, PWM frequency above the audible range to avoid coil whine
        let config = TimerConfig::new().frequency(KiloHertz(25).into());
        let timer = LedcTimerDriver::new(fan_timer, &config).unwrap();
        let pwm = LedcDriver::new(fan_channel, &timer, pins.gpio12).unwrap();
        let fan = Fan::new(pwm, timer);

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
            Option::<gpio::Gpio0<'static>>::None,
            Option::<gpio::Gpio1<'static>>::None,
            &config,
        )
        .unwrap();

        // PmUart clears the RX buffer before each request and adds a read timeout
        let pm1006 = Pm1006::new(PmUart::new(uart_driver));

        // LEDs
        let led_pin = pins.gpio25;
        let mut leds = Leds::new(led_pin);
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
