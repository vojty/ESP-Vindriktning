use std::{thread, time::Duration};

use crate::leds::Color;

const YELLOW: Color = Color {
    r: 255,
    g: 255,
    b: 0,
    brightness: None,
};
const ORANGE: Color = Color {
    r: 255,
    g: 69,
    b: 0,
    brightness: None,
};
const AQUA: Color = Color {
    r: 0,
    g: 255,
    b: 255,
    brightness: None,
};

const GREEN: Color = Color {
    r: 0,
    g: 255,
    b: 0,
    brightness: None,
};

const RED: Color = Color {
    r: 255,
    g: 0,
    b: 0,
    brightness: None,
};

const DARK_RED: Color = Color {
    r: 128,
    g: 0,
    b: 0,
    brightness: None,
};

pub fn sleep_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

pub fn get_co2_color(co2: u16) -> Color {
    // https://www.kane.co.uk/knowledge-centre/what-are-safe-levels-of-co-and-co2-in-rooms
    match co2 {
        0..=400 => AQUA,
        401..=1000 => GREEN,
        1001..=1500 => YELLOW,
        1501..=2000 => ORANGE,
        _ => RED,
    }
}

pub fn get_pm25_color(pm25: u16) -> Color {
    // https://aqicn.org/faq/2013-09-09/revised-pm25-aqi-breakpoints/
    // Good              0.0 - 12.0
    // Moderate         12.1 - 35.4
    // Unhealthy        35.5 - 55.4
    // Very Unhealthy   55.5 - 150.4
    // Hazardous       150.5
    match pm25 {
        0..=12 => GREEN,
        13..=35 => YELLOW,
        36..=55 => ORANGE,
        56..=150 => RED,
        _ => DARK_RED,
    }
}
