use smart_leds_trait::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::{driver::color::LedPixelColorGrb24, LedPixelEsp32Rmt};

const LED_PIN: u32 = 25;

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

    pub fn brightness(&self, brightness: u8) -> Self {
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
    driver: LedPixelEsp32Rmt<RGB8, LedPixelColorGrb24>,
    brightness: Option<u8>,
}

impl Leds {
    pub fn new() -> Self {
        let driver = LedPixelEsp32Rmt::<RGB8, LedPixelColorGrb24>::new(0, LED_PIN).unwrap();
        Self {
            driver,
            colors: [Color::default(); 3],
            brightness: None,
        }
    }

    pub fn flush(&mut self) -> Result<(), ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriverError> {
        self.driver.write(self.colors.iter().cloned())
    }

    pub fn set_brightness(&mut self, brightness: u8) -> &mut Leds {
        self.brightness = Some(brightness);
        self.colors
            .iter_mut()
            .enumerate()
            .for_each(|(index, color)| match index {
                0 | 2 => {
                    *color = color.brightness(brightness);
                }
                _ => {
                    // keep the middle led untouched
                }
            });
        self
    }

    pub fn set_color(&mut self, position: LedPosition, mut color: Color) -> &mut Leds {
        if let Some(brightness) = self.brightness {
            if color.brightness.is_none() {
                color = color.brightness(brightness);
            }
        }

        self.colors[position as usize] = color;
        self
    }
}
