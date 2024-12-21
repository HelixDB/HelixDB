use std::{collections::HashMap, io::Write};

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new() -> Response {
        let mut headers = HashMap::new();
        // TODO: Change to use router config for headers and default routes
        headers.insert("Content-Type".to_string(), "text/plain".to_string());

        Response {
            status: 200,
            headers,
            body: Vec::new(),
        }
    }

    /// Send response back via stream
    pub fn send<W: Write>(&mut self, stream: &mut W) -> std::io::Result<()> {
        let status_message = match self.status { 
            200 => "OK",
            404 => {
                self.body = b"404 - Route Not Found\n".to_vec();
                "Not Found"
            }
            500 => {
                self.body = b"500 - Internal Server Error\n".to_vec();
                "Internal Server Error"
            }
            _ => "Unknown"
        };

        let mut data_to_write = Vec::with_capacity(100);

        // write status 
        write!(data_to_write, "HTTP/1.1 {} {}\r\n", self.status, status_message)?;

        // write headers 
        self.headers.iter().for_each(|(header, value)| {
            write!(data_to_write, "{}: {}\r\n", header, value).unwrap();
        });

        write!(data_to_write, "Content-Length: {}\r\n", self.body.len())?;
        write!(data_to_write, "\r\n")?;

        // write body
        stream.write_all(&data_to_write)?;
        stream.write_all(&self.body)?;
        stream.flush()?;

        Ok(())
    }
}