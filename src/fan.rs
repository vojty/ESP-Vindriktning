use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::sys::EspError;

pub struct Fan<'a> {
    pin: PinDriver<'a, Output>,
}

impl<'a> Fan<'a> {
    pub fn new(pin: PinDriver<'a, Output>) -> Self {
        Self { pin }
    }

    pub fn enable(&mut self) -> Result<(), EspError> {
        self.pin.set_high()
    }

    pub fn disable(&mut self) -> Result<(), EspError> {
        self.pin.set_low()
    }
}
