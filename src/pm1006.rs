use esp_idf_hal::{delay::BLOCK, uart::UartDriver};
use esp_idf_sys::EspError;

pub struct Pm1006<'a> {
    uart_driver: UartDriver<'a>,
    read_buffer: [u8; 20],
}

#[derive(Debug)]
pub enum Error {
    InvalidHeader(u8),
    InvalidCommandResponse(u8),
    InvalidChecksum(u8),
    Serial(EspError),
}

/*
    Command:
        1 byte:     0x11
        1 byte:     length N of command data
        N bytes:    command data
        1 byte:     check sum
*/
const REQUEST_HEADER: u8 = 0x11;
const RESPONSE_HEADER: u8 = 0x16;
const COMMAND_SEQUENCE: [u8; 5] = [REQUEST_HEADER, 0x02, 0x0b, 0x01, 0xe1];

fn parse_response(response: &[u8]) -> Result<u16, Error> {
    // Check header
    if response[0] != RESPONSE_HEADER {
        return Err(Error::InvalidHeader(response[0]));
    }
    /*
        Response:
            1 byte: 0x16
            1 byte: length N of response data
            N bytes: response data (CMD + Data frames)
            1 byte: check sum
    */

    // header + length
    let data_offset = 2;
    let mut checksum: u8 = response.iter().take(data_offset).sum();
    let length = response[1];

    // PM2.5 = DF3 * 256 + DF4 (indexed from 1)
    let mut result: u16 = 0;

    // Loop through data
    for i in 0..length {
        let index = data_offset + i as usize;
        let byte = response[index];
        match i {
            // CMD
            0 => {
                if byte != 0x0b {
                    return Err(Error::InvalidCommandResponse(byte));
                }
            }
            // DF1
            1 => (),

            // DF2
            2 => (),

            // DF3
            3 => result = byte as u16 * 256,

            // DF4
            4 => result += byte as u16,

            // DF5 - DF16 (not used)
            _ => (),
        }
        checksum += byte;
    }

    let expected_checksum = response[data_offset + length as usize];
    let diff = expected_checksum.wrapping_add(checksum);
    if diff != 0 {
        return Err(Error::InvalidChecksum(diff));
    }

    Ok(result)
}

impl<'a> Pm1006<'a> {
    pub fn new(uart_driver: UartDriver<'a>) -> Self {
        Self {
            uart_driver,
            read_buffer: [0; 20],
        }
    }

    pub fn read_pm25(&mut self) -> Result<u16, Error> {
        self.send_command()?;

        self.read_data()?;

        parse_response(&self.read_buffer)
    }

    // https://revspace.nl/VINDRIKTNING
    // https://threadreaderapp.com/thread/1415291684569632768.html
    fn send_command(&mut self) -> Result<usize, Error> {
        self.uart_driver
            .write(&COMMAND_SEQUENCE)
            .map_err(Error::Serial)
    }

    fn read_data(&mut self) -> Result<usize, Error> {
        self.uart_driver
            .read(&mut self.read_buffer, BLOCK)
            .map_err(Error::Serial)
    }
}

// TODO move to a separate workspace
#[cfg(test)]
mod tests {
    use super::parse_response;

    /// Test the get_serial_number function
    #[test]
    fn test_parse_response() {
        // Examples
        //           16   11 CMD   DF1  DF2  DF3  DF4
        //         0x16 0x11 0x0b 0x00 0x00 0x00 0x03 0x00 0x00 0x03 0x7e 0x00 0x00 0x00 0x19 0x02 0x00 0x00 0x16 0x19
        //         0x16 0x10 0x0B 0x00 0x00 0x00 0x22 0x00 0x00 0x03 0xA7 0x00 0x00 0x00 0x2D 0x02 0x00 0x00 0x0C // result = 34 (0x22)
        let response: [u8; 20] = [
            0x16, 0x10, 0x0B, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00, 0x03, 0xA7, 0x00, 0x00, 0x00,
            0x2D, 0x02, 0x00, 0x00, 0x0C,
        ];
        let result = parse_response(&response);
        assert!(result.is_ok());
        assert!(result.unwrap() == 34);
    }
}
