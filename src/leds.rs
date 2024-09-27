use esp_idf_svc::hal::{gpio::OutputPin, peripheral::Peripheral, rmt::RmtChannel};
use smart_leds_trait::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::{driver::color::LedPixelColorGrb24, LedPixelEsp32Rmt};

use crate::utils::{get_co2_color, get_pm25_color};

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub brightness: Option<u8>,
}

impl From<Color> for RGB8 {
    fn from(color: Color) -> Self {
        let brightness = color.brightness.unwrap_or(255);
        Self {
            r: ((color.r as u16) * (brightness as u16 + 1) / 256) as u8,
            g: ((color.g as u16) * (brightness as u16 + 1) / 256) as u8,
            b: ((color.b as u16) * (brightness as u16 + 1) / 256) as u8,
        }
    }
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            brightness: None,
        }
    }

    fn brightness(&self, brightness: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            brightness: Some(brightness),
        }
    }

    pub fn mix(&self, other: &Self) -> Self {
        Self {
            r: ((self.r as u16 + other.r as u16) / 2) as u8,
            g: ((self.g as u16 + other.g as u16) / 2) as u8,
            b: ((self.b as u16 + other.b as u16) / 2) as u8,
            brightness: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum LedPosition {
    Bottom = 0,
    Center = 1,
    Top = 2,
}

pub struct Leds {
    colors: [Color; 3],
    driver: LedPixelEsp32Rmt<'static, RGB8, LedPixelColorGrb24>,
    brightness: u8,
}

pub const INITIAL_BRIGHTNESS: u8 = 20;

impl Leds {
    pub fn new<C: RmtChannel>(
        channel: impl Peripheral<P = C> + 'static,
        pin: impl Peripheral<P = impl OutputPin> + 'static,
    ) -> Self {
        let driver = LedPixelEsp32Rmt::<RGB8, LedPixelColorGrb24>::new(channel, pin).unwrap();
        Self {
            driver,
            colors: [Color::default(); 3],
            brightness: INITIAL_BRIGHTNESS,
        }
    }

    pub fn flush(&mut self) -> Result<(), ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriverError> {
        self.driver.write(self.colors.iter().cloned())
    }

    pub fn set_brightness(&mut self, brightness: u8) -> &mut Leds {
        self.brightness = brightness;
        self.colors.iter_mut().for_each(|color| {
            *color = color.brightness(brightness);
        });
        self
    }

    pub fn set_color(&mut self, position: LedPosition, mut color: Color) -> &mut Leds {
        color = color.brightness(self.brightness);

        self.colors[position as usize] = color;
        self
    }

    pub fn get_brightness(&self) -> u8 {
        self.brightness
    }
}

impl Leds {
    pub fn set_initial_color(&mut self) {
        let initial_color = Color::new(255, 0, 255); // Fuchsia / Magenta / Violet
        self.set_color(LedPosition::Top, initial_color)
            .set_color(LedPosition::Bottom, initial_color)
            .set_color(LedPosition::Center, initial_color)
            .flush()
            .unwrap();
    }

    pub fn set_waiting_color(&mut self) {
        let waiting_color = Color::new(0, 255, 255); // cyan
        self.set_color(LedPosition::Top, waiting_color)
            .set_color(LedPosition::Bottom, waiting_color)
            .set_color(LedPosition::Center, waiting_color)
            .flush()
            .unwrap();
    }

    pub fn visualize_measures(&mut self, co2: u16, pm25: u16) {
        let co2_color = get_co2_color(co2);
        let pm25_color = get_pm25_color(pm25);
        self.set_color(LedPosition::Top, co2_color)
            .set_color(LedPosition::Center, pm25_color.mix(&co2_color))
            .set_color(LedPosition::Bottom, pm25_color)
            .flush()
            .unwrap();
    }
}
