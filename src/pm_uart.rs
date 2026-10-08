use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::io::EspIOError;
use esp_idf_svc::hal::uart::UartDriver;

// The sensor answers within a few ms, so anything longer means it is not responding
const READ_TIMEOUT_MS: u64 = 1_000;

/**
 * UartDriver wrapper for the PM1006 driver:
 * - reads time out instead of blocking forever (a timeout is reported as EOF)
 * - stale bytes in the RX buffer are dropped before a new command is sent
 */
pub struct PmUart {
    uart: UartDriver<'static>,
}

impl PmUart {
    pub fn new(uart: UartDriver<'static>) -> Self {
        Self { uart }
    }
}

impl embedded_io::ErrorType for PmUart {
    type Error = EspIOError;
}

impl embedded_io::Read for PmUart {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let timeout = TickType::new_millis(READ_TIMEOUT_MS).ticks();
        self.uart.read(buf, timeout).map_err(EspIOError)
    }
}

impl embedded_io::Write for PmUart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        // drop leftovers of a previous (partial) response
        self.uart.clear_rx().map_err(EspIOError)?;
        self.uart.write(buf).map_err(EspIOError)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        embedded_io::Write::flush(&mut self.uart)
    }
}
