use esp_idf_hal::gpio::{Output, Pin, PinDriver};
use esp_idf_sys::EspError;

pub struct Fan<'a, PIN: Pin> {
    pin: PinDriver<'a, PIN, Output>,
}

impl<'a, PIN> Fan<'a, PIN>
where
    PIN: Pin,
{
    pub fn new(pin: PinDriver<'a, PIN, Output>) -> Self {
        Self { pin }
    }

    pub fn enable(&mut self) -> Result<(), EspError> {
        self.pin.set_high()
    }

    pub fn disable(&mut self) -> Result<(), EspError> {
        self.pin.set_low()
    }
}
