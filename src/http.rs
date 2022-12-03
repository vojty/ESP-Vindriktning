use embedded_svc::http::server::{Connection, HandlerError, Request};
use embedded_svc::io::{Read, Write};
use serde::Serialize;
use serde_json::Result;

pub trait SendJson {
    fn send_json<T>(self, json: &T) -> anyhow::Result<(), HandlerError>
    where
        T: ?Sized + Serialize;
}

impl<C> SendJson for Request<C>
where
    C: Connection,
{
    fn send_json<T>(self, json: &T) -> anyhow::Result<(), HandlerError>
    where
        T: ?Sized + Serialize,
    {
        let json = serde_json::to_string(json).unwrap();
        self.into_response(200, Some("OK"), &[("Content-Type", "application/json")])
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        Ok(())
    }
}

pub trait BodyParser {
    fn parse_body<T>(&mut self) -> Result<T>
    where
        T: serde::de::DeserializeOwned;
}

impl<C> BodyParser for Request<C>
where
    C: Connection,
{
    fn parse_body<T>(&mut self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // get content length
        let length = self
            .connection()
            .header("Content-Length")
            .unwrap()
            .parse::<usize>()
            .unwrap();

        // allocate buffer
        let mut buffer = vec![0; length];

        // read body
        self.read(&mut buffer).unwrap();

        // parse body
        serde_json::from_slice(&buffer)
    }
}
