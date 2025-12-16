extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use noli::net::{SocketAddr, TcpStream, lookup_host};
use saba_core::error::Error;
use saba_core::http::HttpResponse;

pub struct HttpClient {}

impl HttpClient {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get(&self, host: String, port: u16, path: String) -> Result<HttpResponse, Error> {
        let ips = lookup_host(&host)
            .map_err(|e| Error::Network(format!("Failed to find IP addresses: {:#?}", e)))?;

        if ips.len() < 1 {
            return Err(Error::Network("Failed to find IP addresses".to_string()));
        }

        let socket_addr: SocketAddr = (ips[0], port).into();

        let mut stream = TcpStream::connect(socket_addr)
            .map_err(|_| Err("Failed to connect to TCP stream".to_string()))?;

        let mut request = String::from("GET /");
        request.push_str(&path);
        request.push_str(" HTTP/1.1\n");

        // add headers
        request.push_str("Host: ");
        request.push_str(&host);
        request.push('\n');
        request.push_str("Accept: text/html\n");
        request.push_str("Connection: close\n");
        request.push('\n');

        let _bytes_written = stream
            .write(request.as_bytes())
            .map_err(|_| Error::Network("Failed to send a request to TCP stream".to_string()))?;

        let mut received = Vec::new();
        loop {
            let mut buf = [0u8; 4096];
            let bytes_read = stream.read(&mut buf).map_err(|_| {
                Error::Network("Failed to receive a request from TCP stream".to_string())
            })?;
            if bytes_read == 0 {
                break;
            }
            received.extend_from_slice(&buf[..bytes_read]);
        }

        let response = core::str::from_utf8(&received)
            .map_err(|e| Error::Network(format!("Invalid received response: {e}")))?;

        HttpResponse::new(response.to_string())
    }
}
