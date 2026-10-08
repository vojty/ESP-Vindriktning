use esp_idf_svc::hal::{gpio::OutputPin, rmt::RmtChannel};
use smart_leds_trait::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::{driver::color::LedPixelColorGrb24, LedPixelEsp32Rmt};

use crate::status::Health;
use crate::utils::{get_co2_color, get_pm25_color, BLUE, WHITE};

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
    // LEDs that blink to signal a problem
    blink: [bool; 3],
    // Current blink phase, toggled by `toggle_blink`
    blink_on: bool,
    driver: LedPixelEsp32Rmt<'static, RGB8, LedPixelColorGrb24>,
    brightness: u8,
}

pub const INITIAL_BRIGHTNESS: u8 = 20;
// Blinking LEDs stay visible even in night mode
const BLINK_MIN_BRIGHTNESS: u8 = 5;

impl Leds {
    pub fn new<C: RmtChannel + 'static>(channel: C, pin: impl OutputPin + 'static) -> Self {
        let driver = LedPixelEsp32Rmt::<RGB8, LedPixelColorGrb24>::new(channel, pin).unwrap();
        Self {
            driver,
            colors: [Color::default(); 3],
            blink: [false; 3],
            blink_on: true,
            brightness: INITIAL_BRIGHTNESS,
        }
    }

    pub fn flush(&mut self) -> Result<(), ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriverError> {
        let blink_on = self.blink_on;
        let colors = self.colors.iter().zip(self.blink).map(|(color, blink)| {
            if !blink {
                *color
            } else if blink_on {
                let brightness = color.brightness.unwrap_or(255).max(BLINK_MIN_BRIGHTNESS);
                color.brightness(brightness)
            } else {
                Color::default() // off
            }
        });
        self.driver.write(colors)
    }

    /// Switches the blink phase, called periodically from a timer
    pub fn toggle_blink(&mut self) {
        self.blink_on = !self.blink_on;
        if self.blink.contains(&true) {
            self.flush().unwrap();
        }
    }

    pub fn set_brightness(&mut self, brightness: u8) -> &mut Leds {
        self.brightness = brightness;
        self.colors.iter_mut().for_each(|color| {
            *color = color.brightness(brightness);
        });
        self
    }

    pub fn set_color(&mut self, position: LedPosition, color: Color) -> &mut Leds {
        self.set_blinking_color(position, color, false)
    }

    pub fn set_blinking_color(
        &mut self,
        position: LedPosition,
        color: Color,
        blink: bool,
    ) -> &mut Leds {
        self.colors[position as usize] = color.brightness(self.brightness);
        self.blink[position as usize] = blink;
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
        self.set_color(LedPosition::Top, WHITE)
            .set_color(LedPosition::Bottom, WHITE)
            .set_color(LedPosition::Center, WHITE)
            .flush()
            .unwrap();
    }

    /// Top LED shows CO2, bottom PM2.5 and center a mix of both.
    /// Problems are signalled by blinking, see README.
    pub fn visualize_measures(
        &mut self,
        (co2, co2_health): (u16, Health),
        (pm25, pm25_health): (u16, Health),
        system_ok: bool,
    ) {
        let co2_color = get_co2_color(co2);
        let pm25_color = get_pm25_color(pm25);

        // Center mixes only sensors with a usable value
        let usable = |color: Color, health| match health {
            Health::Ok | Health::Stale => Some(color),
            Health::Waiting | Health::Failed => None,
        };
        let center_color = match (
            usable(co2_color, co2_health),
            usable(pm25_color, pm25_health),
        ) {
            (Some(co2), Some(pm25)) => Some(co2.mix(&pm25)),
            (co2, pm25) => co2.or(pm25),
        };
        let center = match (co2_health, pm25_health, center_color) {
            (Health::Failed, Health::Failed, _) => (BLUE, true),
            (_, _, Some(color)) => (color, !system_ok),
            (_, _, None) => (WHITE, !system_ok),
        };

        self.set_led(LedPosition::Top, co2_color, co2_health)
            .set_blinking_color(LedPosition::Center, center.0, center.1)
            .set_led(LedPosition::Bottom, pm25_color, pm25_health)
            .flush()
            .unwrap();
    }

    fn set_led(&mut self, position: LedPosition, color: Color, health: Health) -> &mut Leds {
        match health {
            Health::Waiting => self.set_color(position, WHITE),
            Health::Ok => self.set_color(position, color),
            Health::Stale => self.set_blinking_color(position, color, true),
            Health::Failed => self.set_blinking_color(position, BLUE, true),
        }
    }
}
