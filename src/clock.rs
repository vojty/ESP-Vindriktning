use sntp_request::SntpRequest;

pub struct Clock {
    sntp: SntpRequest,
}

impl Clock {
    pub fn new() -> Self {
        let sntp = SntpRequest::new();
        Self { sntp }
    }

    pub fn get_timestamp(&self) -> Result<i64, std::io::Error> {
        self.sntp.get_unix_time()
    }
}
